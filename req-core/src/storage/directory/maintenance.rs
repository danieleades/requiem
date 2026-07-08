//! Drift detection, suspect-link review, and repair.

use std::path::PathBuf;

use super::Directory;
use crate::domain::{requirement::LoadError, Hrid};

/// Result of accepting a suspect link.
#[derive(Debug)]
pub enum AcceptResult {
    /// The fingerprint was updated.
    Updated,
    /// The fingerprint was already up to date.
    AlreadyUpToDate,
}

/// Error type for accepting suspect links.
#[derive(Debug)]
pub enum AcceptSuspectLinkError {
    /// The child requirement was not found.
    ChildNotFound(Hrid),
    /// The parent requirement was not found.
    ParentNotFound(LoadError),
    /// The link between child and parent was not found.
    LinkNotFound {
        /// The child requirement HRID.
        child: Hrid,
        /// The parent requirement HRID.
        parent: Hrid,
    },
}

impl std::fmt::Display for AcceptSuspectLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChildNotFound(hrid) => {
                write!(f, "child requirement {} not found", hrid.display(3))
            }
            Self::ParentNotFound(e) => {
                write!(f, "parent requirement not found: {e}")
            }
            Self::LinkNotFound { child, parent } => {
                write!(
                    f,
                    "link from {} to {} not found",
                    child.display(3),
                    parent.display(3)
                )
            }
        }
    }
}

impl std::error::Error for AcceptSuspectLinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParentNotFound(e) => Some(e),
            _ => None,
        }
    }
}

impl Directory {
    /// Check which requirements have stale parent HRIDs without modifying them.
    ///
    /// Returns a list of HRIDs for requirements that would be updated by
    /// [`Self::update_hrids`].
    #[must_use]
    pub fn check_hrid_drift(&self) -> Vec<Hrid> {
        self.tree
            .check_hrid_drift()
            .filter_map(|uuid| self.tree.hrid(uuid))
            .cloned()
            .collect()
    }

    /// Check which requirements are in non-canonical locations.
    ///
    /// Returns a list of (HRID, `current_path`, `canonical_path`) tuples for
    /// requirements that are not stored at their canonical location.
    #[must_use]
    pub fn check_path_drift(&self) -> Vec<(Hrid, PathBuf, PathBuf)> {
        let mut misplaced = Vec::new();

        for req in self.tree.iter() {
            let canonical = self.canonical_path_for(req.hrid);

            if let Some(current) = self.paths.get(req.uuid) {
                // Simple comparison - if paths differ, it's misplaced
                // We compare the actual paths, not canonicalized, because we want to detect
                // when a file is not at its canonical location
                if current != &canonical {
                    misplaced.push((req.hrid.clone(), current.clone(), canonical));
                }
            }
        }

        misplaced
    }

    /// Move requirements to their canonical locations.
    ///
    /// # Errors
    ///
    /// Returns an error if any files cannot be moved.
    pub fn sync_paths(&mut self) -> anyhow::Result<Vec<(Hrid, PathBuf, PathBuf)>> {
        let misplaced = self.check_path_drift();

        if misplaced.is_empty() {
            return Ok(Vec::new());
        }

        let mut moved = Vec::new();

        for (hrid, current_path, canonical_path) in misplaced {
            // Create parent directory if it doesn't exist
            if let Some(parent) = canonical_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Move the file
            std::fs::rename(&current_path, &canonical_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to move {} from {} to {}: {}",
                    hrid.display(self.config.digits()),
                    current_path.display(),
                    canonical_path.display(),
                    e
                )
            })?;

            // Update the paths map
            if let Some(uuid) = self.tree.find_by_hrid(&hrid).map(|v| *v.uuid) {
                self.paths.insert(uuid, canonical_path.clone());
            }

