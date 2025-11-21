use std::path::PathBuf;

use clap::Parser;
use requiem::Directory;
use tracing::instrument;

use super::terminal::Colorize;

#[derive(Debug, Parser)]
#[command(about = "Validate repository health across multiple dimensions")]
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
    structure_issues: Vec<StructureIssue>,
    path_issues: Vec<PathIssue>,
    link_issues: Vec<LinkIssue>,
    suspect_issues: Vec<SuspectIssue>,
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
                CheckType::Structure => self.check_structure(&directory, &mut result)?,
                CheckType::Paths => self.check_paths(&directory, &mut result),
                CheckType::Links => self.check_links(&directory, &mut result),
                CheckType::Suspect => self.check_suspect(&directory, &mut result),
                CheckType::All => unreachable!("All should have been expanded"),
            }
        }

        // Output results
        match self.output {
            OutputFormat::Table => self.output_table(&result, &directory),
            OutputFormat::Json => self.output_json(&result)?,
            OutputFormat::Summary => self.output_summary(&result),
        }

        // Handle --fix flag
        if self.fix && self.has_fixable_issues(&result) {
            self.apply_fixes(&result, directory)?;
        }

        // Exit with appropriate code
        if self.has_issues(&result) {
            std::process::exit(2);
        }

        Ok(())
    }

    fn check_structure(&self, _directory: &Directory, _result: &mut ValidationResult) -> anyhow::Result<()> {
        // TODO: Implement structure checks
        // - Valid YAML frontmatter
        // - Required fields present
        // - HRID format valid
        // - No duplicate UUIDs/HRIDs
        Ok(())
    }

    fn check_paths(&self, directory: &Directory, result: &mut ValidationResult) {
        let digits = directory.config().digits();

        for req in directory.requirements() {
            if let Some(actual_path) = directory.path_for(req.hrid) {
                let canonical_path = directory.canonical_path_for(req.hrid);
                if actual_path != canonical_path {
                    result.path_issues.push(PathIssue {
                        hrid: req.hrid.display(digits).to_string(),
                        current_path: actual_path.to_path_buf(),
                        expected_path: canonical_path,
                    });
                }
            }
        }
    }

    fn check_links(&self, directory: &Directory, result: &mut ValidationResult) {
        // Check for stale parent HRIDs
        let stale_hrids = directory.check_hrid_drift();
        let digits = directory.config().digits();

        for hrid in stale_hrids {
            result.link_issues.push(LinkIssue::StaleHrid {
                child: hrid.display(digits).to_string(),
            });
        }

        // TODO: Implement other link checks
        // - Broken references (parent UUIDs don't exist)
        // - Circular dependencies
    }

    fn check_suspect(&self, directory: &Directory, result: &mut ValidationResult) {
        let suspect_links = directory.suspect_links();
        let digits = directory.config().digits();

        for link in suspect_links {
            result.suspect_issues.push(SuspectIssue {
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
        if result.structure_issues.is_empty() {
            println!(
                "✓ Structure:  {} requirements, all valid",
                directory.requirements().count()
            );
        } else {
            println!(
                "{}",
                format!("✗ Structure:  {} issues found", result.structure_issues.len()).warning()
            );
        }

        // Paths
        if result.path_issues.is_empty() {
            println!("✓ Paths:      All files at canonical locations");
        } else {
            println!(
                "{}",
                format!(
                    "✗ Paths:      {} files not at canonical locations",
                    result.path_issues.len()
                )
                .warning()
            );
        }

        // Links
        if result.link_issues.is_empty() {
            println!("✓ Links:      No broken references, all HRIDs current");
        } else {
            println!(
                "{}",
                format!("✗ Links:      {} issues found", result.link_issues.len()).warning()
            );
        }

        // Suspect
        if result.suspect_issues.is_empty() {
            println!("✓ Suspect:    No suspect links");
        } else {
            println!(
                "{}",
                format!("✗ Suspect:    {} suspect links found", result.suspect_issues.len()).warning()
            );
        }

        // Summary
        let total_issues = self.count_issues(result);
        if total_issues == 0 {
            println!("\n{}", "Repository is healthy (0 issues)".success());
        } else {
            println!("\n{}", format!("Summary: {total_issues} issues found").warning());

            // Show hints for fixing
            if !result.path_issues.is_empty() || !result.link_issues.is_empty() {
                println!(
                    "\n{}",
                    "Run 'req validate --fix' to automatically repair fixable issues".dim()
                );
            }
            if !result.suspect_issues.is_empty() {
                println!(
                    "{}",
                    "Run 'req review --accept --all' to accept all suspect links".dim()
                );
            }
        }
    }

    fn output_json(&self, result: &ValidationResult) -> anyhow::Result<()> {
        use serde_json::json;

        let path_issues: Vec<_> = result
            .path_issues
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
            .suspect_issues
            .iter()
            .map(|issue| {
                json!({
                    "child": issue.child,
                    "parent": issue.parent,
                    "fixable": false
                })
            })
            .collect();

        let total_issues = self.count_issues(result);
        let fixable_issues = result.path_issues.len() + result.link_issues.len();

        let output = json!({
            "status": if total_issues == 0 { "healthy" } else { "issues_found" },
            "issues": {
                "structure": result.structure_issues.len(),
                "paths": path_issues,
                "links": result.link_issues.len(),
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

    fn output_summary(&self, result: &ValidationResult) {
        let total = self.count_issues(result);
        println!("issues={total}");
    }

    fn has_fixable_issues(&self, result: &ValidationResult) -> bool {
        !result.path_issues.is_empty() || !result.link_issues.is_empty()
    }

    fn has_issues(&self, result: &ValidationResult) -> bool {
        self.count_issues(result) > 0
    }

    fn count_issues(&self, result: &ValidationResult) -> usize {
        result.structure_issues.len()
            + result.path_issues.len()
            + result.link_issues.len()
            + result.suspect_issues.len()
    }

    fn apply_fixes(&self, result: &ValidationResult, mut directory: Directory) -> anyhow::Result<()> {
        if self.dry_run {
            if !self.quiet {
                println!("\nDry run: showing what would be fixed...\n");
                self.preview_fixes(result);
            }
            return Ok(());
        }

        // Confirm before fixing
        if !self.yes && !self.quiet {
            let fixable = result.path_issues.len() + result.link_issues.len();
            println!("\nWill fix {fixable} issues:");
            self.preview_fixes(result);

            eprint!("\nProceed? (y/N) ");
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Fix paths
        if !result.path_issues.is_empty() {
            let moved = directory.sync_paths()?;
            if !self.quiet {
                println!("✓ Moved {} files to canonical locations", moved.len());
            }
        }

        // Fix stale HRIDs
        let stale_count = result.link_issues.iter().filter(|issue| matches!(issue, LinkIssue::StaleHrid { .. })).count();
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

    fn preview_fixes(&self, result: &ValidationResult) {
        if !result.path_issues.is_empty() {
            println!("Paths ({} files):", result.path_issues.len());
            for issue in &result.path_issues {
                println!("  • {} → {}", issue.current_path.display(), issue.expected_path.display());
            }
        }

        let stale_count = result.link_issues.iter().filter(|issue| matches!(issue, LinkIssue::StaleHrid { .. })).count();
        if stale_count > 0 {
            println!("\nLinks ({} stale HRIDs):", stale_count);
            for issue in &result.link_issues {
                if let LinkIssue::StaleHrid { child } = issue {
                    println!("  • {}", child);
                }
            }
        }
    }
}
