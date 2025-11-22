use std::{fs, path::Path};

use tracing::instrument;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// Requirement kinds to create templates for
    #[arg(long, value_name = "KIND", num_args = 1..)]
    kinds: Vec<String>,
}

impl Command {
    #[instrument]
    pub fn run(self, root: &Path) -> anyhow::Result<()> {
        // Create .req directory
        let req_dir = root.join(".req");
        if req_dir.exists() {
            anyhow::bail!("Repository already initialized (found existing .req directory)");
        }

        fs::create_dir_all(&req_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create .req directory: {e}"))?;

        // Create config.toml with defaults (no kinds configured)
        let config_path = req_dir.join("config.toml");
        let config = requiem::Config::default();
        config
            .save(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to create config.toml: {e}"))?;

        // Create templates directory
        let templates_dir = req_dir.join("templates");
        fs::create_dir_all(&templates_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create templates directory: {e}"))?;

        println!("Initialized requirements repository in {}", root.display());
        println!("  Created: .req/config.toml");
        println!("  Created: .req/templates/ (empty)");

        // Create templates for specified kinds
        let mut created_templates = Vec::new();
        for kind in &self.kinds {
            let kind_upper = kind.to_uppercase();

            // Validate kind format
            if !kind_upper.chars().all(|c| c.is_ascii_uppercase()) {
                anyhow::bail!("Invalid kind '{kind}': kinds must contain only letters (A-Z)");
            }

            let template_path = templates_dir.join(format!("{kind_upper}.md"));

            fs::File::create(&template_path)
                .map_err(|e| anyhow::anyhow!("Failed to create {kind_upper} template: {e}"))?;

            created_templates.push(kind_upper);
        }

        if !created_templates.is_empty() {
            for kind in &created_templates {
                println!("  Created: .req/templates/{kind}.md");
            }
        }

        println!();
        println!("Next steps:");
        if created_templates.is_empty() {
            println!("  req kind add USR SYS  # Register requirement kinds");
            println!("  req create USR --title \"Your First Requirement\"");
        } else {
            println!(
                "  req create {} --title \"Your First Requirement\"",
                created_templates[0]
            );
        }

        Ok(())
    }
}
