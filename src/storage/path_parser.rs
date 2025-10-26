//! Path parsing utilities for requirements
//!
//! This module provides functions to parse HRIDs from file paths, supporting
//! two directory structure modes:
//!
//! 1. **Filename-based** (default): Full HRID encoded in filename
//!    - Example: `custom/folder/system-auth-REQ-001.md` → `system-auth-REQ-001`
//!
//! 2. **Path-based**: Subfolders encode namespace, filename contains KIND-ID
//!    - Example: `system/auth/REQ-001.md` → `system-auth-REQ-001`
//!    - Example: `system/auth/USR/001.md` → `system-auth-USR-001`
//!    - Format is inferred: numeric filename → KIND in parent folder

use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use non_empty_string::NonEmptyString;

use crate::domain::{hrid::Error as HridError, Hrid};

/// Parse HRID from a file path.
///
/// If `subfolders_are_namespaces` is `false`, the HRID is parsed entirely from
/// the filename stem. If `true`, the namespace is extracted from the directory
/// structure, and the KIND and ID are inferred from the filename.
///
/// # Errors
///
/// Returns an error if:
/// - The path is invalid or cannot be parsed
/// - The filename doesn't match expected patterns
/// - The HRID components are malformed
pub fn parse_hrid_from_path(
    path: &Path,
    root: &Path,
    subfolders_are_namespaces: bool,
) -> Result<Hrid, ParseError> {
    let filename_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or(ParseError::InvalidPath)?;

    if !subfolders_are_namespaces {
        // Current behavior: parse HRID from filename only
        Hrid::from_str(filename_stem).map_err(ParseError::Hrid)
    } else {
        // Path-based: extract namespace from subfolders
        parse_with_namespace_from_path(path, root, filename_stem)
    }
}

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
    if !subfolders_are_namespaces {
        // Filename-based: root/FULL-HRID.md
        root.join(hrid.to_string()).with_extension("md")
    } else {
        // Path-based: root/namespace/folders/KIND-ID.md
        let mut path = root.to_path_buf();

        // Add namespace as subdirectories
        for segment in hrid.namespace() {
            path.push(segment);
        }

        // Add filename as KIND-ID.md
        let filename = format!("{}-{:0width$}", hrid.kind(), hrid.id(), width = digits);
        path.push(filename);
        path.with_extension("md")
    }
}

