use std::path::PathBuf;

use clap::Parser;
use requiem_core::Directory;
use tracing::instrument;

use super::terminal::Colorize;

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
                Self::check_paths(&directory)
            } else {
                vec![]
            },
            links: if checks.contains(&CheckType::Links) {
                Self::check_links(&directory)
            } else {
                vec![]
            },
            suspect: if checks.contains(&CheckType::Suspect) {
                Self::check_suspect(&directory)
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

        // Exit with appropriate code based on remaining unfixable issues
        if result.count_unfixable_issues() > 0 {
            std::process::exit(2);
        }

        Ok(())
    }

    fn check_paths(directory: &Directory) -> Vec<PathIssue> {
        let digits = directory.config().digits();
        let mut issues = Vec::new();

        for req in directory.requirements() {
            if let Some(actual_path) = directory.path_for(req.hrid) {
                let canonical_path = directory.canonical_path_for(req.hrid);
                if actual_path != canonical_path {
                    issues.push(PathIssue {
                        hrid: req.hrid.display(digits).to_string(),
                        current_path: actual_path.to_path_buf(),
                        expected_path: canonical_path,
                    });
                }
            }
        }

        issues
    }

    fn check_links(directory: &Directory) -> Vec<LinkIssue> {
        let digits = directory.config().digits();
        let mut issues = Vec::new();

        // Check for stale parent HRIDs
        let stale_hrids = directory.check_hrid_drift();
        for hrid in stale_hrids {
            issues.push(LinkIssue::StaleHrid {
                child: hrid.display(digits).to_string(),
            });
        }

        // Check for circular dependencies
        let cycles = directory.detect_cycles();
        for cycle in cycles {
            let cycle_path: Vec<String> = cycle
                .iter()
                .map(|hrid| hrid.display(digits).to_string())
                .collect();
            issues.push(LinkIssue::CircularDependency { cycle: cycle_path });
        }

        // Check for broken references (parent UUIDs that don't exist)
        for req in directory.requirements() {
            for (parent_uuid, _parent_info) in &req.parents {
                if directory.find_by_uuid(*parent_uuid).is_none() {
                    issues.push(LinkIssue::BrokenReference {
                        child: req.hrid.display(digits).to_string(),
                        parent_uuid: parent_uuid.to_string(),
                    });
                }
            }
        }

        issues
    }

    fn check_suspect(directory: &Directory) -> Vec<SuspectIssue> {
        let suspect_links = directory.suspect_links();
        let digits = directory.config().digits();
        let mut issues = Vec::new();

        for link in suspect_links {
            issues.push(SuspectIssue {
                child: link.child_hrid.display(digits).to_string(),
                parent: link.parent_hrid.display(digits).to_string(),
            });
        }

        issues
    }

    fn output_table(&self, result: &ValidationResult, directory: &Directory) {
        if self.quiet {
            return;
        }

        println!("Validating repository...\n");

        // Structure (always valid if we got here)
        let req_count = directory.requirements().count();
        println!("✓ Structure:  {req_count} requirements, all valid");

        // Paths
        if result.paths.is_empty() {
            println!("✓ Paths:      All files at canonical locations");
        } else {
            println!(
                "{}",
                format!(
                    "✗ Paths:      {} files not at canonical locations",
                    result.paths.len()
                )
                .warning()
            );
        }

        // Links
        let (cycles, broken, stale) = Self::categorize_link_issues(&result.links);
        if result.links.is_empty() {
            println!("✓ Links:      No broken references, cycles, or stale HRIDs");
        } else {
            let mut issues_desc = Vec::new();
            if !cycles.is_empty() {
                issues_desc.push(format!("{} cycle(s)", cycles.len()));
            }
            if !broken.is_empty() {
                issues_desc.push(format!("{} broken ref(s)", broken.len()));
            }
            if !stale.is_empty() {
                issues_desc.push(format!("{} stale HRID(s)", stale.len()));
            }
            println!(
                "{}",
                format!(
                    "✗ Links:      {} ({})",
                    result.links.len(),
                    issues_desc.join(", ")
                )
                .warning()
            );
        }

        // Suspect
        if result.suspect.is_empty() {
            println!("✓ Suspect:    No suspect links");
        } else {
            println!(
                "{}",
                format!("✗ Suspect:    {} suspect links found", result.suspect.len()).warning()
            );
        }

        // Summary
        let total_issues = result.count_total_issues();
        if total_issues == 0 {
            println!("\n{}", "Repository is healthy (0 issues)".success());
        } else {
            println!(
                "\n{}",
                format!("Summary: {total_issues} issues found").warning()
            );

            // Show hints for fixing
            if !result.paths.is_empty() || !result.links.is_empty() {
                println!(
                    "\n{}",
                    "Run 'req validate --fix' to automatically repair fixable issues".dim()
                );
            }
            if !result.suspect.is_empty() {
                println!(
                    "{}",
                    "Run 'req review --accept --all' to accept all suspect links".dim()
                );
            }
        }
    }

    fn output_json(result: &ValidationResult) -> anyhow::Result<()> {
        use serde_json::json;

        let path_issues: Vec<_> = result
            .paths
            .iter()
            .map(|issue| {
                json!({
                    "type": "path",
                    "hrid": issue.hrid,
                    "current_path": issue.current_path,
                    "expected_path": issue.expected_path,
                    "fixable": true
                })
            })
            .collect();

        let link_issues: Vec<_> = result
            .links
            .iter()
            .map(|issue| match issue {
                LinkIssue::CircularDependency { cycle } => json!({
                    "type": "circular_dependency",
                    "cycle": cycle,
                    "fixable": false
                }),
                LinkIssue::BrokenReference { child, parent_uuid } => json!({
                    "type": "broken_reference",
                    "child": child,
                    "parent_uuid": parent_uuid,
                    "fixable": false
                }),
                LinkIssue::StaleHrid { child } => json!({
                    "type": "stale_hrid",
                    "child": child,
                    "fixable": true
                }),
            })
            .collect();

        let suspect_issues: Vec<_> = result
            .suspect
            .iter()
            .map(|issue| {
                json!({
                    "type": "suspect",
                    "child": issue.child,
                    "parent": issue.parent,
                    "fixable": false
                })
            })
            .collect();

        let total_issues = result.count_total_issues();
        let fixable_issues = result.count_fixable_issues();

        let output = json!({
            "status": if total_issues == 0 { "healthy" } else { "issues_found" },
            "issues": {
                "structure": [],
                "paths": path_issues,
                "links": link_issues,
                "suspect": suspect_issues
            },
            "summary": {
                "total_issues": total_issues,
                "fixable_issues": fixable_issues,
                "manual_issues": total_issues - fixable_issues
            }
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn output_summary(result: &ValidationResult) {
        let total = result.count_total_issues();
        println!("issues={total}");
    }

    fn categorize_link_issues(
        links: &[LinkIssue],
    ) -> (Vec<&LinkIssue>, Vec<&LinkIssue>, Vec<&LinkIssue>) {
        let mut cycles = Vec::new();
        let mut broken = Vec::new();
        let mut stale = Vec::new();

        for link in links {
            match link {
                LinkIssue::CircularDependency { .. } => cycles.push(link),
                LinkIssue::BrokenReference { .. } => broken.push(link),
                LinkIssue::StaleHrid { .. } => stale.push(link),
            }
        }

        (cycles, broken, stale)
    }

    fn apply_fixes(
        &self,
        result: &ValidationResult,
        mut directory: Directory,
    ) -> anyhow::Result<()> {
        if self.dry_run {
            if !self.quiet {
                println!("\nDry run: showing what would be fixed...\n");
                Self::preview_fixes(result);
            }
            return Ok(());
        }

        // Confirm before fixing
        if !self.yes && !self.quiet {
            use std::io::{self, BufRead};

            let fixable = result.count_fixable_issues();
            println!("\nWill fix {fixable} issues:");
            Self::preview_fixes(result);

            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Fix paths
        if !result.paths.is_empty() {
            let moved = directory.sync_paths()?;
            if !self.quiet {
                println!("✓ Moved {} files to canonical locations", moved.len());
            }
        }

        // Fix stale HRIDs
        let stale_count = result.count_stale_hrids();
        if stale_count > 0 {
            let updated = directory.update_hrids();
            if !updated.is_empty() {
                directory.flush()?;
                if !self.quiet {
                    println!("✓ Updated {} parent HRIDs", updated.len());
                }
            }
        }

        Ok(())
    }

    fn preview_fixes(result: &ValidationResult) {
        if !result.paths.is_empty() {
            println!("Paths ({} files):", result.paths.len());
            for issue in &result.paths {
                println!(
                    "  • {} → {}",
                    issue.current_path.display(),
                    issue.expected_path.display()
                );
            }
        }

        let stale_count = result.count_stale_hrids();
        if stale_count > 0 {
            println!("\nLinks ({stale_count} stale HRIDs):");
            for issue in &result.links {
                if let LinkIssue::StaleHrid { child } = issue {
                    println!("  • {child}");
                }
            }
        }
    }
}
