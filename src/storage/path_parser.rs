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

use std::path::{Path, PathBuf};

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
    } else {
        // Filename-based: root/FULL-HRID.md
        root.join(hrid.to_string()).with_extension("md")
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
        assert_eq!(path, root.join("SYSTEM/AUTH/REQ-001.md"));
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
}
