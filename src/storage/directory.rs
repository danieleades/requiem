//! A filesystem backed store of requirements
//!
//! The [`Directory`] provides a way to manage requirements stored in a
//! directory structure. It is a wrapper around the filesystem agnostic
//! [`Tree`].

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt, io,
    path::{Path, PathBuf},
};

use nonempty::NonEmpty;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    domain::{hrid::KindString, requirement::LoadError, Config, Hrid, RequirementView, Tree},
    storage::markdown::trim_empty_lines,
    Requirement,
};

/// A filesystem backed store of requirements.
pub struct Directory {
    /// The root of the directory requirements are stored in.
    root: PathBuf,
    tree: Tree,
    config: Config,
    dirty: HashSet<Uuid>,
    /// Source paths for requirements that were loaded from disk.
    /// Used to save requirements back to their original location.
    paths: HashMap<Uuid, PathBuf>,
    /// Paths to delete on flush.
    deletions: HashSet<PathBuf>,
}

impl Directory {
    /// Mark a requirement as needing to be flushed to disk.
    fn mark_dirty(&mut self, uuid: Uuid) {
        self.dirty.insert(uuid);
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
    pub fn link_requirement(
        &mut self,
        child: &Hrid,
        parent: &Hrid,
    ) -> Result<RequirementView<'_>, LoadError> {
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
            .ok_or(LoadError::NotFound)
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

    /// Opens a directory at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if unrecognised files are found when
    /// `allow_unrecognised` is false in the configuration.
    pub fn new(root: PathBuf) -> Result<Self, DirectoryLoadError> {
        let config = load_config(&root);
        let md_paths = collect_markdown_paths(&root);

        let (requirements, unrecognised_paths): (Vec<_>, Vec<_>) = md_paths
            .par_iter()
            .map(|path| try_load_requirement(path, &root, &config))
            .partition(Result::is_ok);

        let requirements: Vec<(Requirement, PathBuf)> =
            requirements.into_iter().map(Result::unwrap).collect();
        let unrecognised_paths: Vec<_> = unrecognised_paths
            .into_iter()
            .map(Result::unwrap_err)
            .collect();

        if !config.allow_unrecognised && !unrecognised_paths.is_empty() {
            return Err(DirectoryLoadError::UnrecognisedFiles(unrecognised_paths));
        }

        // Check for disallowed kinds if allowed_kinds is configured
        if !config.allowed_kinds().is_empty() {
            let disallowed: Vec<(PathBuf, String)> = requirements
                .iter()
                .filter(|(req, _path)| !config.is_kind_allowed(req.hrid().kind()))
                .map(|(req, path)| (path.clone(), req.hrid().kind().to_string()))
                .collect();

            if !disallowed.is_empty() {
                return Err(DirectoryLoadError::DisallowedKinds {
                    files: disallowed,
                    allowed_kinds: config.allowed_kinds().to_vec(),
                });
            }
        }

        let mut tree = Tree::with_capacity(requirements.len());
        let mut paths = HashMap::with_capacity(requirements.len());
        for (req, path) in requirements {
            let uuid = req.uuid();
            tree.insert(req)
                .map_err(|error| DirectoryLoadError::Duplicate {
                    error,
                    path: path.clone(),
                })?;
            paths.insert(uuid, path);
        }

        // Note: No need to rebuild edges - DiGraphMap::add_edge() automatically
        // creates nodes if they don't exist, so edges are created correctly even
        // when children are inserted before their parents.

        Ok(Self {
            root,
            tree,
            config,
            dirty: HashSet::new(),
            paths,
            deletions: HashSet::new(),
        })
    }
}

/// Error type for directory loading operations.
#[derive(Debug, thiserror::Error)]
pub enum DirectoryLoadError {
    /// One or more files in the directory could not be recognized as valid
    /// requirements.
    UnrecognisedFiles(Vec<(PathBuf, LoadError)>),

    /// A requirement has a duplicate UUID or HRID.
    Duplicate {
        /// The underlying tree insertion error
        error: crate::domain::TreeInsertError,
        /// The path of the file being inserted when the error occurred
        path: PathBuf,
    },

