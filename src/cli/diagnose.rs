use std::path::Path;

use requiem::Directory;
use tracing::instrument;

use crate::cli::terminal::Colorize;

#[derive(Debug, clap::Parser)]
pub struct Command {
    #[command(subcommand)]
    command: DiagnoseCommand,
}

#[derive(Debug, clap::Parser)]
enum DiagnoseCommand {
    /// Diagnose path-related issues
    Paths,
}

impl Command {
    #[instrument]
    pub fn run(self, root: &Path) -> anyhow::Result<()> {
        match self.command {
            DiagnoseCommand::Paths => {
                let directory = Directory::new(root.to_path_buf())?;
                let digits = directory.config().digits();
                let mut issues: Vec<String> = Vec::new();

                for req in directory.requirements() {
                    // Get the actual path where this requirement was loaded from
                    let Some(actual_path) = directory.path_for(req.hrid) else {
                        continue; // Skip if path not found (shouldn't happen)
                    };

                    // Get the expected canonical path based on config
                    let expected_path = directory.canonical_path_for(req.hrid);

                    if actual_path != expected_path {
                        let hrid = req.hrid;
                        let expected_display = expected_path
                            .strip_prefix(root)
                            .unwrap_or(&expected_path)
                            .display();
                        let actual_display = actual_path
                            .strip_prefix(root)
                            .unwrap_or(actual_path)
                            .display();
                        issues.push(format!(
                            "{}: Expected '{expected_display}', found '{actual_display}'",
                            hrid.display(digits)
                        ));
                    }
                }

                if issues.is_empty() {
                    println!("{}", "✅ No path issues detected.".success());
                } else {
                    let issue_count = issues.len();
                    println!(
                        "{}",
                        format!("⚠️  {issue_count} path issues found:").warning()
                    );
                    println!();
                    for (i, issue) in issues.iter().enumerate() {
                        println!("{}. {}", i + 1, issue);
                    }
                    println!(
                        "\n{}",
                        "Review the issues above and fix them manually.".dim()
                    );
                }

                Ok(())
            }
        }
    }
}
