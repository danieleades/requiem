use std::path::Path;

use tracing::instrument;

use crate::cli::terminal::Colorize;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The kinds to add (e.g., USR, SYS, TST)
    #[arg(num_args = 1..)]
    kinds: Vec<String>,

    /// Optional description to attach to these kinds
    #[arg(short, long)]
    description: Option<String>,
}

impl Command {
    #[instrument]
    pub fn run(self, config_path: &Path) -> anyhow::Result<()> {
        // Load config
        let mut config = if config_path.exists() {
            requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            anyhow::bail!(
                "Repository not initialized. Run 'req init' first or ensure you're in a \
                 requirements repository"
            );
        };

        // Validate and add kinds
        let mut added = Vec::new();
        let mut already_exists = Vec::new();
        let mut touched = Vec::new();

        for kind in self.kinds {
            // Validate kind format (must be uppercase alphabetic)
            let kind_upper = kind.to_uppercase();
            if !kind_upper.chars().all(|c| c.is_ascii_uppercase()) {
                anyhow::bail!("Invalid kind '{kind}': kinds must contain only letters (A-Z)");
            }

            touched.push(kind_upper.clone());
            if config.add_kind(&kind_upper) {
                added.push(kind_upper);
            } else {
                already_exists.push(kind_upper);
            }
        }

        if let Some(description) = self.description.clone() {
            for kind in &touched {
                config.set_kind_description(kind, Some(description.clone()));
            }
        }

        // Save config if any kinds were added
        if !added.is_empty() || self.description.is_some() {
            config
                .save(config_path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            if !added.is_empty() {
                println!(
                    "{}",
                    format!("✅ Added {} kind(s): {}", added.len(), added.join(", ")).success()
                );
            }
            if let Some(description) = &self.description {
                println!(
                    "{}",
                    format!(
                        "ℹ️  Description set for: {} ({description})",
                        touched.join(", ")
                    )
                    .dim()
                );
            }
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
