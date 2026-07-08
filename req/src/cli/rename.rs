//! The `req rename` command: change a requirement's HRID.

use std::path::PathBuf;

use requiem_core::{Directory, Hrid};
use tracing::instrument;

use crate::cli::{parse_hrid, terminal::Colorize};

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The current HRID of the requirement to rename
    #[clap(value_parser = parse_hrid)]
    old_hrid: Hrid,

    /// The new HRID for the requirement
    #[clap(value_parser = parse_hrid)]
    new_hrid: Hrid,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Command {
    #[instrument]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.old_hrid) else {
            anyhow::bail!("Requirement {} not found", self.old_hrid.display(digits));
        };

        // Check if children exist
        let children = directory.children_of(&self.old_hrid);

        // Show confirmation if there are children or --yes not specified
        if !self.yes {
            println!(
                "Renaming {} → {}",
                self.old_hrid.display(digits),
                self.new_hrid.display(digits)
            );
            println!("  Title: {}", req.title);

            if !children.is_empty() {
                println!(
                    "\n{} will be updated in {} children:",
                    "Parent HRID".dim(),
                    children.len()
                );
                for child_hrid in &children {
                    println!("  • {}", child_hrid.display(digits));
                }
            }

            super::prompt_to_proceed()?;
        }

        // Perform rename
        let children_updated = directory.rename_requirement(&self.old_hrid, &self.new_hrid)?;
        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Renamed {} → {}",
                self.old_hrid.display(digits),
                self.new_hrid.display(digits)
            )
            .success()
        );

        if !children_updated.is_empty() {
            println!(
                "{}",
                format!("   Updated {} children", children_updated.len()).dim()
            );
        }

        Ok(())
    }
}
