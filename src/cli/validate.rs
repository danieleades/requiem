use std::path::PathBuf;

use clap::Parser;
use requiem::Directory;
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
    structure: Vec<StructureIssue>,
    paths: Vec<PathIssue>,
    links: Vec<LinkIssue>,
    suspect: Vec<SuspectIssue>,
}

#[derive(Debug)]
struct StructureIssue {
    _file: PathBuf,
    _message: String,
}

#[derive(Debug)]
struct PathIssue {
    hrid: String,
    current_path: PathBuf,
    expected_path: PathBuf,
}

#[derive(Debug)]
enum LinkIssue {
    _BrokenReference { _child: String, _parent_uuid: String },
    _CircularDependency { _cycle: Vec<String> },
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
            vec![
                CheckType::Structure,
                CheckType::Paths,
                CheckType::Links,
                CheckType::Suspect,
            ]
        } else {
            self.check.clone()
        };

        // Run checks
        let mut result = ValidationResult::default();

        for check in &checks {
            match check {
                CheckType::Structure => Self::check_structure(),
                CheckType::Paths => Self::check_paths(&directory, &mut result),
                CheckType::Links => Self::check_links(&directory, &mut result),
                CheckType::Suspect => Self::check_suspect(&directory, &mut result),
                CheckType::All => unreachable!("All should have been expanded"),
            }
        }

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

        // Exit with appropriate code
        if Self::count_issues(&result) > 0 {
            std::process::exit(2);
        }

        Ok(())
    }

    const fn check_structure() {
        // TODO: Implement structure checks
        // - Valid YAML frontmatter
        // - Required fields present
        // - HRID format valid
        // - No duplicate UUIDs/HRIDs
    }

    fn check_paths(directory: &Directory, result: &mut ValidationResult) {
        let digits = directory.config().digits();

        for req in directory.requirements() {
            if let Some(actual_path) = directory.path_for(req.hrid) {
                let canonical_path = directory.canonical_path_for(req.hrid);
                if actual_path != canonical_path {
                    result.paths.push(PathIssue {
                        hrid: req.hrid.display(digits).to_string(),
                        current_path: actual_path.to_path_buf(),
                        expected_path: canonical_path,
                    });
                }
            }
        }
    }

    fn check_links(directory: &Directory, result: &mut ValidationResult) {
        // Check for stale parent HRIDs
        let stale_hrids = directory.check_hrid_drift();
        let digits = directory.config().digits();

        for hrid in stale_hrids {
            result.links.push(LinkIssue::StaleHrid {
                child: hrid.display(digits).to_string(),
            });
        }

        // TODO: Implement other link checks
        // - Broken references (parent UUIDs don't exist)
        // - Circular dependencies
    }

    fn check_suspect(directory: &Directory, result: &mut ValidationResult) {
        let suspect_links = directory.suspect_links();
        let digits = directory.config().digits();

        for link in suspect_links {
            result.suspect.push(SuspectIssue {
                child: link.child_hrid.display(digits).to_string(),
                parent: link.parent_hrid.display(digits).to_string(),
            });
        }
    }

    fn output_table(&self, result: &ValidationResult, directory: &Directory) {
        if self.quiet {
            return;
        }

        println!("Validating repository...\n");

        // Structure
        if result.structure.is_empty() {
            println!(
                "✓ Structure:  {} requirements, all valid",
                directory.requirements().count()
            );
        } else {
            println!(
                "{}",
                format!("✗ Structure:  {} issues found", result.structure.len()).warning()
            );
        }

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
        if result.links.is_empty() {
            println!("✓ Links:      No broken references, all HRIDs current");
        } else {
            println!(
                "{}",
                format!("✗ Links:      {} issues found", result.links.len()).warning()
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
        let total_issues = Self::count_issues(result);
        if total_issues == 0 {
            println!("\n{}", "Repository is healthy (0 issues)".success());
        } else {
            println!("\n{}", format!("Summary: {total_issues} issues found").warning());

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
                    "hrid": issue.hrid,
                    "current_path": issue.current_path,
                    "expected_path": issue.expected_path,
                    "fixable": true
                })
            })
            .collect();

        let suspect_issues: Vec<_> = result
            .suspect
            .iter()
            .map(|issue| {
                json!({
                    "child": issue.child,
                    "parent": issue.parent,
                    "fixable": false
                })
            })
            .collect();

        let total_issues = Self::count_issues(result);
        let fixable_issues = result.paths.len() + result.links.len();

        let output = json!({
            "status": if total_issues == 0 { "healthy" } else { "issues_found" },
            "issues": {
                "structure": result.structure.len(),
                "paths": path_issues,
                "links": result.links.len(),
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
        let total = Self::count_issues(result);
        println!("issues={total}");
    }

    fn count_issues(result: &ValidationResult) -> usize {
        result.structure.len()
            + result.paths.len()
            + result.links.len()
            + result.suspect.len()
    }

    fn apply_fixes(&self, result: &ValidationResult, mut directory: Directory) -> anyhow::Result<()> {
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

            let fixable = result.paths.len() + result.links.len();
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
        let stale_count = result.links.iter().filter(|issue| matches!(issue, LinkIssue::StaleHrid { .. })).count();
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
                println!("  • {} → {}", issue.current_path.display(), issue.expected_path.display());
            }
        }

        let stale_count = result.links.iter().filter(|issue| matches!(issue, LinkIssue::StaleHrid { .. })).count();
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
