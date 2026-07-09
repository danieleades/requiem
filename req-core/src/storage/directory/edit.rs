//! Adding, updating, linking, renaming, moving, and deleting requirements.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use super::Directory;
use crate::{
    domain::{
        hrid::{KindString, NamespaceSegment},
        Hrid, LinkRequirementError, RequirementView,
    },
    storage::markdown::trim_empty_lines,
    Requirement,
};

/// Error type for adding requirements.
#[derive(Debug, thiserror::Error)]
pub enum AddRequirementError {
    /// The requirement kind or ID was invalid.
    #[error("failed to add requirement: {0}")]
    Hrid(#[from] crate::domain::HridError),

    /// A requirement with this UUID or HRID already exists.
    #[error("failed to add requirement: {0}")]
    Duplicate(#[from] crate::domain::TreeInsertError),

    /// The requirement kind is not in the allowed kinds list.
    #[error("kind '{kind}' is not allowed (allowed kinds: {allowed_kinds})")]
    DisallowedKind {
        /// The kind that was rejected.
        kind: String,
        /// The list of allowed kinds.
        allowed_kinds: String,
    },
}

impl Directory {
    /// Add a new requirement to the directory.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - the provided `kind` is an empty string or invalid
    /// - the requirement file cannot be written to
    ///
    /// # Panics
    ///
    /// Panics if the tree returns an invalid ID (should never happen).
    pub fn add_requirement(
        &mut self,
        kind: &str,
        content: String,
    ) -> Result<Requirement, AddRequirementError> {
        self.add_requirement_with_namespace(Vec::new(), kind, content)
    }

    /// Add a new requirement to the directory with an optional namespace.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - the provided `kind` or `namespace` segments are empty strings or
    ///   invalid
    /// - the requirement file cannot be written to
    ///
    /// # Panics
    ///
    /// Panics if the tree returns an invalid ID (should never happen).
    pub fn add_requirement_with_namespace(
        &mut self,
        namespace: Vec<String>,
        kind: &str,
        content: String,
    ) -> Result<Requirement, AddRequirementError> {
        let tree = &mut self.tree;

        // Validate kind (CLI already normalized to uppercase)
        let kind_string =
            KindString::new(kind.to_string()).map_err(crate::domain::hrid::Error::from)?;

        // Check if kind is allowed by configuration
        if !self.config.is_kind_allowed(kind) {
            let allowed_kinds = if self.config.allowed_kinds().is_empty() {
                "none configured (all allowed)".to_string()
            } else {
                self.config.allowed_kinds().join(", ")
            };
            return Err(AddRequirementError::DisallowedKind {
                kind: kind.to_string(),
                allowed_kinds,
            });
        }

        // Validate namespace segments (allow lowercase/mixed-case)
        let namespace_strings: Result<Vec<_>, _> = namespace
            .into_iter()
            .map(|seg| {
                NamespaceSegment::new(seg)
                    .map_err(|e| crate::domain::hrid::Error::Namespace(String::new(), e))
            })
            .collect();
        let namespace_strings = namespace_strings?;

        let id = tree.next_index(&namespace_strings, &kind_string);
        let hrid = Hrid::new_with_namespace(namespace_strings, kind_string, id);

        // Parse content to extract title and body
        // If no content is provided via CLI, check for a template
        let (title, body) = if content.is_empty() {
            // Template content - treat as raw body, don't parse
            let template_content = load_template(&self.root, &hrid);
            (String::new(), template_content)
        } else {
            split_title_and_body(content)
        };

        let requirement = Requirement::new(hrid, title, body);

        tree.insert(requirement.clone())?;
        let canonical = self.canonical_path_for(requirement.hrid());
        self.paths.insert(requirement.uuid(), canonical);
        self.mark_dirty(requirement.uuid());

        tracing::info!(
            "Added requirement: {}",
            requirement.hrid().display(self.config.digits())
        );

        Ok(requirement)
    }

    /// Update the title, body, and/or tags of an existing requirement.
    ///
    /// Fields passed as `None` are left unchanged. Changing the body or tags
    /// changes the requirement's fingerprint, so links from its children
    /// become suspect and need review.
    ///
    /// Returns `true` if any field actually changed.
    ///
    /// # Errors
    ///
    /// Returns an error if the requirement does not exist.
    pub fn update_requirement(
        &mut self,
        hrid: &Hrid,
        title: Option<String>,
        body: Option<String>,
        tags: Option<BTreeSet<String>>,
    ) -> anyhow::Result<bool> {
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            anyhow::bail!(
                "Requirement {} not found",
                hrid.display(self.config.digits())
            );
        };
        let uuid = *view.uuid;

        let Some(changed) = self
            .tree
            .update_requirement_content(uuid, title, body, tags)
        else {
            anyhow::bail!(
                "Requirement {} not found",
                hrid.display(self.config.digits())
            );
        };

        if changed {
            self.mark_dirty(uuid);
        }

        Ok(changed)
    }

    /// Link two requirements together with a parent-child relationship.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - either the child or parent requirement file cannot be found
    /// - either the child or parent requirement file cannot be parsed
    /// - the child requirement file cannot be written to
    /// - the parent/child UUIDs cannot be linked
    pub fn link_requirement(
        &mut self,
        child: &Hrid,
        parent: &Hrid,
    ) -> Result<RequirementView<'_>, LinkRequirementError> {
        let outcome = self.tree.link_requirement(child, parent)?;
        self.mark_dirty(outcome.child_uuid);

        if !outcome.already_linked {
            let digits = self.config.digits();
            tracing::info!(
                "Linked {} ← {}",
                outcome.child_hrid.display(digits),
                outcome.parent_hrid.display(digits)
            );
        }

        self.tree
            .requirement(outcome.child_uuid)
            .ok_or(LinkRequirementError::ChildNotFound(outcome.child_hrid))
    }

