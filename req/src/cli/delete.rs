use std::path::PathBuf;

use requiem_core::{Directory, Hrid};
use tracing::instrument;

use crate::cli::{parse_hrid, prompt_to_proceed, terminal::Colorize};

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Command {
    /// The human-readable ID of the requirement to delete
    #[clap(value_parser = parse_hrid)]
    hrid: Hrid,

    /// Delete requirement and orphaned descendants (children with no other
    /// parents)
    #[arg(long)]
    cascade: bool,

    /// Delete requirement and unlink from children (children remain)
    #[arg(long, conflicts_with = "cascade")]
    orphan: bool,

    /// Show what would be deleted without deleting
    #[arg(long)]
    dry_run: bool,

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
        let Some(req) = directory.find_by_hrid(&self.hrid) else {
            anyhow::bail!("Requirement {} not found", self.hrid.display(digits));
        };

        let hrid = req.hrid.clone();
        let children = directory.children_of(&hrid);

        // Check if requirement has children
        if !children.is_empty() && !self.cascade && !self.orphan {
            eprintln!(
                "{}",
                format!(
                    "⚠️  Cannot delete {}: requirement has {} children",
                    hrid.display(digits),
                    children.len()
                )
                .warning()
            );
            eprintln!("\nChildren:");
            for child in &children {
                eprintln!("  • {}", child.display(digits));
            }
            eprintln!(
                "\n{}",
                "Use --cascade to delete with orphaned descendants, or --orphan to unlink children"
                    .dim()
            );
            anyhow::bail!("Cannot delete requirement with children");
        }

        // Determine what will be deleted
        let to_delete = if self.cascade {
            // Smart cascade: find descendants that would become orphans
            directory.find_orphaned_descendants(&hrid)
        } else {
            vec![hrid.clone()]
        };

        // Show preview
        if !self.yes && !self.dry_run {
            println!("Will delete {} requirement(s):", to_delete.len());
            for delete_hrid in &to_delete {
                println!("  • {}", delete_hrid.display(digits));
            }

            if self.orphan && !children.is_empty() {
                println!("\nWill unlink from {} children:", children.len());
                for child in &children {
                    println!("  • {}", child.display(digits));
                }
            }

            // Get confirmation
            prompt_to_proceed()?;
        }

        if self.dry_run {
            println!(
                "{}",
                format!("Would delete {} requirement(s)", to_delete.len()).dim()
            );
            return Ok(());
        }

        // Perform deletion
        if self.orphan {
            directory.delete_and_orphan(&hrid)?;
        } else if self.cascade {
            for delete_hrid in &to_delete {
                directory.delete_requirement(delete_hrid)?;
            }
        } else {
            directory.delete_requirement(&hrid)?;
        }

        directory.flush()?;

        println!(
            "{}",
            format!("✅ Deleted {} requirement(s)", to_delete.len()).success()
        );
        Ok(())
    }
}
