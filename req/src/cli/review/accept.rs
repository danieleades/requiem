//! Accepting suspect links (updating stored fingerprints).

use std::path::PathBuf;

use requiem_core::Directory;
use tracing::instrument;

use super::Command;
use crate::cli::terminal::Colorize;

impl Command {
    /// Handle accepting suspect links
    #[instrument]
    pub(super) fn run_accept(self, path: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(path)?;
        let digits = directory.config().digits();
        let mut suspect_links = directory.suspect_links();

        // Apply filters (same as display mode)
        self.apply_filters(&mut suspect_links);

        if suspect_links.is_empty() {
            println!("No suspect links to accept.");
            return Ok(());
        }

        // Handle --all flag
        if self.all {
            let count = suspect_links.len();
            let mut files = std::collections::HashSet::new();
            for link in &suspect_links {
                files.insert(link.child_hrid.display(digits).to_string());
            }
            let file_count = files.len();

            // Show preview and confirm
            if !self.yes {
                println!("Will accept {count} suspect links across {file_count} files:");
                for link in &suspect_links {
                    println!(
                        "  {} ← {}",
                        link.child_hrid.display(digits),
                        link.parent_hrid.display(digits)
                    );
                }

                crate::cli::prompt_to_proceed()?;
            }

            // Accept all
            let updated = directory.accept_all_suspect_links();
            directory.flush()?;

            println!(
                "{}",
                format!("✅ Accepted {} suspect links", updated.len()).success()
            );
        } else {
            // Single link mode - require both child and parent
            let child = self.child.ok_or_else(|| {
                anyhow::anyhow!("--child is required when accepting without --all")
            })?;
            let parent = self.parent.ok_or_else(|| {
                anyhow::anyhow!("--parent is required when accepting without --all")
            })?;

            // Check if the link is actually suspect
            let link = suspect_links
                .iter()
                .find(|l| l.child_hrid == child && l.parent_hrid == parent);

            if let Some(link) = link {
                if !self.yes {
                    println!(
                        "Reviewing: {} ← {}",
                        child.display(digits),
                        parent.display(digits)
                    );
                    println!("Stored:    {}", link.stored_fingerprint);
                    println!("Current:   {}", link.current_fingerprint);

                    crate::cli::confirm("Accept this link?")?;
                }
            }

            match directory.accept_suspect_link(child.clone(), parent.clone())? {
                requiem_core::AcceptResult::Updated => {
                    directory.flush()?;
                    println!(
                        "{}",
                        format!(
                            "✅ Accepted {} ← {}",
                            child.display(digits),
                            parent.display(digits)
                        )
                        .success()
                    );
                }
                requiem_core::AcceptResult::AlreadyUpToDate => {
                    println!("No changes: link already up-to-date.");
                }
            }
        }

        Ok(())
    }
}