    /// Unlink two requirements, removing the parent-child relationship.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - either the child or parent requirement does not exist
    /// - the link between child and parent does not exist
    pub fn unlink_requirement(&mut self, child: &Hrid, parent: &Hrid) -> anyhow::Result<()> {
        let child_uuid = self.tree.unlink_requirement(child, parent)?;
        self.mark_dirty(child_uuid);

        let digits = self.config.digits();
        tracing::info!(
            "Unlinked {} from parent {}",
            child.display(digits),
            parent.display(digits)
        );

        Ok(())
    }

    /// Delete a requirement from the directory.
    ///
    /// This removes the requirement from the tree and marks it for deletion
    /// from disk. The requirement must not have any children, or this will
    /// fail.
    ///
    /// # Errors
    ///
    /// Returns an error if the requirement has children.
    pub fn delete_requirement(&mut self, hrid: &Hrid) -> anyhow::Result<()> {
        // Find the requirement
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            anyhow::bail!(
                "Requirement {} not found",
                hrid.display(self.config.digits())
            );
        };

        // Check if it has children
        if !view.children.is_empty() {
            anyhow::bail!(
                "Cannot delete {}: requirement has {} children",
                hrid.display(self.config.digits()),
                view.children.len()
            );
        }

        let uuid = *view.uuid;

        // Remove from tree
        self.tree.remove_requirement(uuid)?;

        // Mark file for deletion
        if let Some(path) = self.paths.remove(&uuid) {
            self.deletions.insert(path);
        }

