//! A filesystem backed store of requirements
//!
//! The [`Directory`] provides a way to manage requirements stored in a
//! directory structure. It is a wrapper around the filesystem agnostic
//! [`Tree`].
//!
//! The core store and its read-only queries live here; related behaviour is
//! split into focused submodules:
//!
//! - `load`: opening a directory and loading requirements from disk
//! - `edit`: adding, linking, renaming, moving, and deleting requirements
//! - `maintenance`: drift detection, suspect-link review, and repair
//! - `flush`: persisting pending changes back to disk

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::domain::{Config, Hrid, RequirementView, Tree};

mod edit;
mod flush;
mod load;
mod maintenance;

pub use edit::AddRequirementError;
pub use flush::FlushError;
pub use load::DirectoryLoadError;
pub use maintenance::{AcceptResult, AcceptSuspectLinkError};

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
    pub fn requirement_by_hrid(&self, hrid: &Hrid) -> Option<crate::Requirement> {
        self.tree
            .find_by_hrid(hrid)
            .map(|view| view.to_requirement())
    }

    /// Find a requirement by its HRID, returning a view.
    #[must_use]
    pub fn find_by_hrid(&self, hrid: &Hrid) -> Option<RequirementView<'_>> {
        self.tree.find_by_hrid(hrid)
    }

    /// Find a requirement by its UUID.
    #[must_use]
    pub fn find_by_uuid(&self, uuid: Uuid) -> Option<RequirementView<'_>> {
        self.tree.requirement(uuid)
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

    /// Get all ancestors (transitive parents) of a requirement by HRID.
    ///
    /// The result is deduplicated and sorted.
    #[must_use]
    pub fn ancestors_of(&self, hrid: &Hrid) -> Vec<Hrid> {
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            return vec![];
        };

        let mut collected: BTreeSet<Hrid> = BTreeSet::new();
        for uuid in self.tree.ancestors_of(*view.uuid) {
            if let Some(hrid) = self.tree.hrid(uuid) {
                collected.insert(hrid.clone());
            }
        }

        collected.into_iter().collect()
    }

    /// Get all descendants (transitive children) of a requirement by HRID.
    ///
    /// The result is deduplicated and sorted.
    #[must_use]
    pub fn descendants_of(&self, hrid: &Hrid) -> Vec<Hrid> {
        let Some(view) = self.tree.find_by_hrid(hrid) else {
            return vec![];
        };

        let mut collected: BTreeSet<Hrid> = BTreeSet::new();
        for uuid in self.tree.descendants_of(*view.uuid) {
            if let Some(hrid) = self.tree.hrid(uuid) {
                collected.insert(hrid.clone());
            }
        }

        collected.into_iter().collect()
    }
}

/// Create a temporary directory backed store for tests.
#[cfg(test)]
pub(crate) fn setup_temp_directory() -> (tempfile::TempDir, Directory) {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let path = tmp.path().to_path_buf();
    (tmp, Directory::new(path).unwrap())
}
