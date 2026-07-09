//! Opening a directory and loading requirements from disk.

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use walkdir::WalkDir;

use super::Directory;
use crate::{
    domain::{requirement::LoadError, Config, Tree},
    Requirement,
};

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

    /// The `.req/config.toml` file exists but could not be read or parsed.
    InvalidConfig {
        /// The path of the config file.
        path: PathBuf,
        /// A description of the failure.
        message: String,
    },

    /// The requirements directory could not be traversed.
    Walk(#[from] walkdir::Error),
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
            Self::InvalidConfig { path, message } => {
                write!(f, "Failed to load config {}: {}", path.display(), message)
            }
            Self::Walk(error) => {
                write!(f, "Failed to traverse requirements directory: {error}")
            }
        }
    }
}

impl Directory {
    /// Opens a directory at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if unrecognised files are found when
    /// `allow_unrecognised` is false in the configuration.
    pub fn new(root: PathBuf) -> Result<Self, DirectoryLoadError> {
        Self::new_ignoring(root, &[])
    }

    /// Opens a directory at the given path, treating the given files as if
    /// they did not exist.
    ///
    /// This allows known non-requirement files kept inside the requirements
    /// root (such as an mdBook `SUMMARY.md`) to be skipped even when
    /// `allow_unrecognised` is false in the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if unrecognised files (other than the ignored ones)
    /// are found when `allow_unrecognised` is false in the configuration.
    pub fn new_ignoring(root: PathBuf, ignored: &[PathBuf]) -> Result<Self, DirectoryLoadError> {
        let config = load_config(&root)?;
        let ignored: HashSet<PathBuf> = ignored.iter().map(|path| lexical_absolute(path)).collect();
        let md_paths: Vec<PathBuf> = collect_markdown_paths(&root)?
            .into_iter()
            .filter(|path| !ignored.contains(&lexical_absolute(path)))
            .collect();

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

/// Load `.req/config.toml` from the root, if present.
///
/// A missing config file is the normal un-initialised case and yields the
/// default configuration. A config file that exists but cannot be read or
/// parsed is an error: silently falling back to defaults would flip settings
/// such as `subfolders_are_namespaces` and reinterpret the whole store.
pub(super) fn load_config(root: &Path) -> Result<Config, DirectoryLoadError> {
    let path = root.join(".req/config.toml");
    if path.exists() {
        Config::load(&path).map_err(|message| DirectoryLoadError::InvalidConfig { path, message })
    } else {
        Ok(Config::default())
    }
}

/// Make a path absolute and lexically resolve `.`/`..` components, so paths
/// arrived at via different routes (e.g. `root/../SUMMARY.md` vs
/// `SUMMARY.md`) compare equal. Purely lexical: symlinks are not resolved and
/// the path need not exist.
fn lexical_absolute(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    resolved
}

fn collect_markdown_paths(root: &PathBuf) -> Result<Vec<PathBuf>, DirectoryLoadError> {
    // A root that doesn't exist yet is an empty store, not an error.
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(root) {
        // Traversal errors (unreadable directories, dangling links) must
        // surface: silently skipping them would make requirements vanish.
        let entry = entry?;

        // Skip the .req directory (used for templates and other metadata)
        if entry.path().components().any(|c| c.as_os_str() == ".req") {
            continue;
        }
        if entry.path().extension() == Some(OsStr::new("md")) {
            paths.push(entry.into_path());
        }
    }
    Ok(paths)
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tempfile::TempDir;

    use super::{super::setup_temp_directory, *};
    use crate::domain::Hrid;

    #[test]
    fn load_all_reads_all_saved_requirements() {
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
        let hrid = Hrid::from_str("SYSTEM-AUTH-REQ-001").unwrap();
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
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Create default config (filename-based)
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(root.join(".req/config.toml"), "_version = \"1\"\n").unwrap();

        // Load directory
        let dir = Directory::new(root.to_path_buf()).unwrap();

        // Add a requirement with namespace
        let hrid = Hrid::from_str("SYSTEM-AUTH-REQ-001").unwrap();
        let req = Requirement::new(hrid, "Test Title".to_string(), "Test content".to_string());

        // Save using config
        req.save(root, &dir.config).unwrap();

        // File should be created in root with full HRID
        assert!(root.join("SYSTEM-AUTH-REQ-001.md").exists());
        assert!(!root.join("system/auth/REQ-001.md").exists());
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
    fn new_fails_on_malformed_config() {
        let tmp = TempDir::new().unwrap();
        let req_dir = tmp.path().join(".req");
        std::fs::create_dir_all(&req_dir).unwrap();
        std::fs::write(req_dir.join("config.toml"), "not valid toml [[[").unwrap();

        let result = Directory::new(tmp.path().to_path_buf());
        assert!(matches!(
            result,
            Err(DirectoryLoadError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn new_defaults_when_config_missing() {
        let (_tmp, dir) = setup_temp_directory();
        assert_eq!(dir.config, Config::default());
    }

    #[test]
    fn new_on_nonexistent_root_is_empty_store() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("does-not-exist");
        let dir = Directory::new(root).unwrap();
        assert_eq!(dir.requirements().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn new_reports_unreadable_subdirectory() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("locked");
        std::fs::create_dir(&sub).unwrap();
        let original_perms = std::fs::metadata(&sub).unwrap().permissions();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = Directory::new(tmp.path().to_path_buf());

        std::fs::set_permissions(&sub, original_perms).unwrap();
        assert!(matches!(result, Err(DirectoryLoadError::Walk(_))));
    }

    #[test]
    fn new_ignoring_skips_listed_files_under_strict_config() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path();

        // Strict config: unrecognised files are rejected.
        std::fs::create_dir_all(root.join(".req")).unwrap();
        std::fs::write(
            root.join(".req/config.toml"),
            "_version = \"1\"\nallow_unrecognised = false\n",
        )
        .unwrap();

        std::fs::write(
            root.join("USR-001.md"),
            r"---
_version: '1'
uuid: 12345678-1234-1234-1234-123456789014
created: 2025-01-01T00:00:00Z
---
# USR-001 Test requirement
",
        )
        .unwrap();

        // A non-requirement file, e.g. an mdBook summary.
        std::fs::write(root.join("SUMMARY.md"), "# Summary\n").unwrap();

        assert!(matches!(
            Directory::new(root.to_path_buf()),
            Err(DirectoryLoadError::UnrecognisedFiles(_))
        ));

        // Ignoring the summary makes the load succeed, including via an
        // unnormalized path route.
        let ignored = [root.join("subdir/../SUMMARY.md")];
        let dir = Directory::new_ignoring(root.to_path_buf(), &ignored).unwrap();
        assert_eq!(dir.requirements().count(), 1);
    }
}