        Ok(())
    }

    /// Delete a requirement and unlink it from all children.
    ///
    /// This removes the requirement from the tree and marks it for deletion
    /// from disk. All children will have this requirement removed from
    /// their parent list.
    ///
    /// # Errors
    ///
    /// Returns an error if the requirement doesn't exist.
    pub fn delete_and_orphan(&mut self, hrid: &Hrid) -> anyhow::Result<()> {
        // Find the requirement
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            anyhow::bail!(
                "Requirement {} not found",
                hrid.display(self.config.digits())
            );
        };

        let uuid = *view.uuid;

        // Collect children UUIDs before removing
        let children = view.children;

        // Remove from tree (this also removes edges)
        self.tree.remove_requirement(uuid)?;

        // Mark children as dirty since their parent lists changed
        for child_uuid in children {
            self.mark_dirty(child_uuid);
        }

        // Mark file for deletion
        if let Some(path) = self.paths.remove(&uuid) {
            self.deletions.insert(path);
        }

        Ok(())
    }

    /// Find all descendants that would become orphans if this requirement were
    /// deleted.
    ///
    /// Returns a list of HRIDs for requirements that would have no parents if
    /// the given requirement (and its orphaned descendants) were deleted.
    /// This implements smart cascade deletion logic.
    #[must_use]
    pub fn find_orphaned_descendants(&self, hrid: &Hrid) -> Vec<Hrid> {
        use std::collections::{HashSet, VecDeque};

        let Some(view) = self.tree.find_by_hrid(hrid) else {
            return vec![];
        };

        let mut to_delete = HashSet::new();
        to_delete.insert(*view.uuid);

        let mut queue = VecDeque::new();
        queue.push_back(*view.uuid);

        // BFS to find all descendants that would be orphaned
        while let Some(current_uuid) = queue.pop_front() {
            let Some(current_view) = self.tree.requirement(current_uuid) else {
                continue;
            };

            for &child_uuid in &current_view.children {
                // Skip if we're already planning to delete this child
                if to_delete.contains(&child_uuid) {
                    continue;
                }

                // Count how many parents this child has that we're NOT deleting
                let Some(child_view) = self.tree.requirement(child_uuid) else {
                    continue;
                };

                let remaining_parents = child_view
                    .parents
                    .iter()
                    .filter(|p| !to_delete.contains(&p.0))
                    .count();

                // If this child would have no parents left, it's orphaned
                if remaining_parents == 0 {
                    to_delete.insert(child_uuid);
                    queue.push_back(child_uuid);
                }
            }
        }

        // Convert to HRIDs and return in deterministic order
        let mut result: Vec<_> = to_delete
            .into_iter()
            .filter_map(|uuid| self.tree.hrid(uuid).cloned())
            .collect();
        result.sort();
        result
    }

    /// Renames a requirement by changing its HRID.
    ///
    /// This method:
    /// - Updates the HRID in the tree
    /// - Updates the file path mapping
    /// - Marks the renamed requirement and all its children as dirty
    ///
    /// The actual file renaming and content updates happen during `flush()`.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - The old HRID doesn't exist
    /// - The new HRID already exists
    /// - The new HRID kind is not allowed by configuration
    pub fn rename_requirement(
        &mut self,
        old_hrid: &Hrid,
        new_hrid: &Hrid,
    ) -> anyhow::Result<Vec<Hrid>> {
        // Check kind is allowed
        if !self.config.is_kind_allowed(new_hrid.kind()) {
            anyhow::bail!("Kind '{}' is not allowed by configuration", new_hrid.kind());
        }

        // Perform rename in tree (this updates all parent references)
        let (uuid, children_uuids) = self.tree.rename_requirement(old_hrid, new_hrid)?;

        // Update file path mapping. The new path is registered even when no
        // old path was recorded (e.g. a requirement added and renamed before
        // ever being flushed) so the old file is always reconciled on flush.
        if let Some(old_path) = self.paths.remove(&uuid) {
            self.deletions.insert(old_path);
        }
        let new_path = self.canonical_path_for(new_hrid);
        self.paths.insert(uuid, new_path);

        // Mark the renamed requirement as dirty
        self.dirty.insert(uuid);

        // Mark all children as dirty (their parent HRID changed in frontmatter)
        for child_uuid in &children_uuids {
            self.dirty.insert(*child_uuid);
        }

        // Collect children HRIDs for reporting
        let children_hrids: Vec<Hrid> = children_uuids
            .iter()
            .filter_map(|uuid| self.tree.hrid(*uuid).cloned())
            .collect();

        Ok(children_hrids)
    }

    /// Moves a requirement to a new file path.
    ///
    /// This method:
    /// - Moves the file to the new location
    /// - Extracts the HRID from the new path
    /// - If the HRID changed, updates it (like rename)
    /// - Marks the requirement and optionally its children as dirty
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - The HRID doesn't exist
    /// - The new path would create an HRID conflict
    /// - The new HRID kind is not allowed
    pub fn move_requirement(
        &mut self,
        hrid: &Hrid,
        new_path: PathBuf,
    ) -> anyhow::Result<Option<Vec<Hrid>>> {
        use crate::storage::path_parser::hrid_from_path;

        // Find the requirement
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            anyhow::bail!(
                "Requirement {} not found",
                hrid.display(self.config.digits())
            );
        };
        let uuid = *view.uuid;

        // Extract HRID from new path
        let new_hrid = hrid_from_path(&new_path, &self.root, &self.config)
            .map_err(|e| anyhow::anyhow!("Failed to parse HRID from path: {e}"))?;

        // Check if HRID changed
        let children_updated = if &new_hrid == hrid {
            None
        } else {
            // HRID changed - perform rename
            let (_, children_uuids) = self.tree.rename_requirement(hrid, &new_hrid)?;

            // Collect children HRIDs
            let children_hrids: Vec<Hrid> = children_uuids
                .iter()
                .filter_map(|uuid| self.tree.hrid(*uuid).cloned())
                .collect();

            // Mark children as dirty
            for child_uuid in &children_uuids {
                self.dirty.insert(*child_uuid);
            }

            Some(children_hrids)
        };

        // Update file path mapping
        if let Some(old_path) = self.paths.remove(&uuid) {
            // Mark old file for deletion
            self.deletions.insert(old_path);
        }

        // Set new path
        self.paths.insert(uuid, new_path);

        // Mark the requirement as dirty
        self.dirty.insert(uuid);

        Ok(children_updated)
    }
}

