//! Output formatting for validation results.

use requiem_core::Directory;

use super::{LinkIssue, Validate, ValidationResult};
use crate::cli::terminal::Colorize;

impl Validate {
    pub(super) fn output_table(&self, result: &ValidationResult, directory: &Directory) {
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
        let (cycles, broken, stale) = categorize_link_issues(&result.links);
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

    pub(super) fn output_json(result: &ValidationResult) -> anyhow::Result<()> {
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

    pub(super) fn output_summary(result: &ValidationResult) {
        let total = result.count_total_issues();
        println!("issues={total}");
    }
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
