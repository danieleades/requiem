use std::path::{Path, PathBuf};

use requiem::{Directory, Hrid};
use tracing::instrument;

use crate::cli::{parse_hrid, terminal::Colorize};

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The HRID of the requirement to move
    #[clap(value_parser = parse_hrid)]
    hrid: Hrid,

    /// The new file path (relative to repository root)
    new_path: PathBuf,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Command {
    #[instrument]
    pub fn run(self, root: &Path) -> anyhow::Result<()> {
        let mut directory = Directory::new(root.to_path_buf())?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.hrid) else {
            anyhow::bail!("Requirement {} not found", self.hrid.display(digits));
        };

        // Get current path
        let old_path = directory.path_for(&self.hrid).ok_or_else(|| {
            anyhow::anyhow!("Cannot find current path for {}", self.hrid.display(digits))
        })?;

        // Make new path absolute if relative
        let new_path = if self.new_path.is_absolute() {
            self.new_path.clone()
        } else {
            root.join(&self.new_path)
        };

        // Extract HRID from new path to see if it will change
        let new_hrid = requiem::hrid_from_path(&new_path, root, directory.config())
            .map_err(|e| anyhow::anyhow!("Failed to parse HRID from path: {e}"))?;

        // Check if children exist
        let children = directory.children_of(&self.hrid);

        // Show confirmation if --yes not specified
        if !self.yes {
            use std::io::{self, BufRead};

            println!(
                "Moving {} from {} to {}",
                self.hrid.display(digits),
                old_path.strip_prefix(root).unwrap_or(old_path).display(),
                self.new_path.display()
            );
            println!("  Title: {}", req.title);

            if new_hrid != self.hrid {
                println!(
                    "\n{} HRID will change: {} → {}",
                    "⚠️".warning(),
                    self.hrid.display(digits),
                    new_hrid.display(digits)
                );

                if !children.is_empty() {
                    println!(
                        "   {} will be updated in {} children",
                        "Parent HRID".dim(),
                        children.len()
                    );
                }
            }

            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Perform move
        let children_updated = directory.move_requirement(&self.hrid, new_path.clone())?;

        // Create parent directories if needed
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Moved {} to {}",
                self.hrid.display(digits),
                self.new_path.display()
            )
            .success()
        );

        if let Some(children) = children_updated {
            if !children.is_empty() {
                println!(
                    "{}",
                    format!(
                        "   Updated HRID {} → {} in {} children",
                        self.hrid.display(digits),
                        new_hrid.display(digits),
                        children.len()
                    )
                    .dim()
                );
            }
        }

        Ok(())
    }
}