            moved.push((hrid, current_path, canonical_path));
        }

        Ok(moved)
    }

    /// Update the human-readable IDs (HRIDs) of all 'parents' references in the
    /// requirements.
    ///
    /// These can become out of sync if requirement files are renamed.
    ///
    /// # Errors
    ///
    /// This method returns an error if some of the requirements cannot be saved
    /// to disk. This method does *not* fail fast. That is, it will attempt
    /// to save all the requirements before returning the error.
    pub fn update_hrids(&mut self) -> Vec<Hrid> {
        let updated: Vec<_> = self.tree.update_hrids().collect();

        for &uuid in &updated {
            self.mark_dirty(uuid);
        }

        // Directly access HRIDs from the tree instead of constructing full
        // RequirementViews
        updated
            .into_iter()
            .filter_map(|uuid| self.tree.hrid(uuid))
            .cloned()
            .collect()
    }

    /// Find all suspect links in the requirement graph.
    ///
    /// A link is suspect when the fingerprint stored in a child requirement
    /// does not match the current fingerprint of the parent requirement.
    #[must_use]
    pub fn suspect_links(&self) -> Vec<crate::domain::SuspectLink> {
        self.tree.suspect_links()
    }

    /// Detect all cycles in the requirement graph.
    ///
    /// Returns a list of cycles, where each cycle is represented as a path of
    /// HRIDs.
    #[must_use]
    pub fn detect_cycles(&self) -> Vec<Vec<Hrid>> {
        self.tree.detect_cycles()
    }

    /// Accept a specific suspect link by updating its fingerprint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The child or parent requirement doesn't exist
    /// - The parent link doesn't exist in the child
    /// - The requirement file cannot be saved
    pub fn accept_suspect_link(
        &mut self,
        child: Hrid,
        parent: Hrid,
    ) -> Result<AcceptResult, AcceptSuspectLinkError> {
        let (child_uuid, child_hrid) = match self.tree.find_by_hrid(&child) {
            Some(view) => (*view.uuid, view.hrid.clone()),
            None => return Err(AcceptSuspectLinkError::ChildNotFound(child)),
        };

        let (parent_uuid, parent_hrid) = match self.tree.find_by_hrid(&parent) {
            Some(view) => (*view.uuid, view.hrid.clone()),
            None => return Err(AcceptSuspectLinkError::ParentNotFound(LoadError::NotFound)),
        };

        let has_link = self
            .tree
            .parents(child_uuid)
            .into_iter()
            .any(|(uuid, _)| uuid == parent_uuid);

        if !has_link {
            return Err(AcceptSuspectLinkError::LinkNotFound { child, parent });
        }

        let was_updated = self
            .tree
            .accept_suspect_link(child_uuid, parent_uuid)
            .map_err(|e| match e {
                crate::domain::AcceptLinkError::ParentNotFound(_) => {
                    AcceptSuspectLinkError::ParentNotFound(LoadError::NotFound)
                }
                crate::domain::AcceptLinkError::ChildNotFound(uuid) => {
                    // Shouldn't happen since we validated above, but handle it
                    tracing::error!("Child {uuid} disappeared during accept operation");
                    AcceptSuspectLinkError::ChildNotFound(child.clone())
                }
                crate::domain::AcceptLinkError::LinkNotFound {
                    child: _,
                    parent: _,
                } => AcceptSuspectLinkError::LinkNotFound {
                    child: child.clone(),
                    parent: parent.clone(),
                },
            })?;

        if !was_updated {
            return Ok(AcceptResult::AlreadyUpToDate);
        }

        self.mark_dirty(child_uuid);
        let digits = self.config.digits();
        tracing::info!(
            "Accepted suspect link {} ← {}",
            child_hrid.display(digits),
            parent_hrid.display(digits)
        );

        Ok(AcceptResult::Updated)
    }

    /// Accept all suspect links by updating all fingerprints.
    ///
    /// # Errors
    ///
    /// Returns an error if any requirement file cannot be saved.
    /// This method does not fail fast - it will attempt to save all
    /// requirements before returning the error.
    pub fn accept_all_suspect_links(&mut self) -> Vec<(Hrid, Hrid)> {
        let updated = self.tree.accept_all_suspect_links();

        let mut collected = Vec::new();
        for &(child_uuid, parent_uuid) in &updated {
            if let (Some(child), Some(parent)) = (
                self.tree.requirement(child_uuid),
                self.tree.requirement(parent_uuid),
            ) {
                collected.push((child_uuid, child.hrid.clone(), parent.hrid.clone()));
            }
        }

        for (child_uuid, _, _) in &collected {
            self.mark_dirty(*child_uuid);
        }

        collected
            .into_iter()
            .map(|(_, child_hrid, parent_hrid)| (child_hrid, parent_hrid))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{super::setup_temp_directory, *};
    use crate::{domain::requirement::Parent, Requirement};

    #[test]
    fn update_hrids_corrects_outdated_parent_hrids() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir.add_requirement("P", String::new()).unwrap();
        let mut child = dir.add_requirement("C", String::new()).unwrap();

        dir.flush().expect("flush should succeed");

        // Manually corrupt HRID in child's parent info
        child.add_parent(
            parent.uuid(),
            Parent {
                hrid: Hrid::try_from("WRONG-999").unwrap(),
                fingerprint: parent.fingerprint(),
            },
        );
        child.save(&dir.root, &dir.config).unwrap();

        let mut loaded_dir = Directory::new(dir.root.clone()).unwrap();
        loaded_dir.update_hrids();
        loaded_dir.flush().unwrap();

        let updated = Requirement::load(&loaded_dir.root, child.hrid(), &loaded_dir.config)
            .expect("should load updated child");
        let (_, parent_ref) = updated.parents().next().unwrap();

        assert_eq!(&parent_ref.hrid, parent.hrid());
    }
}
