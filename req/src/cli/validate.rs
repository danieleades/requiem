//! The `req validate` command: repository health checks.
//!
//! The individual checks live in the `check` submodule, output formatting in
//! `report`, and automatic repair in `fix`; this module holds the CLI surface
//! and the result model.

use std::path::PathBuf;

use clap::Parser;
use requiem_core::Directory;
use tracing::instrument;

mod check;
mod fix;
mod report;

#[derive(Debug, Parser)]
#[command(about = "Validate repository health across multiple dimensions")]
#[allow(clippy::struct_excessive_bools)]
pub struct Validate {
    /// Types of checks to run (can be specified multiple times)
    #[arg(long, value_name = "TYPE")]
    check: Vec<CheckType>,

    /// Attempt automatic repair of fixable issues
    #[arg(long)]
    fix: bool,

    /// Show what would be fixed without making changes
    #[arg(long)]
    dry_run: bool,

    /// Output format
    #[arg(long, value_name = "FORMAT", default_value = "table")]
    output: OutputFormat,

    /// Suppress all output except errors
    #[arg(long, short)]
    quiet: bool,

    /// Skip confirmation prompts when fixing
    #[arg(long, short)]
    yes: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
enum CheckType {
    /// Check file structure (YAML, required fields, HRIDs, duplicates)
    Structure,
    /// Check files are at canonical locations
    Paths,
    /// Check for broken references and circular dependencies
    Links,
    /// Check parent fingerprints match current content
    Suspect,
    /// Run all checks
    All,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Table,
    Json,
    Summary,
}

#[derive(Debug, Default)]
struct ValidationResult {
    paths: Vec<PathIssue>,
    links: Vec<LinkIssue>,
    suspect: Vec<SuspectIssue>,
}

impl ValidationResult {
    /// Count the total number of issues across all categories.
    fn count_total_issues(&self) -> usize {
        self.paths.len() + self.links.len() + self.suspect.len()
    }

    /// Count only the fixable issues (paths + stale HRIDs).
    fn count_fixable_issues(&self) -> usize {
        self.paths.len()
            + self
                .links
                .iter()
                .filter(|link| matches!(link, LinkIssue::StaleHrid { .. }))
                .count()
    }

    /// Count unfixable issues (structure + broken refs + cycles + suspect).
    fn count_unfixable_issues(&self) -> usize {
        self.count_total_issues() - self.count_fixable_issues()
    }

    /// Count only stale HRID issues (subset of links).
    fn count_stale_hrids(&self) -> usize {
        self.links
            .iter()
            .filter(|link| matches!(link, LinkIssue::StaleHrid { .. }))
            .count()
    }
}

#[derive(Debug)]
struct PathIssue {
    hrid: String,
    current_path: PathBuf,
    expected_path: PathBuf,
}

#[derive(Debug)]
enum LinkIssue {
    BrokenReference { child: String, parent_uuid: String },
    CircularDependency { cycle: Vec<String> },
    StaleHrid { child: String },
}

#[derive(Debug)]
struct SuspectIssue {
    child: String,
    parent: String,
}

impl Validate {
    #[instrument(level = "debug", skip(self))]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root)?;

        // Determine which checks to run
        let checks = if self.check.is_empty() || self.check.contains(&CheckType::All) {
            &[
                CheckType::Structure,
                CheckType::Paths,
                CheckType::Links,
                CheckType::Suspect,
            ]
        } else {
            self.check.as_slice()
        };

        // Run checks and construct result immutably
        let result = ValidationResult {
            paths: if checks.contains(&CheckType::Paths) {
                check::paths(&directory)
            } else {
                vec![]
            },
            links: if checks.contains(&CheckType::Links) {
                check::links(&directory)
            } else {
                vec![]
            },
            suspect: if checks.contains(&CheckType::Suspect) {
                check::suspect(&directory)
            } else {
                vec![]
            },
        };

        // Output results
        match self.output {
            OutputFormat::Table => self.output_table(&result, &directory),
            OutputFormat::Json => Self::output_json(&result)?,
            OutputFormat::Summary => Self::output_summary(&result),
        }

        // Handle --fix flag
        if self.fix && (!result.paths.is_empty() || !result.links.is_empty()) {
            self.apply_fixes(&result, directory)?;
        }

        // Exit with error code if any remaining issues exist
        // What counts as "remaining" depends on what was actually executed:
        // - If --fix was actually applied: only unfixable issues remain (fixable were
        //   repaired)
        // - If --fix --dry-run or no --fix: all issues remain
        let remaining_issues = if self.fix && !self.dry_run {
            result.count_unfixable_issues()
        } else {
            result.count_total_issues()
        };

        if remaining_issues > 0 {
            std::process::exit(2);
        }

        Ok(())
    }
}
