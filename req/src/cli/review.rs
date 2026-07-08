//! The `req review` command: inspect and accept suspect links.
//!
//! Display formatting lives in the `display` submodule and the accept
//! workflow in `accept`; this module holds the CLI surface and shared
//! filtering.

use std::path::PathBuf;

use requiem_core::{Directory, Hrid, SuspectLink};
use tracing::instrument;

use crate::cli::{parse_hrid, terminal::Colorize};

mod accept;
mod display;

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Command {
    /// Accept suspect links (update fingerprints)
    #[arg(long)]
    accept: bool,

    /// Accept all suspect links matching filters
    #[arg(long, requires = "accept")]
    all: bool,

    /// Skip confirmation prompts when accepting
    #[arg(long, short, requires = "accept")]
    yes: bool,

    /// Show detailed information including fingerprints and paths
    #[arg(long, conflicts_with = "accept")]
    detail: bool,

    /// Output format (table, json, ndjson)
    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "table",
        conflicts_with = "accept"
    )]
    format: SuspectFormat,

    /// Show summary statistics
    #[arg(long, conflicts_with = "accept")]
    stats: bool,

    /// Quiet mode: output only CHILD PARENT pairs (no headers, no colors)
    #[arg(
        long,
        short,
        conflicts_with = "detail",
        conflicts_with = "stats",
        conflicts_with = "accept"
    )]
    quiet: bool,

    /// Filter by child requirement HRID
    #[arg(long, value_parser = parse_hrid)]
    child: Option<Hrid>,

    /// Filter by parent requirement HRID
    #[arg(long, value_parser = parse_hrid)]
    parent: Option<Hrid>,

    /// Filter by child requirement kind
    #[arg(long)]
    kind: Option<String>,

    /// Group output by field (parent, child, none)
    #[arg(long, value_name = "FIELD", conflicts_with = "accept")]
    group_by: Option<GroupBy>,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum SuspectFormat {
    #[default]
    Table,
    Json,
    Ndjson,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum GroupBy {
    Parent,
    Child,
    None,
}

impl Command {
    #[instrument]
    pub fn run(self, path: PathBuf) -> anyhow::Result<()> {
        // If --accept flag is used, handle accept logic
        if self.accept {
            return self.run_accept(path);
        }

        // Otherwise, show suspect links
        let directory = Directory::new(path)?;
        let digits = directory.config().digits();
        let mut suspect_links = directory.suspect_links();

        self.apply_filters(&mut suspect_links);

        // Handle empty results
        if suspect_links.is_empty() {
            if self.child.is_some() || self.parent.is_some() || self.kind.is_some() {
                println!("No suspect links matched the specified filters.");
                return Ok(());
            }
            println!("{}", "✅ No suspect links detected.".success());
            return Ok(());
        }

        // Quiet mode bypasses all formatting
        if self.quiet {
            for link in &suspect_links {
                println!(
                    "{} {}",
                    link.child_hrid.display(digits),
                    link.parent_hrid.display(digits)
                );
            }
            std::process::exit(2);
        }

        // Show stats if requested
        if self.stats {
            Self::output_stats(&suspect_links, digits);
            println!();
        }

        match self.format {
            SuspectFormat::Json => {
                Self::output_json(&suspect_links, &directory, digits)?;
            }
            SuspectFormat::Ndjson => {
                Self::output_ndjson(&suspect_links, &directory, digits)?;
            }
            SuspectFormat::Table => {
                self.output_table(&suspect_links, &directory, digits);
            }
        }

        // Exit with code 2 to indicate suspect links exist (for CI)
        std::process::exit(2);
    }

    /// Apply the `--child`, `--parent`, and `--kind` filters in place.
    fn apply_filters(&self, suspect_links: &mut Vec<SuspectLink>) {
        if let Some(ref child_filter) = self.child {
            suspect_links.retain(|link| &link.child_hrid == child_filter);
        }
        if let Some(ref parent_filter) = self.parent {
            suspect_links.retain(|link| &link.parent_hrid == parent_filter);
        }
        if let Some(ref kind_filter) = self.kind {
            // Normalize to uppercase for comparison (kinds are stored uppercase)
            let kind_upper = kind_filter.to_uppercase();
            suspect_links.retain(|link| link.child_hrid.kind() == kind_upper);
        }
    }
}

/// The path a requirement was loaded from, falling back to its canonical
/// location.
fn display_path(directory: &Directory, hrid: &Hrid) -> String {
    directory.path_for(hrid).map_or_else(
        || directory.canonical_path_for(hrid).display().to_string(),
        |p| p.display().to_string(),
    )
}
