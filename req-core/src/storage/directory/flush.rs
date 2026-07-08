//! Persisting pending changes back to disk.

use std::{collections::HashSet, fmt, io, path::PathBuf};

use nonempty::NonEmpty;
use uuid::Uuid;

use super::Directory;
use crate::{domain::Hrid, Requirement};

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
            .map(|(p, e)| format!("{} ({e})", p.display()))
            .collect();

        let msg = displayed_paths.join(", ");

        if total <= MAX_DISPLAY {
            write!(f, "{msg}")
        } else {
            write!(f, "{msg}... (and {} more)", total - MAX_DISPLAY)
        }
    }
}

impl Directory {
    /// Persist all dirty requirements to disk.
    ///
    /// Returns the HRIDs of the requirements that were written.
    ///
    /// # Errors
    ///
    /// Returns an error containing the paths that failed to flush alongside the
    /// underlying IO error.
    pub fn flush(&mut self) -> Result<Vec<Hrid>, FlushError> {
        let digits = self.config.digits();
        let mut failures: Vec<(PathBuf, io::Error)> = Vec::new();
        let mut flushed = Vec::new();

        // Phase 1: resolve write targets. Computed fallback paths are recorded
        // in `self.paths` so a later rename can find (and queue for deletion)
        // the file this flush is about to create.
        let dirty: Vec<_> = self.dirty.iter().copied().collect();
        let mut writes: Vec<(Uuid, Requirement, PathBuf)> = Vec::new();
        for uuid in dirty {
            let Some(requirement) = self.tree.get_requirement(uuid) else {
                // Requirement was removed since being marked dirty.
                self.dirty.remove(&uuid);
                continue;
            };
            let canonical = self.canonical_path_for(requirement.hrid());
            let path = self.paths.entry(uuid).or_insert(canonical).clone();
            writes.push((uuid, requirement, path));
        }
        // Deterministic write order (and hence deterministic flushed output).
        writes.sort_by(|a, b| a.2.cmp(&b.2));

        // Phase 2: perform all writes (each atomic). A failure must not abort
        // the flush: skipping the reconciliation phase after a rename would
        // leave two files with the same UUID and wedge the next load.
        for (uuid, requirement, path) in &writes {
            match requirement.save_to_path(path, digits) {
                Ok(()) => {
                    self.dirty.remove(uuid);
                    flushed.push(requirement.hrid().clone());
                }
                Err(err) => {
                    failures.push((path.clone(), err));
                }
            }
        }

        // Phase 3: reconcile and process deletions. A queued deletion is
        // dropped (not executed) when its path is now the live location of
        // some requirement (e.g. another requirement was renamed onto the
        // deleted HRID, or a move landed back on the same path).
        let live_paths: HashSet<&PathBuf> = self.paths.values().collect();
        if failures.is_empty() {
            let mut deferred: HashSet<PathBuf> = HashSet::new();
            for path in self.deletions.drain() {
                if live_paths.contains(&path) {
                    continue;
                }
                match std::fs::remove_file(&path) {
                    Ok(()) => {}
                    // Already gone: the deletion's goal is achieved.
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => {
                        failures.push((path.clone(), e));
                        deferred.insert(path);
                    }
                }
            }
            self.deletions = deferred;
        } else {
            // Never remove files while the store is partially written: e.g.
            // deleting an orphaned parent's file while a child's rewrite
            // failed would strand on-disk references to a missing
            // requirement. Stale deletions are dropped; the rest stay queued
            // so a retried flush converges.
            self.deletions.retain(|path| !live_paths.contains(path));
        }

        if let Some(failures) = NonEmpty::from_vec(failures) {
            return Err(FlushError { failures });
        }
        Ok(flushed)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{super::setup_temp_directory, *};

    #[test]
    fn rename_after_flush_removes_old_file() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir.add_requirement("REQ", String::new()).unwrap();
        dir.flush().unwrap();

        let old_path = dir.root.join("REQ-001.md");
        assert!(old_path.exists());

        let new_hrid = Hrid::from_str("SYS-001").unwrap();
        dir.rename_requirement(req.hrid(), &new_hrid).unwrap();
        dir.flush().unwrap();

        assert!(!old_path.exists(), "stale file must be deleted on flush");
        assert!(dir.root.join("SYS-001.md").exists());

        // The store must reload cleanly (no duplicate UUIDs on disk).
        let reloaded = Directory::new(dir.root.clone()).unwrap();
        assert!(reloaded.find_by_hrid(&new_hrid).is_some());
    }