/// Split user-provided content into a title (from a leading `#` heading, if
/// any) and a body.
fn split_title_and_body(content: String) -> (String, String) {
    if let Some(first_line_end) = content.find('\n') {
        let first_line = &content[..first_line_end];
        if first_line.trim_start().starts_with('#') {
            // Has a heading - extract title and body
            let after_hashes = first_line.trim_start_matches('#').trim();
            let title = after_hashes.to_string();
            // Skip newline after heading but preserve indentation in body
            let body = content[first_line_end + 1..].to_string();
            // Trim only empty lines from start/end, preserve indentation
            let body = trim_empty_lines(&body);
            (title, body)
        } else {
            // No heading
            (String::new(), content)
        }
    } else {
        // Single line - check if it's a heading
        let trimmed = content.trim();
        if trimmed.starts_with('#') {
            let after_hashes = trimmed.trim_start_matches('#').trim();
            (after_hashes.to_string(), String::new())
        } else {
            (String::new(), content)
        }
    }
}

/// Load a template for the given HRID from the `.req/templates/` directory.
///
/// This checks for templates in order of specificity:
/// 1. Full HRID prefix with namespace (e.g., `.req/templates/AUTH-USR.md`)
/// 2. KIND only (e.g., `.req/templates/USR.md`)
///
/// Returns an empty string if no template is found.
fn load_template(root: &Path, hrid: &Hrid) -> String {
    let templates_dir = root.join(".req").join("templates");

    // Try full prefix first (e.g., "AUTH-USR.md")
    let full_prefix = hrid.prefix();
    let full_path = templates_dir.join(format!("{full_prefix}.md"));

    if full_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            tracing::debug!("Loaded template from {}", full_path.display());
            return content;
        }
    }

    // Fall back to KIND only (e.g., "USR.md")
    let kind = hrid.kind();
    let kind_path = templates_dir.join(format!("{kind}.md"));

    if kind_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&kind_path) {
            tracing::debug!("Loaded template from {}", kind_path.display());
            return content;
        }
    }

    tracing::debug!(
        "No template found for HRID {}, checked {} and {}",
        hrid.display(3),
        full_path.display(),
        kind_path.display()
    );
    String::new()
}

#[cfg(test)]
mod tests {
    use super::{super::setup_temp_directory, *};
    use crate::storage::directory::load::load_config;

    #[test]
    fn can_add_requirement() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir.add_requirement("REQ", String::new()).unwrap();

        dir.flush().expect("flush should succeed");

        assert_eq!(r1.hrid().display(3).to_string(), "REQ-001");

