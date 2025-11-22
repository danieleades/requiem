use std::path::PathBuf;

use requiem::{Directory, Hrid};
use tracing::instrument;

use crate::cli::parse_hrid;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The human-readable ID of the child document
    #[clap(value_parser = parse_hrid)]
    child: Hrid,

    /// The human-readable ID of the parent document
    #[clap(value_parser = parse_hrid)]
    parent: Hrid,
}

impl Command {
    #[instrument]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();
        let child = &self.child;
        let parent = &self.parent;
        let msg = format!(
            "Linked {} to {}",
            child.display(digits),
            parent.display(digits)
        );

        directory.link_requirement(&self.child, &self.parent)?;
        directory.flush()?;

        println!("{msg}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use requiem::{Directory, RequirementView};
    use tempfile::tempdir;

    use super::Command;

    fn collect_child<'a>(directory: &'a Directory, kind: &'a str) -> RequirementView<'a> {
        directory
            .requirements()
            .find(|req| req.hrid.kind() == kind)
            .expect("expected requirement for kind")
    }

    #[test]
    fn link_run_updates_child_parent_relationship() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR", "# Child".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush initial requirements");

        let link = Command {
            child: child.hrid().clone(),
            parent: parent.hrid().clone(),
        };

        link.run(root.clone()).expect("link command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let reloaded_child = collect_child(&directory, "USR");
        assert!(reloaded_child
            .parents
            .iter()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
    }
}
