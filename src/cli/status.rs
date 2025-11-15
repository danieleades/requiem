use std::{collections::BTreeMap, path::PathBuf, process};

use clap::Parser;
use requiem::Directory;
use tracing::instrument;

use super::terminal::{is_narrow, Colorize};

#[derive(Debug, Parser, Default)]
#[command(about = "Show requirement counts and suspect link totals")]
pub struct Status {
    /// Output format (table, json)
    #[arg(long, value_name = "FORMAT", default_value = "table")]
    output: OutputFormat,

    /// Suppress headers and format for scripting
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Table,
    Json,
}

impl Status {
    #[instrument(level = "debug", skip(self))]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root)?;

        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for requirement in directory.requirements() {
            *counts
                .entry(requirement.hrid.kind().to_string())
                .or_insert(0) += 1;
        }

        let total: usize = counts.values().sum();
        let suspect_count = directory.suspect_links().len();

        // Check for path issues (files not at canonical locations)
        let mut path_issues = 0;
        for req in directory.requirements() {
            if let Some(actual_path) = directory.actual_path_for(req.hrid) {
                let canonical_path = directory.path_for(req.hrid);
                if actual_path != canonical_path {
                    path_issues += 1;
                }
            }
        }

        // Check if we have an empty repository
        if total == 0 {
            println!("No requirements found yet. Create one with 'req add'.");
            return Ok(());
        }

        match self.output {
            OutputFormat::Json => {
                Self::output_json(&counts, total, suspect_count, path_issues)?;
            }
            OutputFormat::Table => {
                if self.quiet {
                    Self::output_quiet(&counts, total, suspect_count, path_issues);
                } else {
                    Self::output_table(&counts, total, suspect_count, path_issues);
                }
            }
        }

        // Exit with code 2 if suspect links or path issues exist (for CI)
        if suspect_count > 0 || path_issues > 0 {
            process::exit(2);
        }

