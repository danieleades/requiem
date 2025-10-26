use std::{collections::BTreeMap, path::PathBuf, process};

use clap::Parser;
use requiem::Directory;
use tracing::instrument;

#[derive(Debug, Parser, Default)]
#[command(about = "Show requirement counts and suspect link totals")]
pub struct Status;

impl Status {
    #[instrument(level = "debug", skip_all)]
    #[allow(clippy::unused_self)]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root).load_all()?;

        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for requirement in directory.requirements() {
            *counts
                .entry(requirement.hrid().kind().to_string())
                .or_insert(0) += 1;
        }

        let total: usize = counts.values().sum();
        let suspect_count = directory.suspect_links().len();

        println!("Requirement Counts");
        println!("==================");
        println!("{:<10} | {}", "Kind", "Count");
        println!("{:-<10}-+-{:-<5}", "", "");
        for (kind, count) in &counts {
            println!("{kind:<10} | {count}");
        }
        println!("{:-<10}-+-{:-<5}", "", "");
        println!("{:<10} | {total}", "Total");

        println!("\nSuspect links: {suspect_count}");

        if suspect_count > 0 {
            process::exit(1);
        }

        Ok(())
    }
}
