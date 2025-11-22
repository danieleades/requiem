use std::{io::BufRead, path::PathBuf};

use requiem::{Directory, Hrid};
use tracing::instrument;

use crate::cli::{parse_hrid, terminal::Colorize};

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Command {
    /// Accept suspect links (update fingerprints)
    #[arg(long)]
    accept: bool,

    /// Accept all suspect links matching filters
    #[arg(long, requires = "accept")]
    all: bool,

    /// Skip confirmation prompts when accepting
    #[arg(long, short, requires = "accept")]
    yes: bool,

    /// Show detailed information including fingerprints and paths
    #[arg(long, conflicts_with = "accept")]
    detail: bool,

    /// Output format (table, json, ndjson)
    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "table",
        conflicts_with = "accept"
    )]
    format: SuspectFormat,

    /// Show summary statistics
    #[arg(long, conflicts_with = "accept")]
    stats: bool,

    /// Quiet mode: output only CHILD PARENT pairs (no headers, no colors)
    #[arg(
        long,
        short,
        conflicts_with = "detail",
        conflicts_with = "stats",
        conflicts_with = "accept"
    )]
    quiet: bool,

    /// Filter by child requirement HRID
    #[arg(long, value_parser = parse_hrid)]
    child: Option<Hrid>,

    /// Filter by parent requirement HRID
    #[arg(long, value_parser = parse_hrid)]
    parent: Option<Hrid>,

    /// Filter by child requirement kind
    #[arg(long)]
    kind: Option<String>,

    /// Group output by field (parent, child, none)
    #[arg(long, value_name = "FIELD", conflicts_with = "accept")]
    group_by: Option<GroupBy>,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum SuspectFormat {
    #[default]
    Table,
    Json,
    Ndjson,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum GroupBy {
    Parent,
    Child,
    None,
}

impl Command {
    #[instrument]
    pub fn run(self, path: PathBuf) -> anyhow::Result<()> {
        // If --accept flag is used, handle accept logic
        if self.accept {
            return self.run_accept(path);
        }

        // Otherwise, show suspect links
        let directory = Directory::new(path)?;
        let digits = directory.config().digits();
        let mut suspect_links = directory.suspect_links();

        // Apply filters
        if let Some(ref child_filter) = self.child {
            suspect_links.retain(|link| &link.child_hrid == child_filter);
        }
        if let Some(ref parent_filter) = self.parent {
            suspect_links.retain(|link| &link.parent_hrid == parent_filter);
        }
        if let Some(ref kind_filter) = self.kind {
            // Normalize to uppercase for comparison (kinds are stored uppercase)
            let kind_upper = kind_filter.to_uppercase();
            suspect_links.retain(|link| link.child_hrid.kind() == kind_upper);
        }

        // Handle empty results
        if suspect_links.is_empty() {
            if self.child.is_some() || self.parent.is_some() || self.kind.is_some() {
                println!("No suspect links matched the specified filters.");
                return Ok(());
            }
            println!("{}", "✅ No suspect links detected.".success());
            return Ok(());
        }

        // Quiet mode bypasses all formatting
        if self.quiet {
            for link in &suspect_links {
                println!(
                    "{} {}",
                    link.child_hrid.display(digits),
                    link.parent_hrid.display(digits)
                );
            }
            std::process::exit(2);
        }

        // Show stats if requested
        if self.stats {
            Self::output_stats(&suspect_links, &directory, digits);
            println!();
        }

        match self.format {
            SuspectFormat::Json => {
                Self::output_json(&suspect_links, &directory, digits)?;
            }
            SuspectFormat::Ndjson => {
                Self::output_ndjson(&suspect_links, &directory, digits)?;
            }
            SuspectFormat::Table => {
                self.output_table(&suspect_links, &directory, digits);
            }
        }

        // Exit with code 2 to indicate suspect links exist (for CI)
        std::process::exit(2);
    }

    fn output_stats(suspect_links: &[requiem::SuspectLink], _directory: &Directory, digits: usize) {
        use std::collections::{HashMap, HashSet};

        let unique_parents: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.parent_hrid.display(digits).to_string())
            .collect();
        let unique_children: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.child_hrid.display(digits).to_string())
            .collect();

        println!("Suspect Link Statistics");
        println!();
        println!("Total suspect links:       {}", suspect_links.len());
        println!("Unique parents affected:   {}", unique_parents.len());
        println!("Unique children affected:  {}", unique_children.len());
        println!();

        // Count by child kind
        let mut by_kind: HashMap<String, usize> = HashMap::new();
        for link in suspect_links {
            *by_kind
                .entry(link.child_hrid.kind().to_string())
                .or_insert(0) += 1;
        }

        if !by_kind.is_empty() {
            println!("By child requirement kind:");
            let mut kinds: Vec<_> = by_kind.iter().collect();
            kinds.sort_by_key(|(k, _)| *k);
            for (kind, count) in kinds {
                println!("  {kind}  →  *     {count} links");
            }
            println!();
        }

        // Most affected parents
        let mut parent_counts: HashMap<String, usize> = HashMap::new();
        for link in suspect_links {
            *parent_counts
                .entry(link.parent_hrid.display(digits).to_string())
                .or_insert(0) += 1;
        }

        if parent_counts.len() > 1 {
            let mut parent_list: Vec<_> = parent_counts.iter().collect();
            parent_list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

            println!("Most affected parents:");
            for (hrid, count) in parent_list.iter().take(3) {
                println!("  {hrid}  ({count} children)");
            }
        }
    }

    fn output_json(
        suspect_links: &[requiem::SuspectLink],
        directory: &Directory,
        digits: usize,
    ) -> anyhow::Result<()> {
        use std::collections::{HashMap, HashSet};

        use serde_json::json;

        let unique_parents: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.parent_hrid.display(digits).to_string())
            .collect();
        let unique_children: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.child_hrid.display(digits).to_string())
            .collect();

        let mut by_kind: HashMap<String, usize> = HashMap::new();
        for link in suspect_links {
            *by_kind
                .entry(link.child_hrid.kind().to_string())
                .or_insert(0) += 1;
        }

        let links: Vec<_> = suspect_links
            .iter()
            .map(|link| {
                let child_req = directory.requirement_by_hrid(&link.child_hrid);
                let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

                json!({
                    "child": {
                        "hrid": link.child_hrid.display(digits).to_string(),
                        "title": child_req.map(|r| r.title().to_string()),
                        "path": directory.path_for(&link.child_hrid).map_or_else(
                            || directory.canonical_path_for(&link.child_hrid).display().to_string(),
                            |p| p.display().to_string()
                        ),
                        "kind": link.child_hrid.kind(),
                    },
                    "parent": {
                        "hrid": link.parent_hrid.display(digits).to_string(),
                        "title": parent_req.map(|r| r.title().to_string()),
                        "path": directory.path_for(&link.parent_hrid).map_or_else(
                            || directory.canonical_path_for(&link.parent_hrid).display().to_string(),
                            |p| p.display().to_string()
                        ),
                        "kind": link.parent_hrid.kind(),
                    },
                    "status": "fingerprint drift",
                    "stored_fingerprint": &link.stored_fingerprint,
                    "current_fingerprint": &link.current_fingerprint,
                })
            })
            .collect();

        let output = json!({
            "summary": {
                "total_count": suspect_links.len(),
                "unique_parents": unique_parents.len(),
                "unique_children": unique_children.len(),
                "by_kind": by_kind,
            },
            "links": links,
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn output_ndjson(
        suspect_links: &[requiem::SuspectLink],
        directory: &Directory,
        digits: usize,
    ) -> anyhow::Result<()> {
        use serde_json::json;

        for link in suspect_links {
            let child_req = directory.requirement_by_hrid(&link.child_hrid);
            let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

            let obj = json!({
                "child": {
                    "hrid": link.child_hrid.display(digits).to_string(),
                    "title": child_req.map(|r| r.title().to_string()),
                    "path": directory.path_for(&link.child_hrid).map_or_else(
                        || directory.canonical_path_for(&link.child_hrid).display().to_string(),
                        |p| p.display().to_string()
                    ),
                    "kind": link.child_hrid.kind(),
                },
                "parent": {
                    "hrid": link.parent_hrid.display(digits).to_string(),
                    "title": parent_req.map(|r| r.title().to_string()),
                    "path": directory.path_for(&link.parent_hrid).map_or_else(
                        || directory.canonical_path_for(&link.parent_hrid).display().to_string(),
                        |p| p.display().to_string()
                    ),
                    "kind": link.parent_hrid.kind(),
                },
                "status": "fingerprint drift",
                "stored_fingerprint": &link.stored_fingerprint,
                "current_fingerprint": &link.current_fingerprint,
            });
            println!("{}", serde_json::to_string(&obj)?);
        }
        Ok(())
    }

    fn output_table(
        &self,
        suspect_links: &[requiem::SuspectLink],
        directory: &Directory,
        digits: usize,
    ) {
        if self.detail {
            // Detailed block format
            for (i, link) in suspect_links.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                println!("{}", "━".repeat(70).dim());
                println!("Suspect Link #{} of {}", i + 1, suspect_links.len());
                println!();

                let child_req = directory.requirement_by_hrid(&link.child_hrid);
                let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

                let child_title = child_req.map(|r| r.title().to_string()).unwrap_or_default();
                let parent_title = parent_req
                    .map(|r| r.title().to_string())
                    .unwrap_or_default();

                println!(
                    "  CHILD:   {}  {}",
                    link.child_hrid.display(digits),
                    child_title
                );
                println!(
                    "           Path:     {}",
                    directory.path_for(&link.child_hrid).map_or_else(
                        || directory
                            .canonical_path_for(&link.child_hrid)
                            .display()
                            .to_string(),
                        |p| p.display().to_string()
                    )
                );
                println!();
                println!(
                    "  PARENT:  {}  {}",
                    link.parent_hrid.display(digits),
                    parent_title
                );
                println!(
                    "           Path:     {}",
                    directory.path_for(&link.parent_hrid).map_or_else(
                        || directory
                            .canonical_path_for(&link.parent_hrid)
                            .display()
                            .to_string(),
                        |p| p.display().to_string()
                    )
                );
                println!();
                println!("  REASON:  Parent content changed (fingerprint drift)");
                println!();
                println!("  STORED:  {}", link.stored_fingerprint);
                println!("  CURRENT: {}", link.current_fingerprint);
                println!();
                println!("  ACTIONS:");
                println!(
                    "    req accept {} {} --apply",
                    link.child_hrid.display(digits),
                    link.parent_hrid.display(digits)
                );
                println!("{}", "━".repeat(70).dim());
            }
        } else if matches!(self.group_by, Some(GroupBy::Parent | GroupBy::Child)) {
            self.output_grouped(suspect_links, directory, digits);
        } else {
            // Enhanced table format with titles
            println!("Suspect Links Found: {}", suspect_links.len());
            println!();
            println!(
                "{:<12} {} {:<12}     TITLE (CHILD → PARENT)",
                "CHILD",
                "→".dim(),
                "PARENT"
            );
            println!("{}", "─".repeat(70).dim());

            for link in suspect_links {
                let child_req = directory.requirement_by_hrid(&link.child_hrid);
                let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

                let child_title =
                    child_req.map_or_else(|| "(no title)".to_string(), |r| r.title().to_string());
                let parent_title =
                    parent_req.map_or_else(|| "(no title)".to_string(), |r| r.title().to_string());

                println!(
                    "{:<12} {} {:<12}     {} {} {}",
                    link.child_hrid.display(digits),
                    "→".dim(),
                    link.parent_hrid.display(digits),
                    child_title,
                    "→".dim(),
                    parent_title
                );
            }

            println!();
            println!(
                "{}",
                "Run 'req suspect --detail' for paths and fingerprints".dim()
            );
            println!(
                "{}",
                "Run 'req accept --all --apply' to accept all changes".dim()
            );
        }
    }

    fn output_grouped(
        &self,
        suspect_links: &[requiem::SuspectLink],
        directory: &Directory,
        digits: usize,
    ) {
        use std::collections::HashMap;

        match self.group_by {
            Some(GroupBy::Parent) => {
                let mut by_parent: HashMap<String, Vec<&requiem::SuspectLink>> = HashMap::new();
                for link in suspect_links {
                    by_parent
                        .entry(link.parent_hrid.display(digits).to_string())
                        .or_default()
                        .push(link);
                }

                println!(
                    "Suspect Links by Parent ({} parents, {} links total)",
                    by_parent.len(),
                    suspect_links.len()
                );
                println!();

                for (parent_hrid_str, links) in &by_parent {
                    let parent_req = directory.requirement_by_hrid(&links[0].parent_hrid);
                    let parent_title = parent_req
                        .map(|r| r.title().to_string())
                        .unwrap_or_default();

                    println!("{parent_hrid_str} ({parent_title})");
                    for (idx, link) in links.iter().enumerate() {
                        let child_req = directory.requirement_by_hrid(&link.child_hrid);
                        let child_title =
                            child_req.map(|r| r.title().to_string()).unwrap_or_default();

                        let prefix = if idx == links.len() - 1 {
                            "└─"
                        } else {
                            "├─"
                        };
                        println!(
                            "{}  {}  {}",
                            prefix,
                            link.child_hrid.display(digits),
                            child_title
                        );
                    }
                    println!();
                }
            }
            Some(GroupBy::Child) => {
                let mut by_child: HashMap<String, Vec<&requiem::SuspectLink>> = HashMap::new();
                for link in suspect_links {
                    by_child
                        .entry(link.child_hrid.display(digits).to_string())
                        .or_default()
                        .push(link);
                }

                println!(
                    "Suspect Links by Child ({} children, {} links total)",
                    by_child.len(),
                    suspect_links.len()
                );
                println!();

                for (child_hrid_str, links) in &by_child {
                    let child_req = directory.requirement_by_hrid(&links[0].child_hrid);
                    let child_title = child_req.map(|r| r.title().to_string()).unwrap_or_default();

                    println!("{child_hrid_str} ({child_title})");
                    for (idx, link) in links.iter().enumerate() {
                        let parent_req = directory.requirement_by_hrid(&link.parent_hrid);
                        let parent_title = parent_req
                            .map(|r| r.title().to_string())
                            .unwrap_or_default();

                        let prefix = if idx == links.len() - 1 {
                            "└─"
                        } else {
                            "├─"
                        };
                        println!(
                            "{}  {}  {}",
                            prefix,
                            link.parent_hrid.display(digits),
                            parent_title
                        );
                    }
                    println!();
                }
            }
            _ => {
                // This should be unreachable since we only call output_grouped
                // for Parent and Child variants
                unreachable!("output_grouped called with invalid group_by variant")
            }
        }
    }

    /// Handle accepting suspect links
    #[instrument]
    fn run_accept(self, path: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(path)?;
        let digits = directory.config().digits();
        let mut suspect_links = directory.suspect_links();

        // Apply filters (same as display mode)
        if let Some(ref child_filter) = self.child {
            suspect_links.retain(|link| &link.child_hrid == child_filter);
        }
        if let Some(ref parent_filter) = self.parent {
            suspect_links.retain(|link| &link.parent_hrid == parent_filter);
        }
        if let Some(ref kind_filter) = self.kind {
            let kind_upper = kind_filter.to_uppercase();
            suspect_links.retain(|link| link.child_hrid.kind() == kind_upper);
        }

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
                use std::io::{self, BufRead};

                println!("Will accept {count} suspect links across {file_count} files:");
                for link in &suspect_links {
                    println!(
                        "  {} ← {}",
                        link.child_hrid.display(digits),
                        link.parent_hrid.display(digits)
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

                    eprint!("\nAccept this link? (y/N) ");
                    let stdin = std::io::stdin();
                    let mut input = String::new();
                    stdin.lock().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Cancelled");
                        std::process::exit(130);
                    }
                }
            }

            match directory.accept_suspect_link(child.clone(), parent.clone())? {
                requiem::AcceptResult::Updated => {
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
                requiem::AcceptResult::AlreadyUpToDate => {
                    println!("No changes: link already up-to-date.");
                }
            }
        }

        Ok(())
    }
}
