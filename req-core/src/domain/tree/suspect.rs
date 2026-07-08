//! Fingerprint-based change detection and HRID drift repair.
//!
//! A link is *suspect* when the fingerprint recorded on the edge no longer
//! matches the parent's current fingerprint, meaning the parent has changed
//! since the link was created or last reviewed.

use tracing::instrument;
use uuid::Uuid;

use super::{AcceptLinkError, Tree};
use crate::domain::Hrid;

/// A suspect link in the requirement graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspectLink {
    /// The UUID of the child requirement.
    pub child_uuid: Uuid,
    /// The HRID of the child requirement.
    pub child_hrid: Hrid,
    /// The UUID of the parent requirement.
    pub parent_uuid: Uuid,
    /// The HRID of the parent requirement.
    pub parent_hrid: Hrid,
    /// The fingerprint stored in the child's parent reference.
    pub stored_fingerprint: String,
    /// The current fingerprint of the parent requirement.
    ///
    /// If empty, indicates the parent requirement is missing (failed to load or
    /// was deleted).
    pub current_fingerprint: String,
}

impl Tree {
    /// Check which requirements have stale parent HRIDs without modifying them.
    ///
    /// Returns an iterator of child UUIDs that have at least one parent link
    /// with an outdated HRID.
    #[instrument(skip(self))]
    pub fn check_hrid_drift(&self) -> impl Iterator<Item = Uuid> + '_ {
        use std::collections::HashSet;

        let mut drifted_uuids = HashSet::new();

        for child_uuid in self.graph.nodes() {
            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Get the current HRID of the parent
                let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                    continue;
                };

