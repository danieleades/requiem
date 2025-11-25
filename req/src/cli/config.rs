// TODO: this approach is brittle and difficult to maintain. Consider using a
// more robust approach, potentially a proc macro which generates
// setters/getters and enumerates options from the actual Config struct.

use std::path::Path;

use tracing::instrument;

use crate::cli::terminal::Colorize;

#[derive(Debug, clap::Parser)]
/// Show or modify repository configuration
///
/// Configuration is stored in .req/config.toml and controls repository
/// behavior.
///
/// Available configuration keys:
///   `subfolders_are_namespaces`  Path mode (true) vs filename mode (false)
///   digits                      Number of digits for HRID padding (default: 3)
///   `allow_unrecognised`         Allow non-HRID markdown files (default:
/// false)
///
/// Note: Use 'req kind' commands to manage `allowed_kinds` configuration.
pub struct Command {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, clap::Parser)]
enum ConfigCommand {
    /// Show all configuration values
    Show,

    /// Get a specific configuration value
    Get {
        /// Configuration key to retrieve
        ///
        /// Available keys: `subfolders_are_namespaces`, digits,
        /// `allow_unrecognised`, `allowed_kinds`
        key: String,
    },

    /// Set a configuration value
    ///
    /// Examples:
    ///   req config set `subfolders_are_namespaces` true
    ///   req config set `allow_unrecognised` false
    Set {
        /// Configuration key to set
        ///
        /// Settable keys: `subfolders_are_namespaces`, `allow_unrecognised`
        key: String,

        /// Value to set
        value: String,
    },
}

impl Command {
    #[instrument]
    pub fn run(self, root: &Path) -> anyhow::Result<()> {
        let config_path = root.join(".req/config.toml");

        match self.command {
            ConfigCommand::Show => Self::show_config(&config_path),
            ConfigCommand::Get { key } => Self::get_config(&config_path, &key),
            ConfigCommand::Set { key, value } => Self::set_config(&config_path, &key, &value),
        }
    }

    fn show_config(config_path: &std::path::Path) -> anyhow::Result<()> {
        let config = if config_path.exists() {
            requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem_core::Config::default()
        };

        println!("Configuration:");
        println!(
            "  subfolders_are_namespaces: {} ({})",
            config.subfolders_are_namespaces,
            if config.subfolders_are_namespaces {
                "path mode".dim()
            } else {
                "filename mode".dim()
            }
        );
        println!("  digits: {}", config.digits());
        println!("  allow_unrecognised: {}", config.allow_unrecognised);
        if config.allowed_kinds().is_empty() {
            println!("  allowed_kinds: {} (all kinds allowed)", "[]".dim());
        } else {
            println!("  allowed_kinds: {:?}", config.allowed_kinds());
        }
        Ok(())
    }

    fn get_config(config_path: &std::path::Path, key: &str) -> anyhow::Result<()> {
        let config = if config_path.exists() {
            requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem_core::Config::default()
        };

        match key {
            "subfolders_are_namespaces" => {
                println!("{}", config.subfolders_are_namespaces);
            }
            "digits" => {
                println!("{}", config.digits());
            }
            "allow_unrecognised" => {
                println!("{}", config.allow_unrecognised);
            }
            "allowed_kinds" => {
                if config.allowed_kinds().is_empty() {
                    println!("[]");
                } else {
                    for kind in config.allowed_kinds() {
                        println!("{kind}");
                    }
                }
            }
            _ => {
                anyhow::bail!(
                    "Unknown configuration key: '{key}'\n\nAvailable keys:\n  \
                     subfolders_are_namespaces\n  digits\n  allow_unrecognised\n  allowed_kinds",
                );
            }
        }
        Ok(())
    }

    fn set_config(config_path: &std::path::Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut config = if config_path.exists() {
            requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem_core::Config::default()
        };

        match key {
            "subfolders_are_namespaces" => {
                let bool_value = value
                    .parse::<bool>()
                    .map_err(|_| anyhow::anyhow!("Value must be 'true' or 'false'"))?;

                config.set_subfolders_are_namespaces(bool_value);
                config
                    .save(config_path)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                println!(
                    "{}",
                    format!(
                        "Directory mode: {}",
                        if bool_value {
                            "path-based"
                        } else {
                            "filename-based"
                        }
                    )
                    .success()
                );

                if bool_value {
                    println!("\n{}", "Path-based mode:".info());
                    println!(
                        "  • Filenames inside namespace folders should contain KIND-ID (e.g., \
                         USR/003.md)."
                    );
                    println!(
                        "  • You will need to manually reorganize existing files to match the new \
                         structure."
                    );
                } else {
                    println!("\n{}", "Filename-based mode:".info());
                    println!("  • Namespaces will no longer be inferred from folders.");
                    println!("  • Full HRID must be in filename (e.g., system-auth-USR-003.md).");
                }

                println!(
                    "\n{}",
                    "See docs/src/requirements/SPC-004.md for migration guide"
                        .to_string()
                        .dim()
                );
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown configuration key: '{key}'\nSupported keys: subfolders_are_namespaces",
                ));
            }
        }
        Ok(())
    }
}
