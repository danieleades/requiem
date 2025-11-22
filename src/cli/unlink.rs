use std::path::PathBuf;

use requiem::{Directory, Hrid};
use tracing::instrument;

use crate::cli::{parse_hrid, prompt_to_proceed, terminal};

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The human-readable ID of the child document
    #[clap(value_parser = parse_hrid)]
    child: Hrid,

    /// The human-readable ID of the parent document to remove
    #[clap(value_parser = parse_hrid)]
    parent: Hrid,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Command {
    #[instrument]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Validate both requirements exist
        let Some(_child_req) = directory.find_by_hrid(&self.child) else {
            anyhow::bail!("Child requirement {} not found", self.child.display(digits));
        };

        let Some(_parent_req) = directory.find_by_hrid(&self.parent) else {
            anyhow::bail!(
                "Parent requirement {} not found",
                self.parent.display(digits)
            );
        };

        // Show confirmation prompt unless --yes was specified
        if !self.yes {
            println!(
                "Will unlink {} from parent {}",
                self.child.display(digits),
                self.parent.display(digits)
            );
            prompt_to_proceed()?;
        }

        // Perform the unlink
        directory.unlink_requirement(&self.child, &self.parent)?;
        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Unlinked {} from {}",
                self.child.display(digits),
                self.parent.display(digits)
            )
            .success()
        );

        Ok(())
    }
}
