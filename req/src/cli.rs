use std::{io::BufRead, path::PathBuf};

mod config;
mod create;
mod delete;
mod diagnose;
mod init;
mod kind;
mod link;
mod list;
mod r#move;
mod rename;
mod review;
mod show;
mod status;
mod sync;
mod terminal;
mod unlink;
mod validate;

use borsh::io;
use clap::ArgAction;
use list::List;
use requiem_core::Hrid;
use show::Show;
use validate::Validate;

/// Parse an HRID from a string, normalizing only the KIND segment to uppercase.
///
/// This is a CLI boundary function that accepts lowercase namespaces
/// but normalizes the kind (category) to uppercase for user convenience.
/// For example: `auth-sys-001` → `auth-SYS-001`
fn parse_hrid(s: &str) -> Result<Hrid, String> {
    // Split on '-' to normalize only the KIND segment (second-to-last position)
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 2 {
        return Err("Invalid HRID format".to_string());
    }

    // Uppercase the kind segment (second-to-last position)
    let kind_idx = parts.len() - 2;
    let normalized = parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if i == kind_idx {
                part.to_uppercase()
            } else {
                (*part).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("-");

    normalized.parse().map_err(|e| format!("{e}"))
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
    Rename(rename::Command),

    /// Move a requirement to a new file path
    Move(r#move::Command),

    /// Diagnose path-related issues
    Diagnose(diagnose::Command),
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

fn prompt_to_proceed() -> io::Result<()> {
    eprint!("\nProceed? (y/N) ");
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    if !line.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled");
        std::process::exit(130);
    }
    Ok(())
}
