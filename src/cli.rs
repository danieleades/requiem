use std::{
    io::{self, BufRead},
    path::{Path, PathBuf},
};

mod config;
mod create;
mod delete;
mod init;
mod kind;
mod link;
mod list;
mod review;
mod show;
mod status;
mod sync;
mod terminal;
mod unlink;
mod validate;

use clap::ArgAction;
use list::List;
use requiem::{Directory, Hrid};
use show::Show;
use tracing::instrument;
use validate::Validate;

/// Parse an HRID from a string, normalizing to uppercase.
///
/// This is a CLI boundary function that accepts lowercase input
/// and normalizes it before parsing.
fn parse_hrid(s: &str) -> Result<Hrid, String> {
    // Normalize to uppercase
    let uppercase = s.to_uppercase();
    // Parse using FromStr (strict validation)
    uppercase.parse().map_err(|e| format!("{e}"))
}

#[derive(Debug, clap::Parser)]
#[command(version, about)]
pub struct Cli {
    /// Verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = ArgAction::Count, global=true)]
    verbose: u8,

    /// The path to the root of the requirements directory
    #[arg(short, long, default_value = ".", global = true)]
    root: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        Self::setup_logging(self.verbose);

        self.command.unwrap_or_default().run(self.root)
    }

    fn setup_logging(verbosity: u8) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let level = match verbosity {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        };

        let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive(level.into());

        let fmt_layer = tracing_subscriber::fmt::layer()
            //.pretty()
            .with_target(false)
            .with_thread_names(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// Show repository status (default)
    Status(status::Command),

    /// Initialize a new requirements repository
    Init(init::Command),

    /// Create a new requirement
    Create(create::Command),

    /// Delete a requirement
    Delete(delete::Command),

    /// Create a link between two requirements
    ///
    /// Links are parent-child relationships.
    Link(link::Command),

    /// Remove a link between two requirements
    Unlink(unlink::Command),

    /// Synchronize parent HRIDs and file paths
    Sync(sync::Command),

    /// Review suspect links
    ///
    /// Suspect links are those where the parent requirement has changed
    /// since the link was created or last reviewed.
    Review(review::Command),

    /// Validate repository health
    Validate(Validate),

    /// Show detailed information about a requirement
    Show(Show),

    /// List requirements with filters and relationship views
    List(List),

    /// Show or modify configuration settings
    Config(config::Command),

    /// Manage requirement kinds
    Kind(kind::Command),

    /// Rename a requirement's HRID
    Rename(Rename),

    /// Move a requirement to a new file path
    Move(Move),

    /// Diagnose path-related issues
    Diagnose(Diagnose),
}

impl Default for Command {
    fn default() -> Self {
        Self::Status(status::Command::default())
    }
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Status(command) => command.run(root)?,
            Self::Init(command) => command.run(&root)?,
            Self::Create(command) => command.run(root)?,
            Self::Delete(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Unlink(command) => command.run(root)?,
            Self::Sync(command) => command.run(root)?,
            Self::Review(command) => command.run(root)?,
            Self::Validate(command) => command.run(root)?,
            Self::Show(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
            Self::Config(command) => command.run(&root)?,
            Self::Kind(command) => command.run(&root)?,
            Self::Rename(command) => command.run(root)?,
            Self::Move(command) => command.run(&root)?,
            Self::Diagnose(command) => command.run(&root)?,
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Rename {
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

impl Rename {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

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

            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
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

#[derive(Debug, clap::Parser)]
pub struct Move {
    /// The HRID of the requirement to move
    #[clap(value_parser = parse_hrid)]
    hrid: Hrid,

    /// The new file path (relative to repository root)
    new_path: PathBuf,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Move {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        use terminal::Colorize;

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

#[derive(Debug, clap::Parser)]
pub struct Diagnose {
    #[command(subcommand)]
    command: DiagnoseCommand,
}

#[derive(Debug, clap::Parser)]
enum DiagnoseCommand {
    /// Diagnose path-related issues
    Paths,
}

impl Diagnose {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        use terminal::Colorize;

        match self.command {
            DiagnoseCommand::Paths => {
                let directory = Directory::new(root.to_path_buf())?;
                let digits = directory.config().digits();
                let mut issues: Vec<String> = Vec::new();

                for req in directory.requirements() {
                    // Get the actual path where this requirement was loaded from
                    let Some(actual_path) = directory.path_for(req.hrid) else {
                        continue; // Skip if path not found (shouldn't happen)
                    };

                    // Get the expected canonical path based on config
                    let expected_path = directory.canonical_path_for(req.hrid);

                    if actual_path != expected_path {
                        let hrid = req.hrid;
                        let expected_display = expected_path
                            .strip_prefix(root)
                            .unwrap_or(&expected_path)
                            .display();
                        let actual_display = actual_path
                            .strip_prefix(root)
                            .unwrap_or(actual_path)
                            .display();
                        issues.push(format!(
                            "{}: Expected '{expected_display}', found '{actual_display}'",
                            hrid.display(digits)
                        ));
                    }
                }

                if issues.is_empty() {
                    println!("{}", "✅ No path issues detected.".success());
                } else {
                    let issue_count = issues.len();
                    println!(
                        "{}",
                        format!("⚠️  {issue_count} path issues found:").warning()
                    );
                    println!();
                    for (i, issue) in issues.iter().enumerate() {
                        println!("{}. {}", i + 1, issue);
                    }
                    println!(
                        "\n{}",
                        "Review the issues above and fix them manually.".dim()
                    );
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use requiem::Directory;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn status_run_reports_counts_without_exit() {
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

        // Create a parent-child relationship to ensure we exercise counting logic.
        let mut directory = Directory::new(root.clone()).unwrap();
        directory
            .link_requirement(child.hrid(), parent.hrid())
            .unwrap();
        directory.flush().unwrap();

        Command::default()
            .run(root)
            .expect("status should succeed when no suspect links exist");
    }
}
