//! Automatic repair of fixable validation issues.

use requiem_core::Directory;

use super::{LinkIssue, Validate, ValidationResult};

impl Validate {
    pub(super) fn apply_fixes(
        &self,
        result: &ValidationResult,
        mut directory: Directory,
    ) -> anyhow::Result<()> {
        if self.dry_run {
            if !self.quiet {
                println!("\nDry run: showing what would be fixed...\n");
                preview_fixes(result);
            }
            return Ok(());
        }

        // Confirm before fixing
        if !self.yes && !self.quiet {
            let fixable = result.count_fixable_issues();
            println!("\nWill fix {fixable} issues:");
            preview_fixes(result);

            crate::cli::prompt_to_proceed()?;
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