    /// One or more requirements have kinds that are not in the allowed list.
    DisallowedKinds {
        /// The files with disallowed kinds
        files: Vec<(PathBuf, String)>,
        /// The list of allowed kinds
        allowed_kinds: Vec<String>,
    },
}

impl fmt::Display for DirectoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrecognisedFiles(files) => {
                write!(f, "Failed to load requirements:")?;
                for (i, (path, error)) in files.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\n  {} ({})", path.display(), error)?;
                }
                Ok(())
            }
            Self::Duplicate { error, path } => {
                write!(f, "Failed to load {}: {}", path.display(), error)
            }
            Self::DisallowedKinds {
                files,
                allowed_kinds,
            } => {
                write!(f, "Requirements with disallowed kinds found: ")?;
                for (i, (path, kind)) in files.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} (kind: {})", path.display(), kind)?;
                }
                write!(f, ". Allowed kinds: ")?;
                if allowed_kinds.is_empty() {
                    write!(f, "none configured")?;
                } else {
                    write!(f, "{}", allowed_kinds.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

fn load_config(root: &Path) -> Config {
    let path = root.join(".req/config.toml");
    Config::load(&path).unwrap_or_else(|e| {
        tracing::debug!("Failed to load config: {e}");
        Config::default()
    })
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

fn collect_markdown_paths(root: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            // Skip the .req directory (used for templates and other metadata)
            !entry.path().components().any(|c| c.as_os_str() == ".req")
        })
        .filter(|entry| entry.path().extension() == Some(OsStr::new("md")))
        .map(walkdir::DirEntry::into_path)
        .collect()
}

fn try_load_requirement(
    path: &Path,
    _root: &Path,
    config: &Config,
) -> Result<(Requirement, PathBuf), (PathBuf, LoadError)> {
    // Load the requirement from the file
    // The HRID is now read from the frontmatter, not parsed from the path
    match load_requirement_from_file(path, config) {
        Ok(req) => Ok((req, path.to_path_buf())),
        Err(e) => {
            tracing::debug!(
                "Failed to load requirement from {}: {:?}",
                path.display(),
                e
            );
            Err((path.to_path_buf(), e))
        }
    }
}

fn load_requirement_from_file(path: &Path, _config: &Config) -> Result<Requirement, LoadError> {
    // Load directly from the file path we found during directory scanning
    // The HRID is read from the frontmatter within the file
    use std::{fs::File, io::BufReader};

    use crate::storage::markdown::MarkdownRequirement;

    let file = File::open(path).map_err(|io_error| match io_error.kind() {
        std::io::ErrorKind::NotFound => LoadError::NotFound,
        _ => LoadError::Io(io_error),
    })?;

    let mut reader = BufReader::new(file);
    let md_req = MarkdownRequirement::read(&mut reader)?;
    Ok(md_req.try_into()?)
}

impl Directory {
    /// Returns the filesystem root backing this directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the canonical/expected path for a requirement based on its HRID.
    ///
    /// This constructs the ideal path where the requirement *should* be located
    /// according to the repository configuration. Use [`Self::path_for`] to get
    /// the actual path where the file was loaded from.
    #[must_use]
    pub fn canonical_path_for(&self, hrid: &Hrid) -> PathBuf {
        crate::storage::construct_path_from_hrid(
            &self.root,
            hrid,
            self.config.subfolders_are_namespaces,
            self.config.digits(),
        )
    }

    /// Returns the actual filesystem path where a requirement was loaded from.
    ///
    /// This returns the real path that was used to load the requirement,
    /// which may differ from the canonical path returned by
    /// [`Self::canonical_path_for`] if the file is misplaced.
    #[must_use]
    pub fn path_for(&self, hrid: &Hrid) -> Option<&Path> {
        self.tree
            .find_by_hrid(hrid)
            .and_then(|view| self.paths.get(view.uuid))
            .map(PathBuf::as_path)
    }

    /// Returns an iterator over all requirements stored in the directory.
    pub fn requirements(&'_ self) -> impl Iterator<Item = RequirementView<'_>> + '_ {
        self.tree.iter()
    }

    /// Returns the configuration used when loading this directory.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Retrieves a requirement by its human-readable identifier.
    #[must_use]
    pub fn requirement_by_hrid(&self, hrid: &Hrid) -> Option<Requirement> {
        self.tree
            .find_by_hrid(hrid)
            .map(|view| view.to_requirement())
    }

    /// Find a requirement by its HRID, returning a view.
    #[must_use]
    pub fn find_by_hrid(&self, hrid: &Hrid) -> Option<RequirementView<'_>> {
        self.tree.find_by_hrid(hrid)
    }

    /// Get the HRIDs of all children of a requirement.
    #[must_use]
    pub fn children_of(&self, hrid: &Hrid) -> Vec<Hrid> {
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            return vec![];
        };

        view.children
            .iter()
            .filter_map(|uuid| self.tree.hrid(*uuid).cloned())
            .collect()
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
            let Some(current_view) = self.tree.find_by_uuid(current_uuid) else {
                continue;
            };

            for &child_uuid in &current_view.children {
                // Skip if we're already planning to delete this child
                if to_delete.contains(&child_uuid) {
                    continue;
                }

                // Count how many parents this child has that we're NOT deleting
                let Some(child_view) = self.tree.find_by_uuid(child_uuid) else {
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

        // Update file path mapping
        if let Some(old_path) = self.paths.remove(&uuid) {
            // Mark old file for deletion
            self.deletions.insert(old_path);

            // Calculate new path
            let new_path = self.canonical_path_for(new_hrid);
            self.paths.insert(uuid, new_path);
        }

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

        // Validate namespace segments (CLI already normalized to uppercase)
        let namespace_strings: Result<Vec<_>, _> = namespace
            .into_iter()
            .map(|seg| KindString::new(seg).map_err(crate::domain::hrid::Error::from))
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
            // User-provided content - parse if it has a heading
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
        };

        let requirement = Requirement::new(hrid, title, body);

        tree.insert(requirement.clone())?;
        self.mark_dirty(requirement.uuid());

        tracing::info!(
            "Added requirement: {}",
            requirement.hrid().display(self.config.digits())
        );

        Ok(requirement)
    }

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

    /// Persist all dirty requirements to disk.
    ///
    /// Returns the HRIDs of the requirements that were written.
    ///
    /// # Errors
    ///
    /// Returns an error containing the paths that failed to flush alongside the
    /// underlying IO error.
    pub fn flush(&mut self) -> Result<Vec<Hrid>, FlushError> {
        use crate::storage::path_parser::construct_path_from_hrid;

        let dirty: Vec<_> = self.dirty.iter().copied().collect();
        let mut flushed = Vec::new();
        let mut failures = Vec::new();

        for uuid in dirty {
            let Some(requirement) = self.tree.get_requirement(uuid) else {
                // Requirement may have been removed; drop from dirty set.
                self.dirty.remove(&uuid);
                continue;
            };

            let hrid = requirement.hrid().clone();

            // Use the stored path if available, otherwise calculate a canonical path
            let path = self.paths.get(&uuid).map_or_else(
                || {
                    construct_path_from_hrid(
                        &self.root,
                        &hrid,
                        self.config.subfolders_are_namespaces,
                        self.config.digits(),
                    )
                },
                PathBuf::clone,
            );

            match requirement.save_to_path(&path, self.config.digits()) {
                Ok(()) => {
                    self.dirty.remove(&uuid);
                    flushed.push(hrid);
                }
                Err(err) => {
                    failures.push((path, err));
                }
            }
        }

        if let Some(failures) = NonEmpty::from_vec(failures) {
            return Err(FlushError { failures });
        }

        // Process deletions
        for path in &self.deletions {
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    eprintln!("Warning: Failed to delete {}: {}", path.display(), e);
                }
            }
        }
        self.deletions.clear();

        Ok(flushed)
    }
}

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