    #[test]
    fn rename_before_first_flush_leaves_single_file() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir.add_requirement("REQ", String::new()).unwrap();

        // Rename before the original file was ever written.
        let new_hrid = Hrid::from_str("SYS-001").unwrap();
        dir.rename_requirement(req.hrid(), &new_hrid).unwrap();
        dir.flush().unwrap();

        assert!(!dir.root.join("REQ-001.md").exists());
        assert!(dir.root.join("SYS-001.md").exists());
        assert!(Directory::new(dir.root.clone()).is_ok());
    }

    #[test]
    fn rename_onto_deleted_hrid_keeps_new_file() {
        let (_tmp, mut dir) = setup_temp_directory();
        let doomed = dir.add_requirement("REQ", String::new()).unwrap();
        let survivor = dir.add_requirement("REQ", String::new()).unwrap();
        dir.flush().unwrap();

        // Delete REQ-001, then rename REQ-002 onto the freed HRID, so the
        // deleted requirement's path is also the survivor's write target.
        dir.delete_requirement(doomed.hrid()).unwrap();
        dir.rename_requirement(survivor.hrid(), doomed.hrid())
            .unwrap();
        dir.flush().unwrap();

        assert!(dir.root.join("REQ-001.md").exists());
        assert!(!dir.root.join("REQ-002.md").exists());

        let reloaded = Directory::new(dir.root.clone()).unwrap();
        let view = reloaded.find_by_hrid(doomed.hrid()).unwrap();
        assert_eq!(*view.uuid, survivor.uuid());
    }

    #[test]
    fn move_to_same_path_does_not_delete_file() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir.add_requirement("REQ", String::new()).unwrap();
        dir.flush().unwrap();

        let path = dir.path_for(req.hrid()).unwrap().to_path_buf();
        dir.move_requirement(req.hrid(), path.clone()).unwrap();
        dir.flush().unwrap();

        assert!(path.exists(), "moving onto itself must not delete the file");
    }

    #[cfg(unix)]
    #[test]
    fn failed_write_defers_deletion_and_retries() {
        let (_tmp, mut dir) = setup_temp_directory();
        let req = dir.add_requirement("REQ", String::new()).unwrap();
        dir.flush().unwrap();

        // Block the rename target with a directory so the write fails.
        let target = dir.root.join("SYS-001.md");
        std::fs::create_dir(&target).unwrap();

        let new_hrid = Hrid::from_str("SYS-001").unwrap();
        dir.rename_requirement(req.hrid(), &new_hrid).unwrap();
        assert!(dir.flush().is_err());
        assert!(
            dir.root.join("REQ-001.md").exists(),
            "the only good copy must not be deleted when its replacement failed to write"
        );

        // Unblock and retry: the flush heals itself.
        std::fs::remove_dir(&target).unwrap();
        dir.flush().unwrap();
        assert!(!dir.root.join("REQ-001.md").exists());
        assert!(target.exists());
        assert!(Directory::new(dir.root.clone()).is_ok());
    }

    #[test]
    fn orphan_delete_defers_file_removal_when_child_write_fails() {
        let (_tmp, mut dir) = setup_temp_directory();
        let parent = dir.add_requirement("SYS", String::new()).unwrap();
        let child = dir.add_requirement("REQ", String::new()).unwrap();
        dir.link_requirement(child.hrid(), parent.hrid()).unwrap();
        dir.flush().unwrap();

        // Make the child's rewrite fail by replacing its file with a
        // directory.
        let child_path = dir.root.join("REQ-001.md");
        std::fs::remove_file(&child_path).unwrap();
        std::fs::create_dir(&child_path).unwrap();

        dir.delete_and_orphan(parent.hrid()).unwrap();
        assert!(dir.flush().is_err());
        assert!(
            dir.root.join("SYS-001.md").exists(),
            "the parent file must not be removed while an orphaned child could not be rewritten"
        );

        // Unblock and retry: the flush converges.
        std::fs::remove_dir(&child_path).unwrap();
        dir.flush().unwrap();
        assert!(!dir.root.join("SYS-001.md").exists());
        assert!(child_path.exists());
        assert!(Directory::new(dir.root.clone()).is_ok());
    }
}
