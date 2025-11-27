//! Path construction utilities for requirements
//!
//! This module provides functions to construct file paths from HRIDs,
//! supporting two directory structure modes:
//!
//! 1. **Filename-based** (default): Full HRID encoded in filename
//!    - Example: `SYSTEM-AUTH-REQ-001` → `root/SYSTEM-AUTH-REQ-001.md`
//!
//! 2. **Path-based**: Subfolders encode namespace, filename contains KIND-ID
//!    - Example: `SYSTEM-AUTH-REQ-001` → `root/SYSTEM/AUTH/REQ-001.md`

use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use crate::domain::Hrid;

/// Construct a file path from an HRID.
///
/// If `subfolders_are_namespaces` is `false`, the file is placed directly in
/// the root with the full HRID as the filename. If `true`, namespace segments
/// become subdirectories.
///
/// # Arguments
///
/// * `root` - The root directory for requirements
/// * `hrid` - The HRID to convert to a path
/// * `subfolders_are_namespaces` - Whether to use path-based structure
/// * `digits` - Number of digits to pad the ID (e.g., 3 for "001")
#[must_use]
pub fn construct_path_from_hrid(
    root: &Path,
    hrid: &Hrid,
    subfolders_are_namespaces: bool,
    digits: usize,
) -> PathBuf {
    if subfolders_are_namespaces {
        // Path-based: root/namespace/folders/KIND/ID.md
        let mut path = root.to_path_buf();

        // Add namespace as subdirectories
        for segment in hrid.namespace() {
            path.push(segment);
        }

        // Add KIND as a subdirectory
        path.push(hrid.kind());

        // Add filename as numeric ID only
        let filename = format!("{:0width$}", hrid.id(), width = digits);
        path.push(filename);
        path.with_extension("md")
    } else {
        // Filename-based: root/FULL-HRID.md
        root.join(hrid.display(digits).to_string())
            .with_extension("md")
    }
}

