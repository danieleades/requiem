use std::path::Path;

use requiem::Directory;
use tracing::instrument;

use crate::cli::terminal::Colorize;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The kinds to remove
    #[arg(num_args = 1..)]
    kinds: Vec<String>,

    /// Skip confirmation prompt
    #[arg(long, short)]
    yes: bool,
}

impl Command {
    #[instrument]
    pub fn run(self, config_path: &Path, root: &Path) -> anyhow::Result<()> {
        // Load config
        let mut config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            anyhow::bail!(
                "Repository not initialized. Run 'req init' first or ensure you're in a \
                 requirements repository"
            );
        };

        // Load directory to check for existing requirements
        let directory = Directory::new(root.to_path_buf())?;

        // Check if requirements exist for these kinds
        let mut warnings = Vec::new();
        for kind in &self.kinds {
            let kind_upper = kind.to_uppercase();
            let count = directory
                .requirements()
                .filter(|req| req.hrid.kind() == kind_upper)
                .count();

            if count > 0 {
                warnings.push(format!("  • {kind_upper}: {count} requirements exist"));
            }
        }

        // Show warnings if requirements exist
        if !warnings.is_empty() && !self.yes {
            use std::io::{self, BufRead};

            println!(
                "{}",
                "⚠️  The following kinds have existing requirements:".warning()
            );
            for warning in &warnings {
                println!("{warning}");
            }
            println!(
                "\n{}",
                "Removing these kinds will NOT delete the requirements, but they will be \
                 considered invalid."
                    .dim()
            );

            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Remove kinds
        let mut removed = Vec::new();
        let mut not_found = Vec::new();

        for kind in self.kinds {
            let kind_upper = kind.to_uppercase();
            if config.remove_kind(&kind_upper) {
                removed.push(kind_upper);
            } else {
                not_found.push(kind_upper);
            }
        }

        // Save config if any kinds were removed
        if !removed.is_empty() {
            config
                .save(config_path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!(
                "{}",
                format!(
                    "✅ Removed {} kind(s): {}",
                    removed.len(),
                    removed.join(", ")
                )
                .success()
            );
        }

        if !not_found.is_empty() {
            println!(
                "{}",
                format!("ℹ️  Not found: {}", not_found.join(", ")).dim()
            );
        }

        Ok(())
    }
}