                // Check if the stored HRID is outdated
                if &edge_data.parent_hrid != current_parent_hrid {
                    drifted_uuids.insert(child_uuid);
                }
            }
        }

        drifted_uuids.into_iter()
    }

    /// Update parent HRIDs in all requirements.
    ///
    /// When requirements are renamed or moved, the stored parent HRIDs in child
    /// requirements can become stale. This method updates all outdated parent
    /// HRIDs to match their current values.
    ///
    /// Returns an iterator of child UUIDs that were updated.
    ///
    /// Edges whose parent is missing from the tree (e.g. failed to load) are
    /// skipped with a warning.
    #[instrument(skip(self))]
    pub fn update_hrids(&mut self) -> impl Iterator<Item = Uuid> + '_ {
        use std::collections::HashSet;

        let mut updated_uuids = HashSet::new();

        // Collect all edges that need updating (store edge identifiers, not cloned
        // HRIDs)
        let mut edges_to_update = Vec::new();

        for child_uuid in self.graph.nodes() {
            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Get the current HRID of the parent
                // Skip edges with missing parents (e.g., failed to load)
                let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                    tracing::warn!(
                        "Skipping edge with missing parent: child={child_uuid}, \
                         parent={parent_uuid}"
                    );
                    continue;
                };

                // Check if the stored HRID is outdated
                if &edge_data.parent_hrid != current_parent_hrid {
                    edges_to_update.push((child_uuid, parent_uuid));
                    updated_uuids.insert(child_uuid);
                }
            }
        }

        // Apply the updates to EdgeData only
        for (child_uuid, parent_uuid) in edges_to_update {
            // Look up the HRID again (HashMap lookup is O(1) and cheaper than cloning
            // earlier). Should always succeed since we already checked above.
            let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                // Parent disappeared between collection and update - skip
                tracing::warn!(
                    "Parent {parent_uuid} disappeared during HRID update for child {child_uuid}"
                );
                continue;
            };
            if let Some(edge_data) = self.graph.edge_weight_mut(child_uuid, parent_uuid) {
                // Update EdgeData (the sole source of truth)
                edge_data.parent_hrid = current_parent_hrid.clone();
            }
        }

        updated_uuids.into_iter()
    }

    /// Find all suspect links in the requirement graph.
    ///
    /// A link is suspect when the fingerprint stored in the edge data
    /// does not match the current fingerprint of the parent requirement.
    ///
    /// Graph nodes without a corresponding HRID (e.g. failed to load) are
    /// skipped.
    #[must_use]
    pub fn suspect_links(&self) -> Vec<SuspectLink> {
        use crate::domain::requirement::ContentRef;

        let mut suspect = Vec::new();

        for child_uuid in self.graph.nodes() {
            let Some(child_hrid) = self.hrids.get(&child_uuid) else {
                // Skip nodes that failed to register an HRID instead of panicking.
                continue;
            };

            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Access RequirementData directly to avoid full RequirementView construction
                let parent_data = self.requirements.get(&parent_uuid);

                // Calculate current fingerprint, or use empty string if parent is missing
                // Empty string indicates a dangling/broken reference (parent failed to load or
                // was deleted)
                let current_fingerprint = parent_data.map_or_else(String::new, |data| {
                    ContentRef {
                        title: &data.title,
                        body: &data.body,
                        tags: &data.tags,
                    }
                    .fingerprint()
                });

                // Report as suspect if fingerprints don't match, OR if parent is missing
                if edge_data.fingerprint != current_fingerprint {
                    suspect.push(SuspectLink {
                        child_uuid,
                        child_hrid: child_hrid.clone(),
                        parent_uuid,
                        parent_hrid: edge_data.parent_hrid.clone(),
                        stored_fingerprint: edge_data.fingerprint.clone(),
                        current_fingerprint,
                    });
                }
            }
        }

        suspect
    }

    /// Update the fingerprint for a specific parent link.
    ///
    /// Returns `Ok(true)` if the fingerprint was updated, `Ok(false)` if
    /// already up to date.
    ///
    /// # Errors
    ///
    /// Returns an error if the child or parent requirement is not found, or if
    /// the link doesn't exist.
    pub fn accept_suspect_link(
        &mut self,
        child_uuid: Uuid,
        parent_uuid: Uuid,
    ) -> Result<bool, AcceptLinkError> {
        use crate::domain::requirement::ContentRef;

        // Check if child exists in graph
        if !self.graph.contains_node(child_uuid) {
            return Err(AcceptLinkError::ChildNotFound(child_uuid));
        }

        // Compute the parent's current fingerprint directly from its stored
        // data; building a full RequirementView here would clone every parent
        // and child link just to hash the content.
        let current_fingerprint = self
            .requirements
            .get(&parent_uuid)
            .map(|data| {
                ContentRef {
                    title: &data.title,
                    body: &data.body,
                    tags: &data.tags,
                }
                .fingerprint()
            })
            .ok_or(AcceptLinkError::ParentNotFound(parent_uuid))?;

        // Find and update the edge
        if let Some(edge_data) = self.graph.edge_weight_mut(child_uuid, parent_uuid) {
            if edge_data.fingerprint == current_fingerprint {
                return Ok(false); // Already up to date
            }

            // Update EdgeData (the sole source of truth)
            edge_data.fingerprint.clone_from(&current_fingerprint);

            Ok(true)
        } else {
            Err(AcceptLinkError::LinkNotFound {
                child: child_uuid,
                parent: parent_uuid,
            })
        }
    }

    /// Update all suspect fingerprints in the tree.
    ///
    /// Skips links where the parent is missing (logs a warning instead of
    /// failing).
    pub fn accept_all_suspect_links(&mut self) -> Vec<(Uuid, Uuid)> {
        let suspect = self.suspect_links();
        let mut updated = Vec::new();

        for link in suspect {
            match self.accept_suspect_link(link.child_uuid, link.parent_uuid) {
                Ok(true) => {
                    updated.push((link.child_uuid, link.parent_uuid));
                }
                Ok(false) => {
                    // Already up to date, skip
                }
                Err(AcceptLinkError::ParentNotFound(parent_uuid)) => {
                    // Parent missing - log but don't fail the entire operation
                    tracing::warn!(
                        "Cannot accept suspect link: parent {parent_uuid} not found (child: {})",
                        link.child_uuid
                    );
                }
                Err(e) => {
                    // Other errors are unexpected but also shouldn't stop the batch
                    tracing::error!("Failed to accept suspect link: {e}");
                }
            }
        }

        updated
    }
}