        let loaded = Requirement::load(&dir.root, r1.hrid(), &dir.config)
            .expect("should load saved requirement");
        assert_eq!(loaded.uuid(), r1.uuid());
    }

    #[test]
    fn can_add_multiple_requirements_with_incrementing_id() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir.add_requirement("REQ", String::new()).unwrap();
        let r2 = dir.add_requirement("REQ", String::new()).unwrap();

        dir.flush().expect("flush should succeed");

        assert_eq!(r1.hrid().display(3).to_string(), "REQ-001");
        assert_eq!(r2.hrid().display(3).to_string(), "REQ-002");
    }

    #[test]
    fn can_link_two_requirements() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir.add_requirement("SYS", String::new()).unwrap();
        let child = dir.add_requirement("USR", String::new()).unwrap();
        dir.flush().expect("flush should succeed");

        let mut reloaded = Directory::new(dir.root.clone()).unwrap();
        reloaded
            .link_requirement(child.hrid(), parent.hrid())
            .unwrap();
        reloaded.flush().unwrap();

        let config = load_config(&dir.root).unwrap();
        let updated =
            Requirement::load(&dir.root, child.hrid(), &config).expect("should load child");

        let parents: Vec<_> = updated.parents().collect();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].0, parent.uuid());
        assert_eq!(&parents[0].1.hrid, parent.hrid());
    }

    #[test]
    fn add_requirement_rejects_disallowed_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create config with allowed_kinds
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nallowed_kinds = [\"USR\", \"SYS\"]\n",
        )
        .unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Try to add a requirement with a disallowed kind
        let result = dir.add_requirement("REQ", "# Test".to_string());

        // Should fail with DisallowedKind error
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            AddRequirementError::DisallowedKind { kind, .. } => {
                assert_eq!(kind, "REQ");
            }
            _ => panic!("Expected DisallowedKind error, got: {error:?}"),
        }
    }

    #[test]
    fn add_requirement_allows_configured_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create config with allowed_kinds
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nallowed_kinds = [\"USR\", \"SYS\"]\n",
        )
        .unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Try to add a requirement with an allowed kind
        let result = dir.add_requirement("USR", "# Test".to_string());

        // Should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn add_namespaced_requirements_increments_ids_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Add first namespaced requirement
        let req1 = dir
            .add_requirement_with_namespace(
                vec!["SYSTEM".to_string(), "AUTH".to_string()],
                "USR",
                "# First requirement".to_string(),
            )
            .unwrap();

        // Add second namespaced requirement with same namespace and kind
        let req2 = dir
            .add_requirement_with_namespace(
                vec!["SYSTEM".to_string(), "AUTH".to_string()],
                "USR",
                "# Second requirement".to_string(),
            )
            .unwrap();

        // Verify IDs are different and incrementing
        assert_eq!(req1.hrid().display(3).to_string(), "SYSTEM-AUTH-USR-001");
        assert_eq!(req2.hrid().display(3).to_string(), "SYSTEM-AUTH-USR-002");

        // Verify both can be flushed without error
        assert!(dir.flush().is_ok());
    }

    #[test]
    fn update_requirement_changes_content_and_persists() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir
            .add_requirement("REQ", "# Old title\n\nOld body".to_string())
            .unwrap();
        dir.flush().unwrap();

        let changed = dir
            .update_requirement(
                req.hrid(),
                Some("New title".to_string()),
                Some("New body".to_string()),
                Some(["tag1".to_string()].into()),
            )
            .unwrap();
        assert!(changed);
        dir.flush().unwrap();

        let config = load_config(&dir.root).unwrap();
        let updated = Requirement::load(&dir.root, req.hrid(), &config).unwrap();
        assert_eq!(updated.title(), "New title");
        assert_eq!(updated.body(), "New body");
        assert_eq!(updated.tags(), &BTreeSet::from(["tag1".to_string()]));
        // Identity is preserved across content updates.
        assert_eq!(updated.uuid(), req.uuid());
    }

    #[test]
    fn update_requirement_leaves_omitted_fields_unchanged() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir
            .add_requirement("REQ", "# Title\n\nBody".to_string())
            .unwrap();
        dir.flush().unwrap();

        let changed = dir
            .update_requirement(req.hrid(), Some("Renamed".to_string()), None, None)
            .unwrap();
        assert!(changed);
        dir.flush().unwrap();

        let config = load_config(&dir.root).unwrap();
        let updated = Requirement::load(&dir.root, req.hrid(), &config).unwrap();
        assert_eq!(updated.title(), "Renamed");
        assert_eq!(updated.body(), "Body");
    }

    #[test]
    fn update_requirement_reports_no_change() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir
            .add_requirement("REQ", "# Title\n\nBody".to_string())
            .unwrap();
        dir.flush().unwrap();

        let changed = dir
            .update_requirement(req.hrid(), Some("Title".to_string()), None, None)
            .unwrap();
        assert!(!changed);
    }

    #[test]
    fn update_requirement_body_marks_child_links_suspect() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir
            .add_requirement("SYS", "# Parent\n\nOriginal".to_string())
            .unwrap();
        let child = dir.add_requirement("USR", "# Child".to_string()).unwrap();
        dir.link_requirement(child.hrid(), parent.hrid()).unwrap();
        dir.flush().unwrap();

        let mut dir = Directory::new(dir.root.clone()).unwrap();
        assert!(dir.suspect_links().is_empty());

        dir.update_requirement(parent.hrid(), None, Some("Changed".to_string()), None)
            .unwrap();

        let suspects = dir.suspect_links();
        assert_eq!(suspects.len(), 1);
        assert_eq!(&suspects[0].child_hrid, child.hrid());
        assert_eq!(&suspects[0].parent_hrid, parent.hrid());
    }

    #[test]
    fn update_requirement_unknown_hrid_errors() {
        let (_tmp, mut dir) = setup_temp_directory();
        let missing: Hrid = "REQ-999".parse().unwrap();
        assert!(dir
            .update_requirement(&missing, Some("Title".to_string()), None, None)
            .is_err());
    }

    #[test]
    fn add_requirement_records_path() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir.add_requirement("REQ", String::new()).unwrap();

        let expected = dir.canonical_path_for(req.hrid());
        assert_eq!(dir.path_for(req.hrid()), Some(expected.as_path()));
    }
}
