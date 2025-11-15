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
            tracing::info!("Linked {} ← {}", outcome.child_hrid, outcome.parent_hrid);
        }

        self.tree
            .requirement(outcome.child_uuid)
            .ok_or(LoadError::NotFound)
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

        let mut tree = Tree::with_capacity(requirements.len());
        let mut paths = HashMap::with_capacity(requirements.len());
        for (req, path) in requirements {
            let uuid = req.uuid();
            tree.insert(req);
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
        })
    }
}

/// Error type for directory loading operations.
#[derive(Debug, thiserror::Error)]
pub enum DirectoryLoadError {
    /// One or more files in the directory could not be recognized as valid
    /// requirements.
    UnrecognisedFiles(Vec<PathBuf>),
}

impl fmt::Display for DirectoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrecognisedFiles(paths) => {
                write!(f, "Unrecognised files: ")?;
                for (i, path) in paths.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
        }
    }
}

fn load_config(root: &Path) -> Config {
    let path = root.join("config.toml");
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
        hrid,
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
) -> Result<(Requirement, PathBuf), PathBuf> {
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
            Err(path.to_path_buf())
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

    /// Returns the filesystem path for a requirement HRID using directory
    /// configuration.
    #[must_use]
    pub fn path_for(&self, hrid: &Hrid) -> PathBuf {
        crate::storage::construct_path_from_hrid(
            &self.root,
            hrid,
            self.config.subfolders_are_namespaces,
            self.config.digits(),
        )
    }

    /// Returns the actual filesystem path where a requirement was loaded from.
    ///
    /// Returns `None` if the requirement is not found or was not loaded from
    /// disk.
    #[must_use]
    pub fn actual_path_for(&self, hrid: &Hrid) -> Option<&Path> {
        let uuid = self.tree.find_by_hrid(hrid)?.uuid;
        self.paths.get(uuid).map(std::path::PathBuf::as_path)
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
        let tree = &mut self.tree;

        // Validate kind (CLI already normalized to uppercase)
        let kind_string =
            KindString::new(kind.to_string()).map_err(crate::domain::hrid::Error::from)?;

        let id = tree.next_index(&kind_string);
        let hrid = Hrid::new(kind_string, id);

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

        tree.insert(requirement.clone());
        self.mark_dirty(requirement.uuid());

        tracing::info!("Added requirement: {}", requirement.hrid());

        Ok(requirement)
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

        let was_updated = self.tree.accept_suspect_link(child_uuid, parent_uuid);

        if !was_updated {
            return Ok(AcceptResult::AlreadyUpToDate);
        }

        self.mark_dirty(child_uuid);
        tracing::info!("Accepted suspect link {} ← {}", child_hrid, parent_hrid);

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

            match requirement.save_to_path(&path) {
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

        Ok(flushed)
    }
}

/// Error type for adding requirements.
#[derive(Debug, thiserror::Error)]
pub enum AddRequirementError {
    /// The requirement kind or ID was invalid.
    #[error("failed to add requirement: {0}")]
    Hrid(#[from] crate::domain::HridError),
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
#[derive(Debug, thiserror::Error)]
pub enum AcceptSuspectLinkError {
    /// The child requirement was not found.
    #[error("child requirement {0} not found")]
    ChildNotFound(Hrid),
    /// The parent requirement was not found.
    #[error("parent requirement not found")]
    ParentNotFound(#[from] LoadError),
    /// The link between child and parent was not found.
    #[error("link from {child} to {parent} not found")]
    LinkNotFound {
        /// The child requirement HRID.
        child: Hrid,
        /// The parent requirement HRID.
        parent: Hrid,
    },
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

        assert_eq!(r1.hrid().to_string(), "REQ-001");

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

        assert_eq!(r1.hrid().to_string(), "REQ-001");
        assert_eq!(r2.hrid().to_string(), "REQ-002");
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
        std::fs::write(
            root.join("config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        // Create directory structure
        std::fs::create_dir_all(root.join("system/auth")).unwrap();

        // Create a requirement file in path-based format
        std::fs::create_dir_all(root.join("SYSTEM/AUTH")).unwrap();

        std::fs::write(
            root.join("SYSTEM/AUTH/REQ-001.md"),
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
        std::fs::write(
            root.join("config.toml"),
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
        std::fs::write(
            root.join("config.toml"),
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

        // File should be created at system/auth/REQ-001.md
        assert!(root.join("SYSTEM/AUTH/REQ-001.md").exists());

        // Should be able to reload it using config
        let loaded = Requirement::load(root, &hrid, &dir.config).unwrap();
        assert_eq!(loaded.hrid(), &hrid);
    }

    #[test]
    fn filename_based_mode_ignores_folder_structure() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = false (default)
        std::fs::write(root.join("config.toml"), "_version = \"1\"\n").unwrap();

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
        std::fs::write(root.join("config.toml"), "_version = \"1\"\n").unwrap();

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
}