/// Error type for flush failures.
#[derive(Debug, thiserror::Error)]
pub struct FlushError {
    failures: NonEmpty<(PathBuf, io::Error)>,
}

impl fmt::Display for FlushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MAX_DISPLAY: usize = 5;

        write!(f, "failed to flush requirements: ")?;

        let total = self.failures.len();

        let displayed_paths: Vec<String> = self
            .failures
            .iter()
            .take(MAX_DISPLAY)
            .map(|(p, _e)| p.display().to_string())
            .collect();

        let msg = displayed_paths.join(", ");

        if total <= MAX_DISPLAY {
            write!(f, "{msg}")
        } else {
            write!(f, "{msg}... (and {} more)", total - MAX_DISPLAY)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{domain::requirement::Parent, Requirement};

    fn setup_temp_directory() -> (TempDir, Directory) {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().to_path_buf();
        (tmp, Directory::new(path).unwrap())
    }

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

        let config = load_config(&dir.root);
        let updated =
            Requirement::load(&dir.root, child.hrid(), &config).expect("should load child");

        let parents: Vec<_> = updated.parents().collect();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].0, parent.uuid());
        assert_eq!(&parents[0].1.hrid, parent.hrid());
    }

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

    #[test]
    fn load_all_reads_all_saved_requirements() {
        use std::str::FromStr;
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir.add_requirement("X", String::new()).unwrap();
        let r2 = dir.add_requirement("X", String::new()).unwrap();

        dir.flush().expect("flush should succeed");

        let loaded = Directory::new(dir.root.clone()).unwrap();

        let mut found = 0;
        for i in 1..=2 {
            let hrid = Hrid::from_str(&format!("X-00{i}")).unwrap();
            let req = Requirement::load(&loaded.root, &hrid, &loaded.config).unwrap();
            if req.uuid() == r1.uuid() || req.uuid() == r2.uuid() {
                found += 1;
            }
        }

        assert_eq!(found, 2);
    }

    #[test]
    fn path_based_mode_kind_in_filename() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = true
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        // Create directory structure and requirement file in path-based format
        std::fs::create_dir_all(root.join("SYSTEM/AUTH/REQ")).unwrap();

        std::fs::write(
            root.join("SYSTEM/AUTH/REQ/001.md"),
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789012
created: 2025-01-01T00:00:00Z
---
# SYSTEM-AUTH-REQ-001 Test requirement
",
        )
        .unwrap();

        // Load all requirements
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Should be able to load the requirement with the correct HRID using config
        let hrid = Hrid::try_from("SYSTEM-AUTH-REQ-001").unwrap();
        let req = Requirement::load(root, &hrid, &dir.config).unwrap();
        assert_eq!(req.hrid(), &hrid);
    }

    #[test]
    fn path_based_mode_kind_in_parent_folder() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = true
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        // Create directory structure with KIND as parent folder
        std::fs::create_dir_all(root.join("SYSTEM/AUTH/USR")).unwrap();

        // Create a requirement file with numeric filename
        std::fs::write(
            root.join("SYSTEM/AUTH/USR/001.md"),
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789013
created: 2025-01-01T00:00:00Z
---
# SYSTEM-AUTH-USR-001 Test requirement
",
        )
        .unwrap();

        // Load all requirements
        let _dir = Directory::new(root.to_path_buf()).unwrap();

        // Verify the requirement was loaded with correct HRID (KIND from parent folder)
        let hrid = Hrid::try_from("SYSTEM-AUTH-USR-001").unwrap();
        // The requirement should have been loaded from system/auth/USR/001.md during
        // load_all We verify it exists by checking the file was found
        let loaded_path = root.join("SYSTEM/AUTH/USR/001.md");
        assert!(loaded_path.exists());

        // Verify the requirement can be read directly from the file
        {
            use std::{fs::File, io::BufReader};

            use crate::storage::markdown::MarkdownRequirement;
            let file = File::open(&loaded_path).unwrap();
            let mut reader = BufReader::new(file);
            let md_req = MarkdownRequirement::read(&mut reader).unwrap();
            let req: Requirement = md_req.try_into().unwrap();
            assert_eq!(req.hrid(), &hrid);
        }
    }

    #[test]
    fn path_based_mode_saves_in_subdirectories() {
        use std::num::NonZeroUsize;

        use crate::domain::hrid::KindString;

        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = true
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        // Load directory
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec![
                KindString::new("SYSTEM".to_string()).unwrap(),
                KindString::new("AUTH".to_string()).unwrap(),
            ],
            KindString::new("REQ".to_string()).unwrap(),
            NonZeroUsize::new(1).unwrap(),
        );
        let req = Requirement::new(
            hrid.clone(),
            "Test Title".to_string(),
            "Test content".to_string(),
        );

        // Save using config
        req.save(root, &dir.config).unwrap();

        // File should be created at system/auth/REQ/001.md
        assert!(root.join("SYSTEM/AUTH/REQ/001.md").exists());

        // Should be able to reload it using config
        let loaded = Requirement::load(root, &hrid, &dir.config).unwrap();
        assert_eq!(loaded.hrid(), &hrid);
    }

    #[test]
    fn filename_based_mode_ignores_folder_structure() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = false (default)
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(root.join(".req/config.toml"), "_version = \"1\"\n").unwrap();

        // Create nested directory structure
        std::fs::create_dir_all(root.join("some/random/path")).unwrap();

        // Create a requirement with full HRID in filename
        std::fs::write(
            root.join("some/random/path/system-auth-REQ-001.md"),
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789014
created: 2025-01-01T00:00:00Z
---
# SYSTEM-AUTH-REQ-001 Test requirement
",
        )
        .unwrap();

        // Load all requirements
        let _dir = Directory::new(root.to_path_buf()).unwrap();

        // Verify the requirement was loaded with HRID from filename, not path
        // (The file is in some/random/path/ but the HRID comes from the filename)
        let hrid = Hrid::try_from("SYSTEM-AUTH-REQ-001").unwrap();
        // The requirement should have been loaded from the nested path during load_all
        // We verify it exists by checking it can be found in the directory structure
        let loaded_path = root.join("some/random/path/system-auth-REQ-001.md");
        assert!(loaded_path.exists());

        // Verify the requirement can be read directly from the file
        {
            use std::{fs::File, io::BufReader};

            use crate::storage::markdown::MarkdownRequirement;
            let file = File::open(&loaded_path).unwrap();
            let mut reader = BufReader::new(file);
            let md_req = MarkdownRequirement::read(&mut reader).unwrap();
            let req: Requirement = md_req.try_into().unwrap();
            assert_eq!(req.hrid(), &hrid);
        }
    }

    #[test]
    fn filename_based_mode_saves_in_root() {
        use std::num::NonZeroUsize;

        use crate::domain::hrid::KindString;

        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create default config (filename-based)
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(root.join(".req/config.toml"), "_version = \"1\"\n").unwrap();

        // Load directory
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec![
                KindString::new("SYSTEM".to_string()).unwrap(),
                KindString::new("AUTH".to_string()).unwrap(),
            ],
            KindString::new("REQ".to_string()).unwrap(),
            NonZeroUsize::new(1).unwrap(),
        );
        let req = Requirement::new(hrid, "Test Title".to_string(), "Test content".to_string());

        // Save using config
        req.save(root, &dir.config).unwrap();

        // File should be created in root with full HRID
        assert!(root.join("SYSTEM-AUTH-REQ-001.md").exists());
        assert!(!root.join("system/auth/REQ-001.md").exists());
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
    fn directory_new_rejects_disallowed_kinds() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // First create an empty directory and add a REQ requirement
        let mut dir = Directory::new(root.to_path_buf()).unwrap();
        dir.add_requirement("REQ", "# Test\n\nContent".to_string())
            .unwrap();
        dir.flush().unwrap();

        // Now update config to disallow REQ
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nallowed_kinds = [\"USR\", \"SYS\"]\n",
        )
        .unwrap();

        // Try to load directory
        let result = Directory::new(root.to_path_buf());

        // Should fail with DisallowedKinds error
        match result {
            Err(DirectoryLoadError::DisallowedKinds { files, .. }) => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].1, "REQ");
            }
            Ok(_) => panic!("Expected DisallowedKinds error, but directory loaded successfully"),
            Err(e) => panic!("Expected DisallowedKinds error, got: {e}"),
        }
    }

    #[test]
    fn directory_new_allows_when_no_kinds_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create a requirement file using add_requirement
        let mut dir = Directory::new(root.to_path_buf()).unwrap();
        dir.add_requirement("REQ", "# Test\n\nContent".to_string())
            .unwrap();
        dir.flush().unwrap();

        // Reload directory - should succeed with empty allowed_kinds (all allowed)
        let result = Directory::new(root.to_path_buf());

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
}