/// Parse HRID when subfolders encode namespace.
///
/// The format is inferred from the filename:
/// - Pure numeric (e.g., "001") → KIND from parent folder, ID from filename
/// - KIND-ID format (e.g., "REQ-001") → KIND and ID from filename
fn parse_with_namespace_from_path(
    path: &Path,
    root: &Path,
    filename_stem: &str,
) -> Result<Hrid, ParseError> {
    // Get relative path from root
    let rel_path = path
        .strip_prefix(root)
        .map_err(|_| ParseError::InvalidPath)?;

    // Extract namespace from parent directories
    let parent_components: Vec<String> = rel_path
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| {
                    if let Component::Normal(s) = c {
                        s.to_str().map(String::from)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // INFER FORMAT from filename
    if filename_stem.parse::<usize>().is_ok() {
        // Filestem is pure numeric → KIND in parent folder
        parse_kind_in_parent(&parent_components, filename_stem)
    } else if filename_stem.contains('-') {
        // Filestem has dash → try KIND-ID format
        parse_kind_in_filename(&parent_components, filename_stem)
    } else {
        Err(ParseError::InvalidFormat(
            "Filename must be either numeric (ID only) or KIND-ID format".into(),
        ))
    }
}

/// Parse HRID when KIND is in the parent directory name.
///
/// Example: `system/auth/REQ/001.md` → `system-auth-REQ-001`
fn parse_kind_in_parent(components: &[String], id_str: &str) -> Result<Hrid, ParseError> {
    if components.is_empty() {
        return Err(ParseError::MissingKind);
    }

    // Last component is KIND
    let kind = NonEmptyString::from_str(components.last().unwrap())
        .map_err(|_| ParseError::InvalidKind)?;

    // Remaining components are namespace
    let namespace: Result<Vec<_>, _> = components[..components.len() - 1]
        .iter()
        .map(|s| NonEmptyString::from_str(s))
        .collect();
    let namespace = namespace.map_err(|_| ParseError::InvalidNamespace)?;

    // Parse numeric ID
    let id = id_str
        .parse::<usize>()
        .map_err(|_| ParseError::InvalidId(id_str.into()))?;

    Ok(Hrid::new_with_namespace_unchecked(namespace, kind, id))
}

/// Parse HRID when KIND and ID are in the filename.
///
/// Example: `system/auth/REQ-001.md` → `system-auth-REQ-001`
fn parse_kind_in_filename(
    components: &[String],
    filename_stem: &str,
) -> Result<Hrid, ParseError> {
    // Split on last dash to handle multi-dash patterns
    let dash_pos = filename_stem
        .rfind('-')
        .ok_or_else(|| ParseError::InvalidFormat("Filename must be KIND-ID format".into()))?;

    let kind_str = &filename_stem[..dash_pos];
    let id_str = &filename_stem[dash_pos + 1..];

    let kind = NonEmptyString::from_str(kind_str).map_err(|_| ParseError::InvalidKind)?;
    let id = id_str
        .parse::<usize>()
        .map_err(|_| ParseError::InvalidId(id_str.into()))?;

    // All components are namespace
    let namespace: Result<Vec<_>, _> = components
        .iter()
        .map(|s| NonEmptyString::from_str(s))
        .collect();
    let namespace = namespace.map_err(|_| ParseError::InvalidNamespace)?;

    Ok(Hrid::new_with_namespace_unchecked(namespace, kind, id))
}

/// Errors that can occur during path parsing
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid file path")]
    InvalidPath,

    #[error("Invalid HRID format: {0}")]
    InvalidFormat(String),

    #[error("Missing KIND in path")]
    MissingKind,

    #[error("Invalid KIND")]
    InvalidKind,

    #[error("Invalid namespace segment")]
    InvalidNamespace,

    #[error("Invalid ID: {0}")]
    InvalidId(String),

    #[error("HRID parsing error: {0}")]
    Hrid(#[from] HridError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn filename_based_flat() {
        let root = PathBuf::from("/root");
        let path = root.join("REQ-001.md");

        let hrid = parse_hrid_from_path(&path, &root, false).unwrap();
        assert_eq!(hrid.to_string(), "REQ-001");
    }

    #[test]
    fn filename_based_with_namespace() {
        let root = PathBuf::from("/root");
        let path = root.join("system-auth-REQ-001.md");

        let hrid = parse_hrid_from_path(&path, &root, false).unwrap();
        assert_eq!(hrid.to_string(), "system-auth-REQ-001");
    }

    #[test]
    fn filename_based_in_subdirectory() {
        let root = PathBuf::from("/root");
        let path = root.join("custom/folder/system-auth-REQ-001.md");

        let hrid = parse_hrid_from_path(&path, &root, false).unwrap();
        assert_eq!(hrid.to_string(), "system-auth-REQ-001");
    }

    #[test]
    fn path_based_kind_in_filename() {
        let root = PathBuf::from("/root");
        let path = root.join("system/auth/REQ-001.md");

        let hrid = parse_hrid_from_path(&path, &root, true).unwrap();
        assert_eq!(hrid.to_string(), "system-auth-REQ-001");
        assert_eq!(hrid.namespace(), vec!["system", "auth"]);
        assert_eq!(hrid.kind(), "REQ");
        assert_eq!(hrid.id(), 1);
    }

    #[test]
    fn path_based_kind_in_parent() {
        let root = PathBuf::from("/root");
        let path = root.join("system/auth/REQ/001.md");

        let hrid = parse_hrid_from_path(&path, &root, true).unwrap();
        assert_eq!(hrid.to_string(), "system-auth-REQ-001");
        assert_eq!(hrid.namespace(), vec!["system", "auth"]);
        assert_eq!(hrid.kind(), "REQ");
        assert_eq!(hrid.id(), 1);
    }

    #[test]
    fn path_based_no_namespace_kind_in_filename() {
        let root = PathBuf::from("/root");
        let path = root.join("REQ-001.md");

        let hrid = parse_hrid_from_path(&path, &root, true).unwrap();
        assert_eq!(hrid.to_string(), "REQ-001");
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "REQ");
        assert_eq!(hrid.id(), 1);
    }

    #[test]
    fn path_based_no_namespace_kind_in_parent() {
        let root = PathBuf::from("/root");
        let path = root.join("REQ/001.md");

        let hrid = parse_hrid_from_path(&path, &root, true).unwrap();
        assert_eq!(hrid.to_string(), "REQ-001");
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "REQ");
        assert_eq!(hrid.id(), 1);
    }

    #[test]
    fn construct_path_filename_based() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, false, 3);
        assert_eq!(path, root.join("system-auth-REQ-001.md"));
    }

    #[test]
    fn construct_path_path_based() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("system-auth-REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 3);
        assert_eq!(path, root.join("system/auth/REQ-001.md"));
    }

    #[test]
    fn construct_path_path_based_no_namespace() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 3);
        assert_eq!(path, root.join("REQ-001.md"));
    }

    #[test]
    fn construct_path_with_custom_digits() {
        let root = PathBuf::from("/root");
        let hrid = Hrid::try_from("REQ-001").unwrap();

        let path = construct_path_from_hrid(&root, &hrid, true, 5);
        assert_eq!(path, root.join("REQ-00001.md"));
    }

    #[test]
    fn path_based_invalid_format() {
        let root = PathBuf::from("/root");
        let path = root.join("system/auth/INVALIDFORMAT.md");

        let result = parse_hrid_from_path(&path, &root, true);
        assert!(matches!(result, Err(ParseError::InvalidFormat(_))));
    }

    #[test]
    fn path_based_missing_kind() {
        let root = PathBuf::from("/root");
        let path = root.join("001.md");

        let result = parse_hrid_from_path(&path, &root, true);
        assert!(matches!(result, Err(ParseError::MissingKind)));
    }
}
