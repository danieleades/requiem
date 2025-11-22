use std::path::Path;

use tracing::instrument;

use crate::cli::terminal::Colorize;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The kinds to add (e.g., USR, SYS, TST)
    #[arg(num_args = 1..)]
    kinds: Vec<String>,
}

impl Command {
    #[instrument]
    pub fn run(self, config_path: &Path) -> anyhow::Result<()> {
        // Load config
        let mut config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            anyhow::bail!(
                "Repository not initialized. Run 'req init' first or ensure you're in a \
                 requirements repository"
            );
        };

        // Validate and add kinds
        let mut added = Vec::new();
        let mut already_exists = Vec::new();

        for kind in self.kinds {
            // Validate kind format (must be uppercase alphabetic)
            let kind_upper = kind.to_uppercase();
            if !kind_upper.chars().all(|c| c.is_ascii_uppercase()) {
                anyhow::bail!("Invalid kind '{kind}': kinds must contain only letters (A-Z)");
            }

            if config.add_kind(&kind_upper) {
                added.push(kind_upper);
            } else {
                already_exists.push(kind_upper);
            }
        }

        // Save config if any kinds were added
        if !added.is_empty() {
            config
                .save(config_path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!(
                "{}",
                format!("✅ Added {} kind(s): {}", added.len(), added.join(", ")).success()
            );
        }

        if !already_exists.is_empty() {
            println!(
                "{}",
                format!("ℹ️  Already registered: {}", already_exists.join(", ")).dim()
            );
        }

        Ok(())
    }
}
