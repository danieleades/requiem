//! A filesystem backed store of requirements
//!
//! The [`Directory`] provides a way to manage requirements stored in a
//! directory structure. It is a wrapper around the filesystem agnostic
//! [`Tree`].

use std::{
    ffi::OsStr,
    fmt::{self},
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use nonempty::NonEmpty;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use walkdir::WalkDir;

pub use crate::storage::Tree;
use crate::{
    domain::{
        requirement::{LoadError, Parent},
        Config, Hrid,
    },
    storage::path_parser::parse_hrid_from_path,
    EmptyStringError, Requirement,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    tree: Tree,
    config: Config,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Unloaded;

/// A filesystem backed store of requirements.
pub struct Directory<S> {
    /// The root of the directory requirements are stored in.
    root: PathBuf,
    state: S,
}

impl<S> Directory<S> {
    /// Link two requirements together with a parent-child relationship.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - either the child or parent requirement file cannot be found
    /// - either the child or parent requirement file cannot be parsed
    /// - the child requirement file cannot be written to
    pub fn link_requirement(&self, child: Hrid, parent: Hrid) -> Result<Requirement, LoadError> {
        let mut child = self.load_requirement(child)?;
        let parent = self.load_requirement(parent)?;

        child.add_parent(
            parent.uuid(),
            Parent {
                hrid: parent.hrid().clone(),
                fingerprint: parent.fingerprint(),
            },
        );

        // Load config to use for saving
        let config = load_config(&self.root);
        child.save(&self.root, &config)?;

        Ok(child)
    }

    fn load_requirement(&self, hrid: Hrid) -> Result<Requirement, LoadError> {
        // Load config to use config-aware loading
        let config = load_config(&self.root);
        Requirement::load_with_config(&self.root, hrid, &config)
    }
}

impl Directory<Unloaded> {
    /// Opens a directory at the given path.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Unloaded,
        }
    }

    /// Load all requirements from disk
    ///
    /// # Errors
    ///
    /// This method has different behaviour depending on the configuration file
    /// in the requirements root. If `allow_unrecognised` is `true`, then
    /// any files with names that are not valid HRIDs, or any files that cannot
    /// be parsed as requirements, are skipped. if `allow_unrecognised` is
    /// `false` (the default), then any unrecognised or invalid markdown files
    /// in the directory will return an error.
    pub fn load_all(self) -> Result<Directory<Loaded>, DirectoryLoadError> {
        let config = load_config(&self.root);
        let md_paths = collect_markdown_paths(&self.root);

        let (requirements, unrecognised_paths): (Vec<_>, Vec<_>) = md_paths
            .par_iter()
            .map(|path| try_load_requirement(path, &self.root, &config))
            .partition(Result::is_ok);

        let requirements: Vec<_> = requirements.into_iter().map(Result::unwrap).collect();
        let unrecognised_paths: Vec<_> = unrecognised_paths
            .into_iter()
            .map(Result::unwrap_err)
            .collect();

        if !config.allow_unrecognised && !unrecognised_paths.is_empty() {
            return Err(DirectoryLoadError::UnrecognisedFiles(unrecognised_paths));
        }

        let mut tree = Tree::with_capacity(requirements.len());
        for req in requirements {
            tree.insert(req);
        }

        Ok(Directory {
            root: self.root,
            state: Loaded { tree, config },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DirectoryLoadError {
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

fn try_load_requirement(path: &Path, root: &Path, config: &Config) -> Result<Requirement, PathBuf> {
    // Parse HRID from path using the configured structure mode
    let hrid = match parse_hrid_from_path(path, root, config.subfolders_are_namespaces) {
        Ok(hrid) => hrid,
        Err(e) => {
            tracing::debug!(
                "Skipping file with invalid HRID at {}: {:?}",
                path.display(),
                e
            );
            return Err(path.to_path_buf());
        }
    };

    // Load the requirement from the file
    // For path-based mode, we need to load from the actual file path
    // For filename-based mode, we load from the parent directory
    match load_requirement_from_file(path, hrid.clone(), config) {
        Ok(req) => Ok(req),
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

fn load_requirement_from_file(
    path: &Path,
    hrid: Hrid,
    _config: &Config,
) -> Result<Requirement, LoadError> {
    // Load directly from the file path we found during directory scanning
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).map_err(|io_error| match io_error.kind() {
        std::io::ErrorKind::NotFound => LoadError::NotFound,
        _ => LoadError::Io(io_error),
    })?;

    let mut reader = BufReader::new(file);
    use crate::domain::requirement::storage::MarkdownRequirement;
    let md_req = MarkdownRequirement::read(&mut reader, hrid)?;
    Ok(Requirement::from(md_req))
}

impl Directory<Loaded> {
    /// Add a new requirement to the directory.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    ///
    /// - the provided `kind` is an empty string
    /// - the requirement file cannot be written to
    pub fn add_requirement(
        &mut self,
        kind: String,
        content: String,
    ) -> Result<Requirement, AddRequirementError> {
        let tree = &mut self.state.tree;

        let id = tree.next_index(&kind);
        let hrid = Hrid::new(kind, id)?;

        // If no content is provided via CLI, check for a template
        let final_content = if content.is_empty() {
            load_template(&self.root, &hrid)
        } else {
            content
        };

        let requirement = Requirement::new(hrid, final_content);

        requirement.save(&self.root, &self.state.config)?;
        tree.insert(requirement.clone());

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
    pub fn update_hrids(&mut self) -> Result<(), UpdateHridsError> {
        let tree = &mut self.state.tree;
        let updated: Vec<_> = tree.update_hrids().collect();

        let failures = updated
            .iter()
            .filter_map(|&id| {
                let requirement = tree.requirement(id)?;
                requirement
                    .save(&self.root, &self.state.config)
                    .err()
                    .map(|e| {
                        // Construct the path using the same logic as save_with_config
                        use crate::storage::path_parser::construct_path_from_hrid;
                        let path = construct_path_from_hrid(
                            &self.root,
                            requirement.hrid(),
                            self.state.config.subfolders_are_namespaces,
                            self.state.config.digits(),
                        );
                        (path, e)
                    })
            })
            .collect();

        NonEmpty::from_vec(failures).map_or(Ok(()), |failures| Err(UpdateHridsError { failures }))
    }

    /// Find all suspect links in the requirement graph.
    ///
    /// A link is suspect when the fingerprint stored in a child requirement
    /// does not match the current fingerprint of the parent requirement.
    #[must_use]
    pub fn suspect_links(&self) -> Vec<crate::storage::SuspectLink> {
        self.state.tree.suspect_links()
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
        let child_req = self.load_requirement(child.clone())?;
        let parent_req = self.load_requirement(parent.clone())?;

        let child_uuid = child_req.uuid();
        let parent_uuid = parent_req.uuid();

        // Check if the link exists
        let has_link = child_req.parents().any(|(pid, _)| pid == parent_uuid);
        if !has_link {
            return Err(AcceptSuspectLinkError::LinkNotFound { child, parent });
        }

        let was_updated = self.state.tree.accept_suspect_link(child_uuid, parent_uuid);

        if !was_updated {
            return Ok(AcceptResult::AlreadyUpToDate);
        }

        // Save the updated requirement
        let updated_child = self
            .state
            .tree
            .requirement(child_uuid)
            .ok_or_else(|| AcceptSuspectLinkError::ChildNotFound(child))?;
        updated_child.save(&self.root, &self.state.config)?;

        Ok(AcceptResult::Updated)
    }

    /// Accept all suspect links by updating all fingerprints.
    ///
    /// # Errors
    ///
    /// Returns an error if any requirement file cannot be saved.
    /// This method does not fail fast - it will attempt to save all
    /// requirements before returning the error.
    pub fn accept_all_suspect_links(
        &mut self,
    ) -> Result<Vec<(Hrid, Hrid)>, UpdateSuspectLinksError> {
        let updated = self.state.tree.accept_all_suspect_links();

        let failures: Vec<_> = updated
            .iter()
            .filter_map(|&(child_uuid, _parent_uuid)| {
                let requirement = self.state.tree.requirement(child_uuid)?;
                requirement
                    .save(&self.root, &self.state.config)
                    .err()
                    .map(|e| {
                        // Construct the path using the same logic as save_with_config
                        use crate::storage::path_parser::construct_path_from_hrid;
                        let path = construct_path_from_hrid(
                            &self.root,
                            requirement.hrid(),
                            self.state.config.subfolders_are_namespaces,
                            self.state.config.digits(),
                        );
                        (path, e)
                    })
            })
            .collect();

        if let Some(failures) = NonEmpty::from_vec(failures) {
            return Err(UpdateSuspectLinksError { failures });
        }

        // Convert UUIDs to HRIDs for return value
        let updated_hrids: Vec<_> = updated
            .iter()
            .filter_map(|&(child_uuid, parent_uuid)| {
                let child = self.state.tree.requirement(child_uuid)?;
                let parent = self.state.tree.requirement(parent_uuid)?;
                Some((child.hrid().clone(), parent.hrid().clone()))
            })
            .collect();

        Ok(updated_hrids)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to add requirement: {0}")]
pub enum AddRequirementError {
    Kind(#[from] EmptyStringError),
    Io(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub struct UpdateHridsError {
    failures: NonEmpty<(PathBuf, io::Error)>,
}

impl fmt::Display for UpdateHridsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MAX_DISPLAY: usize = 5;

        write!(f, "failed to update HRIDS: ")?;

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

#[derive(Debug)]
pub enum AcceptResult {
    Updated,
    AlreadyUpToDate,
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptSuspectLinkError {
    #[error("child requirement {0} not found")]
    ChildNotFound(Hrid),
    #[error("parent requirement not found")]
    ParentNotFound(#[from] LoadError),
    #[error("link from {child} to {parent} not found")]
    LinkNotFound { child: Hrid, parent: Hrid },
    #[error("failed to save requirement: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub struct UpdateSuspectLinksError {
    failures: NonEmpty<(PathBuf, io::Error)>,
}

impl fmt::Display for UpdateSuspectLinksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MAX_DISPLAY: usize = 5;

        write!(f, "failed to update suspect links: ")?;

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::Requirement;

    fn setup_temp_directory() -> (TempDir, Directory<Loaded>) {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().to_path_buf();
        (tmp, Directory::new(path).load_all().unwrap())
    }

    #[test]
    fn can_add_requirement() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir
            .add_requirement("REQ".to_string(), String::new())
            .unwrap();

        assert_eq!(r1.hrid().to_string(), "REQ-001");

        let loaded = Requirement::load(&dir.root, r1.hrid().clone(), &dir.state.config)
            .expect("should load saved requirement");
        assert_eq!(loaded.uuid(), r1.uuid());
    }

    #[test]
    fn can_add_multiple_requirements_with_incrementing_id() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir
            .add_requirement("REQ".to_string(), String::new())
            .unwrap();
        let r2 = dir
            .add_requirement("REQ".to_string(), String::new())
            .unwrap();

        assert_eq!(r1.hrid().to_string(), "REQ-001");
        assert_eq!(r2.hrid().to_string(), "REQ-002");
    }

    #[test]
    fn can_link_two_requirements() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir
            .add_requirement("SYS".to_string(), String::new())
            .unwrap();
        let child = dir
            .add_requirement("USR".to_string(), String::new())
            .unwrap();

        Directory::new(dir.root.clone())
            .link_requirement(child.hrid().clone(), parent.hrid().clone())
            .unwrap();

        let config = load_config(&dir.root);
        let updated =
            Requirement::load(&dir.root, child.hrid().clone(), &config).expect("should load child");

        let parents: Vec<_> = updated.parents().collect();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].0, parent.uuid());
        assert_eq!(&parents[0].1.hrid, parent.hrid());
    }

    #[test]
    fn update_hrids_corrects_outdated_parent_hrids() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir.add_requirement("P".to_string(), String::new()).unwrap();
        let mut child = dir.add_requirement("C".to_string(), String::new()).unwrap();

        // Manually corrupt HRID in child's parent info
        child.add_parent(
            parent.uuid(),
            Parent {
                hrid: Hrid::try_from("WRONG-999").unwrap(),
                fingerprint: parent.fingerprint(),
            },
        );
        child.save(&dir.root).unwrap();

        let mut loaded_dir = Directory::new(dir.root.clone()).load_all().unwrap();
        loaded_dir.update_hrids().unwrap();

        let updated = Requirement::load(
            &loaded_dir.root,
            child.hrid().clone(),
            &loaded_dir.state.config,
        )
        .expect("should load updated child");
        let (_, parent_ref) = updated.parents().next().unwrap();

        assert_eq!(&parent_ref.hrid, parent.hrid());
    }

    #[test]
    fn load_all_reads_all_saved_requirements() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir.add_requirement("X".to_string(), String::new()).unwrap();
        let r2 = dir.add_requirement("X".to_string(), String::new()).unwrap();

        let loaded = Directory::new(dir.root.clone()).load_all().unwrap();

        let mut found = 0;
        for i in 1..=2 {
            let hrid = Hrid::from_str(&format!("X-00{i}")).unwrap();
            let req = Requirement::load(&loaded.root, hrid, &loaded.state.config).unwrap();
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
        std::fs::write(
            root.join("system/auth/REQ-001.md"),
            r#"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789012
created: 2025-01-01T00:00:00Z
---
Test requirement
"#,
        )
        .unwrap();

        // Load all requirements
        let dir = Directory::new(root.to_path_buf()).load_all().unwrap();

        // Should be able to load the requirement with the correct HRID using config
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();
        let req = Requirement::load_with_config(&root, hrid.clone(), &dir.state.config).unwrap();
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
        std::fs::create_dir_all(root.join("system/auth/USR")).unwrap();

        // Create a requirement file with numeric filename
        std::fs::write(
            root.join("system/auth/USR/001.md"),
            r#"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789013
created: 2025-01-01T00:00:00Z
---
Test requirement
"#,
        )
        .unwrap();

        // Load all requirements
        let dir = Directory::new(root.to_path_buf()).load_all().unwrap();

        // Should be able to load with correct HRID (KIND from parent folder) using config
        let hrid = Hrid::try_from("system-auth-USR-001").unwrap();
        let req = Requirement::load_with_config(&root, hrid.clone(), &dir.state.config).unwrap();
        assert_eq!(req.hrid(), &hrid);
    }

    #[test]
    fn path_based_mode_saves_in_subdirectories() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create config with subfolders_are_namespaces = true
        std::fs::write(
            root.join("config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        // Load directory
        let mut dir = Directory::new(root.to_path_buf()).load_all().unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec!["system".to_string(), "auth".to_string()],
            "REQ".to_string(),
            1,
        )
        .unwrap();
        let req = Requirement::new(hrid.clone(), "Test content".to_string());

        // Save using config
        req.save(&root, &dir.state.config).unwrap();

        // File should be created at system/auth/REQ-001.md
        assert!(root.join("system/auth/REQ-001.md").exists());

        // Should be able to reload it using config
        let loaded = Requirement::load_with_config(&root, hrid.clone(), &dir.state.config).unwrap();
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
            r#"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789014
created: 2025-01-01T00:00:00Z
---
Test requirement
"#,
        )
        .unwrap();

        // Load all requirements
        let dir = Directory::new(root.to_path_buf()).load_all().unwrap();

        // Should load with HRID from filename, not path
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();
        let req = Requirement::load(&root, hrid.clone(), &dir.state.config).unwrap();
        assert_eq!(req.hrid(), &hrid);
    }

    #[test]
    fn filename_based_mode_saves_in_root() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create default config (filename-based)
        std::fs::write(root.join("config.toml"), "_version = \"1\"\n").unwrap();

        // Load directory
        let mut dir = Directory::new(root.to_path_buf()).load_all().unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec!["system".to_string(), "auth".to_string()],
            "REQ".to_string(),
            1,
        )
        .unwrap();
        let req = Requirement::new(hrid.clone(), "Test content".to_string());

        // Save using config
        req.save(&root, &dir.state.config).unwrap();

        // File should be created in root with full HRID
        assert!(root.join("system-auth-REQ-001.md").exists());
        assert!(!root.join("system/auth/REQ-001.md").exists());
    }
}