        Ok(())
    }

    fn output_json(
        counts: &BTreeMap<String, usize>,
        total: usize,
        suspect_count: usize,
        path_issues: usize,
    ) -> anyhow::Result<()> {
        use serde_json::json;

        let kinds: Vec<_> = counts
            .iter()
            .map(|(kind, count)| {
                json!({
                    "kind": kind,
                    "count": count,
                    "delta": 0  // TODO: implement git delta
                })
            })
            .collect();

        let output = json!({
            "kinds": kinds,
            "total": {
                "count": total,
                "delta": 0  // TODO: implement git delta
            },
            "suspect_links": suspect_count,
            "path_issues": path_issues
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn output_quiet(
        _counts: &BTreeMap<String, usize>,
        total: usize,
        suspect_count: usize,
        path_issues: usize,
    ) {
        println!("total={total} suspect={suspect_count} paths={path_issues}");
    }

    fn output_table(
        counts: &BTreeMap<String, usize>,
        total: usize,
        suspect_count: usize,
        path_issues: usize,
    ) {
        let narrow = is_narrow();

        println!("Requirement counts");
        println!("{}", "──────────────────".dim());

        if narrow {
            // Stacked output for narrow terminals
            for (kind, count) in counts {
                println!("{}: {} (Δ {})", kind, count, "–".dim());
            }
            println!("Total: {total}");
        } else {
            // Table layout
            println!("{:<10} {:<6} Δ", "Kind", "Count");
            for (kind, count) in counts {
                println!("{kind:<10} {count:<6} {}", "–".dim());
            }
            println!("Total      {total}");
        }

        println!();
        println!("Health checks:");

        // Suspect links summary with emoji
        if suspect_count == 0 {
            println!("  Suspect links: {} ✅", "0".success());
        } else {
            println!(
                "  Suspect links: {} ⚠️",
                suspect_count.to_string().warning()
            );
        }

        // Path issues summary with emoji
        if path_issues == 0 {
            println!("  Path issues:   {} ✅", "0".success());
        } else {
            println!("  Path issues:   {} ⚠️", path_issues.to_string().warning());
        }

        // Show help hints if there are any issues
        if suspect_count > 0 || path_issues > 0 {
            println!();
            if suspect_count > 0 {
                println!(
                    "{}",
                    "Run 'req suspect' to investigate suspect links.".dim()
                );
            }
            if path_issues > 0 {
                println!("{}", "Run 'req diagnose paths' to see path details.".dim());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use requiem::Directory;
    use tempfile::tempdir;

    #[test]
    fn status_detects_path_issues_in_path_based_mode() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // Create .req/config.toml with path-based mode and allow_unrecognised
        let config_dir = root.join(".req");
        fs::create_dir(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\ndigits = 3\nallow_unrecognised = \
             true\n",
        )
        .unwrap();

        // Create a requirement using the API (will be at canonical location)
        let mut directory = Directory::new(root.clone()).unwrap();
        let req = directory
            .add_requirement("REQ", "# Test Requirement\n\nBody content".to_string())
            .unwrap();
        directory.flush().unwrap();

        // Now move the file to a non-canonical location
        let canonical_path = directory.path_for(req.hrid());
        let non_canonical_path = root.join("moved").join("REQ-001.md");
        fs::create_dir_all(non_canonical_path.parent().unwrap()).unwrap();
        fs::rename(&canonical_path, &non_canonical_path).unwrap();

        // Reload directory - it should load from the non-canonical location
        let directory = Directory::new(root).unwrap();

        // Run status check
        let mut path_issues = 0;
        for req in directory.requirements() {
            if let Some(actual_path) = directory.actual_path_for(req.hrid) {
                let canonical_path = directory.path_for(req.hrid);
                if actual_path != canonical_path {
                    path_issues += 1;
                }
            }
        }

        assert_eq!(
            path_issues, 1,
            "Should detect one path issue for non-canonical location"
        );
    }

    #[test]
    fn status_no_path_issues_when_files_at_canonical_locations() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // Create .req/config.toml with path-based mode
        let config_dir = root.join(".req");
        fs::create_dir(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = true\ndigits = 3\n",
        )
        .unwrap();

        // Create a requirement using the API (will be at canonical location)
        let mut directory = Directory::new(root.clone()).unwrap();
        directory
            .add_requirement("REQ", "# Test Requirement\n\nBody content".to_string())
            .unwrap();
        directory.flush().unwrap();

        // Reload directory
        let directory = Directory::new(root).unwrap();

        // Run status check
        let mut path_issues = 0;
        for req in directory.requirements() {
            if let Some(actual_path) = directory.actual_path_for(req.hrid) {
                let canonical_path = directory.path_for(req.hrid);
                if actual_path != canonical_path {
                    path_issues += 1;
                }
            }
        }

        assert_eq!(
            path_issues, 0,
            "Should detect no path issues when file is at canonical location"
        );
    }

    #[test]
    fn status_detects_path_issues_filename_based_mode() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // Create .req/config.toml with filename-based mode (default)
        let config_dir = root.join(".req");
        fs::create_dir(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            "_version = \"1\"\nsubfolders_are_namespaces = false\ndigits = 3\nallow_unrecognised \
             = true\n",
        )
        .unwrap();

        // Create requirement in a subdirectory (non-canonical for filename mode)
        let subdir = root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(
            subdir.join("REQ-001.md"),
            r#"---
_version: '1'
uuid: "550e8400-e29b-41d4-a716-446655440000"
created: 2024-01-01T00:00:00Z
---
# REQ-001 Test Requirement

This is a test.
"#,
        )
        .unwrap();

        let directory = Directory::new(root.to_path_buf()).unwrap();

        // Check for path issues
        let mut path_issues = 0;
        for req in directory.requirements() {
            if let Some(actual_path) = directory.actual_path_for(req.hrid) {
                let canonical_path = directory.path_for(req.hrid);
                if actual_path != canonical_path {
                    path_issues += 1;
                }
            }
        }

        assert!(
            path_issues > 0,
            "Should detect path issues in filename mode"
        );
    }
}
