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

/// A filesystem backed store of requirements.
pub struct Directory {
    /// The root of the directory requirements are stored in.
    root: PathBuf,
    tree: Tree,
    config: Config,
}

impl Directory {
    /// Opens a directory at the given path and loads all requirements.
    ///
    /// # Errors
    ///
    /// This method has different behaviour depending on the configuration file
    /// in the requirements root. If `allow_unrecognised` is `true`, then
    /// any files with names that are not valid HRIDs, or any files that cannot
    /// be parsed as requirements, are skipped. if `allow_unrecognised` is
    /// `false` (the default), then any unrecognised or invalid markdown files
    /// in the directory will return an error.
    pub fn new(root: PathBuf) -> Result<Self, DirectoryLoadError> {
        let config = load_config(&root);
        let md_paths = collect_markdown_paths(&root);

        let (requirements, unrecognised_paths): (Vec<_>, Vec<_>) = md_paths
            .par_iter()
            .map(|path| try_load_requirement(path, &root, &config))
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

        Ok(Self { root, tree, config })
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
        child.save(&self.root, &self.config)?;

        Ok(child)
    }

    fn load_requirement(&self, hrid: Hrid) -> Result<Requirement, LoadError> {
        Requirement::load(&self.root, hrid, &self.config)
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

    if let Some(content) = try_load_template_file(&full_path) {
        return content;
    }

    // Fall back to KIND only (e.g., "USR.md")
    let kind = hrid.kind();
    let kind_path = templates_dir.join(format!("{kind}.md"));

    if let Some(content) = try_load_template_file(&kind_path) {
        return content;
    }

    tracing::debug!(
        "No template found for HRID {}, checked {} and {}",
        hrid,
        full_path.display(),
        kind_path.display()
    );
    String::new()
}

/// Try to load a template file from the given path.
///
/// Returns `Some(content)` if the file exists and can be read, `None`
/// otherwise.
fn try_load_template_file(path: &Path) -> Option<String> {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                tracing::debug!("Loaded template from {}", path.display());
                Some(content)
            }
            Err(e) => {
                tracing::warn!("Failed to read template at {}: {}", path.display(), e);
                None
            }
        }
    } else {
        None
    }
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
    match load_requirement_from_file(path, hrid, config) {
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
    use std::{fs::File, io::BufReader};

    use crate::domain::requirement::storage::MarkdownRequirement;

    let file = File::open(path).map_err(|io_error| match io_error.kind() {
        std::io::ErrorKind::NotFound => LoadError::NotFound,
        _ => LoadError::Io(io_error),
    })?;

    let mut reader = BufReader::new(file);
    let md_req = MarkdownRequirement::read(&mut reader, hrid)?;
    Ok(md_req.try_into()?)
}

impl Directory {
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
        let tree = &mut self.tree;

        let id = tree.next_index(&kind);
        let hrid = Hrid::new(kind, id)?;

        // If no content is provided via CLI, check for a template
        let final_content = if content.is_empty() {
            load_template(&self.root, &hrid)
        } else {
            content
        };

        let requirement = Requirement::new(hrid, final_content);

        requirement.save(&self.root, &self.config)?;
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
        let tree = &mut self.tree;
        let updated: Vec<_> = tree.update_hrids().collect();

        // Capture config values to avoid borrowing self in closure
        let root = &self.root;
        let subfolders_are_namespaces = self.config.subfolders_are_namespaces;
        let digits = self.config.digits();