/// Extract an HRID from a file path.
///
/// This is the inverse of `construct_path_from_hrid`. It parses the HRID from
/// a path based on the configuration mode.
///
/// # Errors
///
/// Returns an error if the path cannot be parsed into a valid HRID.
pub fn hrid_from_path(
    path: &Path,
    root: &Path,
    config: &crate::domain::Config,
) -> Result<Hrid, String> {
    // Remove root prefix and .md extension
    let relative_path = path.strip_prefix(root).map_err(|_| {
        format!(
            "Path {} is not under root {}",
            path.display(),
            root.display()
        )
    })?;

    let path_without_ext = relative_path.with_extension("");

    if config.subfolders_are_namespaces {
        // Path-based: parse namespace/KIND/ID
        let components: Vec<_> = path_without_ext.components().collect();

        if components.is_empty() {
            return Err("Path has no components".to_string());
        }

        // Last component is the ID (e.g., "001")
        let id_str = components
            .last()
            .and_then(|c| c.as_os_str().to_str())
            .ok_or_else(|| "Cannot extract ID from path".to_string())?;

        // Parse ID
        let id: NonZeroUsize = id_str
            .parse()
            .map_err(|_| format!("Invalid ID in path: {id_str}"))?;

        // Second-to-last component is the KIND
        if components.len() < 2 {
            return Err("Path must have at least KIND/ID".to_string());
        }

        let kind_str = components[components.len() - 2]
            .as_os_str()
            .to_str()
            .ok_or_else(|| "Cannot extract KIND from path".to_string())?;

        let kind = crate::domain::hrid::KindString::new(kind_str.to_string())
            .map_err(|e| format!("Invalid KIND: {e}"))?;

        // Everything before KIND is the namespace
        let namespace_strings: Vec<String> = components[..components.len() - 2]
            .iter()
            .map(|c| {
                c.as_os_str()
                    .to_str()
                    .ok_or_else(|| "Invalid namespace component".to_string())
                    .map(String::from)
            })
            .collect::<Result<_, _>>()?;

        // Convert namespace strings to NamespaceSegments
        let namespace: Vec<_> = namespace_strings
            .into_iter()
            .map(|s| {
                crate::domain::hrid::NamespaceSegment::new(s)
                    .map_err(|e| format!("Invalid namespace component: {e}"))
            })
            .collect::<Result<_, _>>()?;

        Ok(Hrid::new_with_namespace(namespace, kind, id))
    } else {
        // Filename-based: parse FULL-HRID from filename
        let filename = path_without_ext
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| "Cannot extract filename from path".to_string())?;

        filename
            .parse()
            .map_err(|e: crate::domain::HridError| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn construct_path_filename_based() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("SYSTEM-AUTH-REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, false, 3);
        assert_eq!(path, root.join("SYSTEM-AUTH-REQ-001.md"));
    }

    #[test]
    fn construct_path_path_based() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("SYSTEM-AUTH-REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 3);
        assert_eq!(path, root.join("SYSTEM/AUTH/REQ/001.md"));
    }

    #[test]
    fn construct_path_path_based_no_namespace() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 3);
        assert_eq!(path, root.join("REQ/001.md"));
    }

    #[test]
    fn construct_path_with_custom_digits() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 5);
        assert_eq!(path, root.join("REQ/00001.md"));
    }

    #[test]
    fn construct_path_with_lowercase_namespace() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("auth-api-SYS-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 3);
        assert_eq!(path, root.join("auth/api/SYS/001.md"));
    }

    #[test]
    fn construct_path_lowercase_namespace_filename_based() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("auth-api-SYS-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, false, 3);
        assert_eq!(path, root.join("auth-api-SYS-001.md"));
    }

    #[test]
    fn parse_hrid_from_path_filename_based() {
        let root = PathBuf::from("/root");
        let path = root.join("SYSTEM-AUTH-REQ-001.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = false;

        let hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(hrid.display(3).to_string(), "SYSTEM-AUTH-REQ-001");
    }

    #[test]
    fn parse_hrid_from_path_path_based() {
        let root = PathBuf::from("/root");
        let path = root.join("SYSTEM/AUTH/REQ/001.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = true;

        let hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(hrid.display(3).to_string(), "SYSTEM-AUTH-REQ-001");
    }

    #[test]
    fn parse_hrid_from_path_path_based_no_namespace() {
        let root = PathBuf::from("/root");
        let path = root.join("REQ/001.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = true;

        let hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(hrid.display(3).to_string(), "REQ-001");
    }

    #[test]
    fn parse_hrid_from_path_lowercase_namespace() {
        let root = PathBuf::from("/root");
        let path = root.join("auth/api/SYS/001.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = true;

        let hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(hrid.display(3).to_string(), "auth-api-SYS-001");
    }

    #[test]
    fn parse_hrid_from_path_lowercase_namespace_filename_based() {
        let root = PathBuf::from("/root");
        let path = root.join("auth-api-SYS-001.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = false;

        let hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(hrid.display(3).to_string(), "auth-api-SYS-001");
    }

    #[test]
    fn parse_hrid_from_path_not_under_root() {
        let root = PathBuf::from("/root");
        let path = PathBuf::from("/other/REQ-001.md");
        let config = crate::domain::Config::default();

        let result = hrid_from_path(&path, &root, &config);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hrid_from_path_invalid_id_path_based() {
        let root = PathBuf::from("/root");
        let path = root.join("REQ/invalid.md");
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = true;

        let result = hrid_from_path(&path, &root, &config);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hrid_from_path_roundtrip_filename_based() {
        let root = PathBuf::from("/root");
        let original_hrid = Hrid::try_from("auth-api-SYS-001").unwrap();

        let path = construct_path_from_hrid(&root, &original_hrid, false, 3);
        let config = crate::domain::Config::default();

        let parsed_hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(original_hrid.display(3).to_string(), parsed_hrid.display(3).to_string());
    }

    #[test]
    fn parse_hrid_from_path_roundtrip_path_based() {
        let root = PathBuf::from("/root");
        let original_hrid = Hrid::try_from("auth-api-SYS-001").unwrap();

        let path = construct_path_from_hrid(&root, &original_hrid, true, 3);
        let mut config = crate::domain::Config::default();
        config.subfolders_are_namespaces = true;

        let parsed_hrid = hrid_from_path(&path, &root, &config).unwrap();
        assert_eq!(original_hrid.display(3).to_string(), parsed_hrid.display(3).to_string());
    }
}
