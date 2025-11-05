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
        let suspect_count = directory.suspect_links().count();
        let cycles_raw = directory.cycles();
        let cycle_count = cycles_raw.len();
        let cycles: Vec<Vec<String>> = cycles_raw
            .iter()
            .map(|cycle| cycle.iter().map(ToString::to_string).collect())
            .collect();

        // Check if we have an empty repository
        if total == 0 {
            println!("No requirements found yet. Create one with 'req add'.");
            return Ok(());
        }

        match self.output {
            OutputFormat::Json => {
                Self::output_json(&counts, total, suspect_count, cycle_count, &cycles)?;
            }
            OutputFormat::Table => {
                if self.quiet {
                    Self::output_quiet(total, suspect_count, cycle_count);
                } else {
                    Self::output_table(&counts, total, suspect_count, cycle_count, &cycles);
                }
            }
        }

        // Exit with a non-zero code when the repository needs attention.
        let mut exit_code = 0;
        if cycle_count > 0 {
            exit_code = exit_code.max(3);
        }
        if suspect_count > 0 {
            exit_code = exit_code.max(2);
        }

        if exit_code != 0 {
            process::exit(exit_code);
        }

        Ok(())
    }

    fn output_json(
        counts: &BTreeMap<String, usize>,
        total: usize,
        suspect_count: usize,
        cycle_count: usize,
        cycles: &[Vec<String>],
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

        let cycle_groups: Vec<_> = cycles.to_vec();

        let output = json!({
            "kinds": kinds,
            "total": {
                "count": total,
                "delta": 0  // TODO: implement git delta
            },
            "suspect_links": suspect_count,
            "cycles": {
                "count": cycle_count,
                "members": cycle_groups,
            }
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn output_quiet(total: usize, suspect_count: usize, cycle_count: usize) {
        println!("total={total} suspect={suspect_count} cycles={cycle_count}");
    }

    fn output_table(
        counts: &BTreeMap<String, usize>,
        total: usize,
        suspect_count: usize,
        cycle_count: usize,
        cycles: &[Vec<String>],
    ) {
        const MAX_CYCLE_DISPLAY: usize = 5;
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

        // Suspect links summary with emoji
        if suspect_count == 0 {
            println!("Suspect links: {} ✅", "0".success());
        } else {
            println!("Suspect links: {} ⚠️", suspect_count.to_string().warning());
            println!("{}", "Run 'req suspect' to investigate.".dim());
        }

        println!();

        if cycle_count == 0 {
            println!("Cycles: {} ✅", "0".success());
        } else {
            println!("Cycles: {} ⚠️", cycle_count.to_string().warning());
            for cycle in cycles.iter().take(MAX_CYCLE_DISPLAY) {
                println!("  - {}", cycle.join(" -> "));
            }
            if cycle_count > MAX_CYCLE_DISPLAY {
                println!(
                    "  - ... and {} more cycles",
                    cycle_count - MAX_CYCLE_DISPLAY
                );
            }
            println!(
                "{}",
                "Resolve cycles to restore an acyclic dependency graph.".dim()
            );
        }
    }
}