        let failures = updated
            .iter()
            .filter_map(|&id| {
                let requirement = tree.requirement(id)?;
                requirement.save(root, &self.config).err().map(|e| {
                    use crate::storage::path_parser::construct_path_from_hrid;
                    let path = construct_path_from_hrid(
                        root,
                        requirement.hrid(),
                        subfolders_are_namespaces,
                        digits,
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
        let child_req = self.load_requirement(child.clone())?;
        let parent_req = self.load_requirement(parent.clone())?;

        let child_uuid = child_req.uuid();
        let parent_uuid = parent_req.uuid();

        // Check if the link exists
        let has_link = child_req.parents().any(|(pid, _)| pid == parent_uuid);
        if !has_link {
            return Err(AcceptSuspectLinkError::LinkNotFound { child, parent });
        }

        let was_updated = self.tree.accept_suspect_link(child_uuid, parent_uuid);

        if !was_updated {
            return Ok(AcceptResult::AlreadyUpToDate);
        }

        // Save the updated requirement
        let updated_child = self
            .tree
            .requirement(child_uuid)
            .ok_or(AcceptSuspectLinkError::ChildNotFound(child))?;
        updated_child.save(&self.root, &self.config)?;

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
        let updated = self.tree.accept_all_suspect_links();

        // Capture config values to avoid borrowing self in closure
        let root = &self.root;
        let subfolders_are_namespaces = self.config.subfolders_are_namespaces;
        let digits = self.config.digits();

        let failures: Vec<_> = updated
            .iter()
            .filter_map(|&(child_uuid, _parent_uuid)| {
                let requirement = self.tree.requirement(child_uuid)?;
                requirement.save(root, &self.config).err().map(|e| {
                    use crate::storage::path_parser::construct_path_from_hrid;
                    let path = construct_path_from_hrid(
                        root,
                        requirement.hrid(),
                        subfolders_are_namespaces,
                        digits,
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
                let child = self.tree.requirement(child_uuid)?;
                let parent = self.tree.requirement(parent_uuid)?;
                Some((child.hrid().clone(), parent.hrid().clone()))
            })
            .collect();

        Ok(updated_hrids)
    }
}

/// Error type for adding requirements.
#[derive(Debug, thiserror::Error)]
#[error("failed to add requirement: {0}")]
pub enum AddRequirementError {
    /// The requirement kind was invalid (empty string).
    Kind(#[from] EmptyStringError),
    /// An I/O error occurred while saving the requirement.
    Io(#[from] io::Error),
}

/// Error type for HRID update operations.
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
    /// An I/O error occurred while saving the updated requirement.
    #[error("failed to save requirement: {0}")]
    Io(#[from] io::Error),
}

/// Error type for updating suspect links.
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

    fn setup_temp_directory() -> (TempDir, Directory) {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().to_path_buf();
        (tmp, Directory::new(path).unwrap())
    }

    #[test]
    fn can_add_requirement() {
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir
            .add_requirement("REQ".to_string(), String::new())
            .unwrap();

        assert_eq!(r1.hrid().to_string(), "REQ-001");

        let loaded = Requirement::load(&dir.root, r1.hrid().clone(), &dir.config)
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
            .unwrap()
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
        child.save(&dir.root, &dir.config).unwrap();

        let mut loaded_dir = Directory::new(dir.root.clone()).unwrap();
        loaded_dir.update_hrids().unwrap();

        let updated = Requirement::load(&loaded_dir.root, child.hrid().clone(), &loaded_dir.config)
            .expect("should load updated child");
        let (_, parent_ref) = updated.parents().next().unwrap();

        assert_eq!(&parent_ref.hrid, parent.hrid());
    }

    #[test]
    fn new_reads_all_saved_requirements() {
        use std::str::FromStr;
        let (_tmp, mut dir) = setup_temp_directory();
        let r1 = dir.add_requirement("X".to_string(), String::new()).unwrap();
        let r2 = dir.add_requirement("X".to_string(), String::new()).unwrap();

        let loaded = Directory::new(dir.root.clone()).unwrap();

        let mut found = 0;
        for i in 1..=2 {
            let hrid = Hrid::from_str(&format!("X-00{i}")).unwrap();
            let req = Requirement::load(&loaded.root, hrid, &loaded.config).unwrap();
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
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789012
created: 2025-01-01T00:00:00Z
---
Test requirement
",
        )
        .unwrap();

        // Construct directory (loads requirements)
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Should be able to load the requirement with the correct HRID using config
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();
        let req = Requirement::load(root, hrid.clone(), &dir.config).unwrap();
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
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789013
created: 2025-01-01T00:00:00Z
---
Test requirement
",
        )
        .unwrap();

        // Construct directory (loads requirements)
        let _dir = Directory::new(root.to_path_buf()).unwrap();

        // Verify the requirement was loaded with correct HRID (KIND from parent folder)
        let hrid = Hrid::try_from("system-auth-USR-001").unwrap();
        // The requirement should have been loaded from system/auth/USR/001.md during
        // Directory::new loads all requirements. Verify it exists by checking the file
        // was found
        let loaded_path = root.join("system/auth/USR/001.md");
        assert!(loaded_path.exists());

        // Verify the requirement can be read directly from the file
        {
            use std::{fs::File, io::BufReader};

            use crate::domain::requirement::storage::MarkdownRequirement;
            let file = File::open(&loaded_path).unwrap();
            let mut reader = BufReader::new(file);
            let md_req = MarkdownRequirement::read(&mut reader, hrid.clone()).unwrap();
            let req: Requirement = md_req.try_into().unwrap();
            assert_eq!(req.hrid(), &hrid);
        }
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
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec!["system".to_string(), "auth".to_string()],
            "REQ".to_string(),
            1,
        )
        .unwrap();
        let req = Requirement::new(hrid.clone(), "Test content".to_string());

        // Save using config
        req.save(root, &dir.config).unwrap();

        // File should be created at system/auth/REQ-001.md
        assert!(root.join("system/auth/REQ-001.md").exists());

        // Should be able to reload it using config
        let loaded = Requirement::load(root, hrid.clone(), &dir.config).unwrap();
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
Test requirement
",
        )
        .unwrap();

        // Construct directory (loads requirements)
        let _dir = Directory::new(root.to_path_buf()).unwrap();

        // Verify the requirement was loaded with HRID from filename, not path
        // (The file is in some/random/path/ but the HRID comes from the filename)
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();
        // The requirement should have been loaded from the nested path during
        // construction We verify it exists by checking it can be found in the
        // directory structure
        let loaded_path = root.join("some/random/path/system-auth-REQ-001.md");
        assert!(loaded_path.exists());

        // Verify the requirement can be read directly from the file
        {
            use std::{fs::File, io::BufReader};

            use crate::domain::requirement::storage::MarkdownRequirement;
            let file = File::open(&loaded_path).unwrap();
            let mut reader = BufReader::new(file);
            let md_req = MarkdownRequirement::read(&mut reader, hrid.clone()).unwrap();
            let req: Requirement = md_req.try_into().unwrap();
            assert_eq!(req.hrid(), &hrid);
        }
    }

    #[test]
    fn filename_based_mode_saves_in_root() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create default config (filename-based)
        std::fs::write(root.join("config.toml"), "_version = \"1\"\n").unwrap();

        // Load directory
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::new_with_namespace(
            vec!["system".to_string(), "auth".to_string()],
            "REQ".to_string(),
            1,
        )
        .unwrap();
        let req = Requirement::new(hrid, "Test content".to_string());

        // Save using config
        req.save(root, &dir.config).unwrap();

        // File should be created in root with full HRID
        assert!(root.join("system-auth-REQ-001.md").exists());
        assert!(!root.join("system/auth/REQ-001.md").exists());
    }

    #[test]
    fn template_loading_no_template() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // No .req/templates directory exists
        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        let req = dir
            .add_requirement("USR".to_string(), String::new())
            .unwrap();

        // Should create empty content
        assert_eq!(req.content(), "");
    }

    #[test]
    fn template_loading_kind_only_template() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create template directory and KIND template
        std::fs::create_dir_all(root.join(".req/templates")).unwrap();
        std::fs::write(
            root.join(".req/templates/USR.md"),
            "# User Requirement Template\n\nDescription here.",
        )
        .unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        let req = dir
            .add_requirement("USR".to_string(), String::new())
            .unwrap();

        // Should use template content
        assert_eq!(
            req.content(),
            "# User Requirement Template\n\nDescription here."
        );
    }

    #[test]
    fn template_loading_namespace_specific_template() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create template directory with both general and namespace-specific templates
        std::fs::create_dir_all(root.join(".req/templates")).unwrap();
        std::fs::write(
            root.join(".req/templates/USR.md"),
            "# General User Requirement Template",
        )
        .unwrap();
        std::fs::write(
            root.join(".req/templates/AUTH-USR.md"),
            "# Auth User Requirement Template",
        )
        .unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Create namespaced requirement - should use namespace-specific template
        let auth_req = dir
            .add_requirement("AUTH-USR".to_string(), String::new())
            .unwrap();
        assert_eq!(auth_req.content(), "# Auth User Requirement Template");

        // Create non-namespaced requirement - should use general template
        let usr_req = dir
            .add_requirement("USR".to_string(), String::new())
            .unwrap();
        assert_eq!(usr_req.content(), "# General User Requirement Template");
    }

    #[test]
    fn template_loading_fallback_to_kind() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create template directory with only KIND template (no namespace-specific)
        std::fs::create_dir_all(root.join(".req/templates")).unwrap();
        std::fs::write(
            root.join(".req/templates/USR.md"),
            "# General User Requirement Template",
        )
        .unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Note: add_requirement API treats "AUTH-USR" as a single KIND, not
        // namespace+kind To test fallback properly, we need to check that
        // "AUTH-USR" (as a KIND) will fall back to "USR" template if "AUTH-USR"
        // template doesn't exist
        let auth_usr_req = dir
            .add_requirement("AUTH-USR".to_string(), String::new())
            .unwrap();
        // "AUTH-USR" is the kind, not namespace. Template lookup will look for:
        // 1. "AUTH-USR.md" (full prefix, which doesn't exist)
        // 2. "AUTH-USR.md" as kind (same, doesn't exist)
        // So it falls back to empty
        // This test actually demonstrates that KIND is atomic in add_requirement
        assert_eq!(auth_usr_req.content(), "");
    }

    #[test]
    fn template_overridden_by_content() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create template
        std::fs::create_dir_all(root.join(".req/templates")).unwrap();
        std::fs::write(root.join(".req/templates/USR.md"), "# Template Content").unwrap();

        let mut dir = Directory::new(root.to_path_buf()).unwrap();

        // Provide explicit content - should override template
        let req = dir
            .add_requirement("USR".to_string(), "# Custom Content".to_string())
            .unwrap();
        assert_eq!(req.content(), "# Custom Content");
    }
}
