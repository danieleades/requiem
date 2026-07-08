//! Output formatting for suspect links (table, detail, grouped, JSON).

use requiem_core::{Directory, SuspectLink};

use super::{display_path, Command, GroupBy};
use crate::cli::terminal::Colorize;

/// Serialize one suspect link for the JSON and NDJSON formats.
fn link_json(link: &SuspectLink, directory: &Directory, digits: usize) -> serde_json::Value {
    use serde_json::json;

    let child_req = directory.requirement_by_hrid(&link.child_hrid);
    let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

    json!({
        "child": {
            "hrid": link.child_hrid.display(digits).to_string(),
            "title": child_req.map(|r| r.title().to_string()),
            "path": display_path(directory, &link.child_hrid),
            "kind": link.child_hrid.kind(),
        },
        "parent": {
            "hrid": link.parent_hrid.display(digits).to_string(),
            "title": parent_req.map(|r| r.title().to_string()),
            "path": display_path(directory, &link.parent_hrid),
            "kind": link.parent_hrid.kind(),
        },
        "status": "fingerprint drift",
        "stored_fingerprint": &link.stored_fingerprint,
        "current_fingerprint": &link.current_fingerprint,
    })
}

impl Command {
    pub(super) fn output_stats(suspect_links: &[SuspectLink], digits: usize) {
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
            // Sort by HRID as a tiebreaker so equal counts print stably.
            parent_list.sort_by_key(|(hrid, count)| (std::cmp::Reverse(*count), (*hrid).clone()));

            println!("Most affected parents:");
            for (hrid, count) in parent_list.iter().take(3) {
                println!("  {hrid}  ({count} children)");
            }
        }
    }

    pub(super) fn output_json(
        suspect_links: &[SuspectLink],
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
            .map(|link| link_json(link, directory, digits))
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

    pub(super) fn output_ndjson(
        suspect_links: &[SuspectLink],
        directory: &Directory,
        digits: usize,
    ) -> anyhow::Result<()> {
        for link in suspect_links {
            let obj = link_json(link, directory, digits);
            println!("{}", serde_json::to_string(&obj)?);
        }
        Ok(())
    }

    pub(super) fn output_table(
        &self,
        suspect_links: &[SuspectLink],
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
                    display_path(directory, &link.child_hrid)
                );
                println!();
                println!(
                    "  PARENT:  {}  {}",
                    link.parent_hrid.display(digits),
                    parent_title
                );
                println!(
                    "           Path:     {}",
                    display_path(directory, &link.parent_hrid)
                );
                println!();
                println!("  REASON:  Parent content changed (fingerprint drift)");
                println!();
                println!("  STORED:  {}", link.stored_fingerprint);
                println!("  CURRENT: {}", link.current_fingerprint);
                println!();
                println!("  ACTIONS:");
                println!(
                    "    req review --accept --child {} --parent {}",
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
                "Run 'req review --detail' for paths and fingerprints".dim()
            );
            println!(
                "{}",
                "Run 'req review --accept --all --yes' to accept all changes".dim()
            );
        }
    }

    fn output_grouped(&self, suspect_links: &[SuspectLink], directory: &Directory, digits: usize) {
        use std::collections::BTreeMap;

        match self.group_by {
            Some(GroupBy::Parent) => {
                // BTreeMap keeps group output stable across runs.
                let mut by_parent: BTreeMap<String, Vec<&SuspectLink>> = BTreeMap::new();
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
                let mut by_child: BTreeMap<String, Vec<&SuspectLink>> = BTreeMap::new();
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
}
