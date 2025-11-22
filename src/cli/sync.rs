use std::path::PathBuf;

use requiem::Directory;
use tracing::instrument;

use crate::cli::terminal::Colorize;

/// What to synchronize
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum SyncWhat {
    /// Update parent HRIDs in child requirements (default)
    Parents,
    /// Move files to canonical locations
    Paths,
    /// Update both parent HRIDs and file paths
    All,
}

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Command {
    /// What to synchronize
    #[arg(long, default_value = "parents")]
    what: SyncWhat,

    /// Check for drift without making changes (exits with code 2 if drift
    /// found)
    #[arg(long)]
    check: bool,

    /// Show what would be changed without making changes
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,

    /// Suppress output
    #[arg(long, short)]
    quiet: bool,
}

impl Command {
    #[instrument]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;

        match (self.what, self.check, self.dry_run) {
            (SyncWhat::Parents, true, _) => {
                self.check_parent_drift(&directory);
                Ok(())
            }
            (SyncWhat::Parents, false, dry_run) => self.sync_parents(&mut directory, dry_run),
            (SyncWhat::Paths, true, _) => {
                self.check_path_drift(&directory);
                Ok(())
            }
            (SyncWhat::Paths, false, dry_run) => self.sync_paths(&mut directory, dry_run),
            (SyncWhat::All, check, dry_run) => self.sync_all(&mut directory, check, dry_run),
        }
    }

    fn check_parent_drift(&self, directory: &Directory) {
        let would_update = directory.check_hrid_drift();
        if would_update.is_empty() {
            if !self.quiet {
                println!("{}", "✅ No HRID drift detected.".success());
            }
        } else {
            if !self.quiet {
                println!(
                    "{}",
                    format!(
                        "⚠️  {} requirements have stale parent HRIDs",
                        would_update.len()
                    )
                    .warning()
                );
                for hrid in &would_update {
                    println!("  • {}", hrid.display(directory.config().digits()));
                }
            }
            std::process::exit(2);
        }
    }

    fn sync_parents(&self, directory: &mut Directory, dry_run: bool) -> anyhow::Result<()> {
        let updated = directory.update_hrids();

        if updated.is_empty() {
            if !self.quiet {
                println!("{}", "✅ All parent HRIDs are current.".success());
            }
            return Ok(());
        }

        if dry_run {
            if !self.quiet {
                println!("Would update {} parent HRIDs:", updated.len());
                for hrid in &updated {
                    println!("  • {}", hrid.display(directory.config().digits()));
                }
            }
            return Ok(());
        }

        directory.flush()?;

        if !self.quiet {
            println!(
                "{}",
                format!("✅ Updated {} parent HRIDs", updated.len()).success()
            );
        }
        Ok(())
    }

    fn check_path_drift(&self, directory: &Directory) {
        let misplaced = directory.check_path_drift();
        if misplaced.is_empty() {
            if !self.quiet {
                println!(
                    "{}",
                    "✅ All requirements are in canonical locations.".success()
                );
            }
        } else {
            if !self.quiet {
                println!(
                    "{}",
                    format!("⚠️  {} requirements are misplaced", misplaced.len()).warning()
                );
                for (hrid, current, canonical) in &misplaced {
                    println!(
                        "  • {} ({} → {})",
                        hrid.display(directory.config().digits()),
                        current.display(),
                        canonical.display()
                    );
                }
            }
            std::process::exit(2);
        }
    }

    fn sync_paths(&self, directory: &mut Directory, dry_run: bool) -> anyhow::Result<()> {
        let misplaced = directory.check_path_drift();

        if misplaced.is_empty() {
            if !self.quiet {
                println!(
                    "{}",
                    "✅ All requirements are in canonical locations.".success()
                );
            }
            return Ok(());
        }

        if dry_run {
            if !self.quiet {
                println!("Would move {} files:", misplaced.len());
                for (hrid, current, canonical) in &misplaced {
                    println!(
                        "  • {}: {} → {}",
                        hrid.display(directory.config().digits()),
                        current.display(),
                        canonical.display()
                    );
                }
            }
            return Ok(());
        }

        // Confirm before moving files
        if !self.yes {
            use std::io::{self, BufRead};

            println!(
                "Will move {} files to canonical locations:",
                misplaced.len()
            );
            for (hrid, current, canonical) in &misplaced {
                println!(
                    "  • {}: {} → {}",
                    hrid.display(directory.config().digits()),
                    current.display(),
                    canonical.display()
                );
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

        let moved = directory.sync_paths()?;

        if !self.quiet {
            println!("{}", format!("✅ Moved {} files", moved.len()).success());
        }
        Ok(())
    }

    fn sync_all(
        &self,
        directory: &mut Directory,
        check: bool,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        let hrid_drift = directory.check_hrid_drift();
        let path_drift = directory.check_path_drift();

        if check {
            self.check_all_drift(&hrid_drift, &path_drift);
            return Ok(());
        }

        if dry_run {
            self.dry_run_all(&hrid_drift, &path_drift);
            return Ok(());
        }

        // Confirm before making changes
        if !self.yes && (!hrid_drift.is_empty() || !path_drift.is_empty()) {
            Self::confirm_sync_all(&hrid_drift, &path_drift)?;
        }

        // Perform both updates
        let updated_hrids = directory.update_hrids();
        if !updated_hrids.is_empty() {
            directory.flush()?;
        }

        let moved = directory.sync_paths()?;

        if !self.quiet {
            if !updated_hrids.is_empty() {
                println!(
                    "{}",
                    format!("✅ Updated {} parent HRIDs", updated_hrids.len()).success()
                );
            }
            if !moved.is_empty() {
                println!("{}", format!("✅ Moved {} files", moved.len()).success());
            }
            if updated_hrids.is_empty() && moved.is_empty() {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }
        Ok(())
    }

    fn check_all_drift(
        &self,
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) {
        let has_drift = !hrid_drift.is_empty() || !path_drift.is_empty();

        if !self.quiet {
            if !hrid_drift.is_empty() {
                println!(
                    "{}",
                    format!(
                        "⚠️  {} requirements have stale parent HRIDs",
                        hrid_drift.len()
                    )
                    .warning()
                );
            }
            if !path_drift.is_empty() {
                println!(
                    "{}",
                    format!("⚠️  {} requirements are misplaced", path_drift.len()).warning()
                );
            }
            if !has_drift {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }

        if has_drift {
            std::process::exit(2);
        }
    }

    fn dry_run_all(
        &self,
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) {
        if !self.quiet {
            if !hrid_drift.is_empty() {
                println!("Would update {} parent HRIDs", hrid_drift.len());
            }
            if !path_drift.is_empty() {
                println!("Would move {} files", path_drift.len());
            }
            if hrid_drift.is_empty() && path_drift.is_empty() {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }
    }

    fn confirm_sync_all(
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) -> anyhow::Result<()> {
        use std::io::{self, BufRead};

        println!("Will synchronize:");
        if !hrid_drift.is_empty() {
            println!("  • Update {} parent HRIDs", hrid_drift.len());
        }
        if !path_drift.is_empty() {
            println!("  • Move {} files", path_drift.len());
        }

        eprint!("\nProceed? (y/N) ");
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            std::process::exit(130);
        }
        Ok(())
    }
}
