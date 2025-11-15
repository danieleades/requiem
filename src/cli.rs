use std::path::{Path, PathBuf};

mod list;
mod status;
mod terminal;

use clap::ArgAction;
use list::List;
use requiem::{Directory, Hrid};
use status::Status;
use tracing::instrument;

/// Parse an HRID from a string, normalizing to uppercase.
///
/// This is a CLI boundary function that accepts lowercase input
/// and normalizes it before parsing.
fn parse_hrid(s: &str) -> Result<Hrid, String> {
    // Normalize to uppercase
    let uppercase = s.to_uppercase();
    // Parse using FromStr (strict validation)
    uppercase.parse().map_err(|e| format!("{e}"))
}

#[derive(Debug, clap::Parser)]
#[command(version, about)]
pub struct Cli {
    /// Verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = ArgAction::Count, global=true)]
    verbose: u8,

    /// The path to the root of the requirements directory
    #[arg(short, long, default_value = ".", global = true)]
    root: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        Self::setup_logging(self.verbose);

        self.command
            .unwrap_or_else(|| Command::Status(Status::default()))
            .run(self.root)
    }

    fn setup_logging(verbosity: u8) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let level = match verbosity {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        };

        let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive(level.into());

        let fmt_layer = tracing_subscriber::fmt::layer()
            //.pretty()
            .with_target(false)
            .with_thread_names(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// Show repository status (default)
    Status(Status),

    /// Initialize a new requirements repository
    Init,

    /// Add a new requirement
    Add(Add),

    /// Create a link between two requirements
    ///
    /// Links are parent-child relationships.
    Link(Link),

    /// Correct parent HRIDs
    Clean,

    /// List all suspect links
    ///
    /// Suspect links are those where the parent requirement has changed
    /// since the link was created or last reviewed.
    Suspect(Suspect),

    /// Check for suspect links (alias for suspect)
    ///
    /// Suspect links are those where the parent requirement has changed
    /// since the link was created or last reviewed.
    Check(Suspect),

    /// Accept suspect links after review
    ///
    /// Updates fingerprints to mark requirements as reviewed and valid.
    Accept(Accept),

    /// List requirements with filters and relationship views
    List(List),

    /// Show or modify configuration settings
    Config(Config),

    /// Diagnose path-related issues
    Diagnose(Diagnose),
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Status(command) => command.run(root)?,
            Self::Init => Init::run(&root)?,
            Self::Add(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Clean => Clean::run(root)?,
            Self::Suspect(command) | Self::Check(command) => command.run(root)?,
            Self::Accept(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
            Self::Config(command) => command.run(&root)?,
            Self::Diagnose(command) => command.run(&root)?,
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Init {}

impl Init {
    #[instrument]
    fn run(root: &PathBuf) -> anyhow::Result<()> {
        use std::fs;

        // Create .req directory
        let req_dir = root.join(".req");
        if req_dir.exists() {
            anyhow::bail!("Repository already initialized (found existing .req directory)");
        }

        fs::create_dir_all(&req_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create .req directory: {e}"))?;

        // Create config.toml with defaults
        let config_path = req_dir.join("config.toml");
        let config = requiem::Config::default();
        config
            .save(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to create config.toml: {e}"))?;

        // Create templates directory
        let templates_dir = req_dir.join("templates");
        fs::create_dir_all(&templates_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create templates directory: {e}"))?;

        // Create example templates
        let usr_template = templates_dir.join("USR.md");
        fs::write(
            &usr_template,
            "## Statement\n\nThe system shall [describe what must be accomplished from user \
             perspective].\n\n## Rationale\n\n[Explain why this requirement exists]\n\n## \
             Acceptance Criteria\n\n- [Criterion 1: Specific, measurable condition that must be \
             met]\n- [Criterion 2: Observable behavior or outcome]\n",
        )
        .map_err(|e| anyhow::anyhow!("Failed to create USR template: {e}"))?;

        let sys_template = templates_dir.join("SYS.md");
        fs::write(
            &sys_template,
            "## Description\n\n[Describe the system-level requirement or implementation \
             approach]\n\n## Technical Details\n\n[Technical specifications, constraints, or \
             implementation notes]\n",
        )
        .map_err(|e| anyhow::anyhow!("Failed to create SYS template: {e}"))?;

        println!("Initialized requirements repository in {}", root.display());
        println!("  Created: .req/config.toml");
        println!("  Created: .req/templates/USR.md");
        println!("  Created: .req/templates/SYS.md");
        println!();
        println!("Next steps:");
        println!("  req add USR --title \"Your First Requirement\"");

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Add {
    /// The kind of requirement to create, optionally with namespace.
    ///
    /// Accepts a dash-separated list where the last token is the kind
    /// and any preceding tokens form the namespace.
    ///
    /// Examples:
    /// - 'USR' creates a requirement with kind USR and no namespace
    /// - 'SYSTEM-AUTH-USR' creates a requirement with namespace [SYSTEM, AUTH]
    ///   and kind USR
    kind: String,

    /// The human-readable IDs of the parent requirements.
    #[clap(long, short, value_delimiter = ',', value_parser = parse_hrid)]
    parent: Vec<Hrid>,

    /// The title of the requirement (will be formatted as a markdown heading).
    #[clap(long, short)]
    title: Option<String>,

    /// The body text of the requirement.
    #[clap(long, short)]
    body: Option<String>,
}

impl Add {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Construct content from title and body
        let content = match (&self.title, &self.body) {
            (Some(title), Some(body)) => format!("# {title}\n\n{body}"),
            (Some(title), None) => format!("# {title}"),
            (None, Some(body)) => body.clone(),
            (None, None) => String::new(),
        };

        // Parse kind string as dash-separated tokens (CLI boundary - normalize to
        // uppercase)
        let tokens: nonempty::NonEmpty<String> = {
            let parts: Vec<String> = self
                .kind
                .split('-')
                .map(|s| s.trim().to_uppercase())
                .collect();
            nonempty::NonEmpty::from_vec(parts)
                .ok_or_else(|| anyhow::anyhow!("kind must contain at least one token"))?
        };

        // Last token is the kind, everything before is the namespace
        let (namespace, kind) = {
            let mut parts: Vec<String> = tokens.into();
            let kind = parts.pop().expect("nonempty has at least one element");
            (parts, kind)
        };

        let requirement = if namespace.is_empty() {
            directory.add_requirement(&kind, content)?
        } else {
            directory.add_requirement_with_namespace(namespace, &kind, content)?
        };

        for parent in &self.parent {
            // TODO: the linkage should be done before the requirement is saved by the
            // 'add_requirement' method to avoid unnecessary IO.
            directory.link_requirement(requirement.hrid(), parent)?;
        }
        directory.flush()?;

        println!("Added requirement {}", requirement.hrid().display(digits));
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Link {
    /// The human-readable ID of the child document
    #[clap(value_parser = parse_hrid)]
    child: Hrid,

    /// The human-readable ID of the parent document
    #[clap(value_parser = parse_hrid)]
    parent: Hrid,
}

impl Link {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();
        let child = &self.child;
        let parent = &self.parent;
        let msg = format!(
            "Linked {} to {}",
            child.display(digits),
            parent.display(digits)
        );

        directory.link_requirement(&self.child, &self.parent)?;
        directory.flush()?;

        println!("{msg}");

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Clean {}

impl Clean {
    #[instrument]
    fn run(path: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(path)?;
        directory.update_hrids();
        directory.flush()?;
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Suspect {
    /// Show detailed information including fingerprints and paths
    #[arg(long)]
    detail: bool,

    /// Output format (table, json, ndjson)
    #[arg(long, value_name = "FORMAT", default_value = "table")]
    format: SuspectFormat,

    /// Show summary statistics
    #[arg(long)]
    stats: bool,

    /// Quiet mode: output only CHILD PARENT pairs (no headers, no colors)
    #[arg(long, short, conflicts_with = "detail", conflicts_with = "stats")]
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
    #[arg(long, value_name = "FIELD")]
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

impl Suspect {
    #[instrument]
    fn run(self, path: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

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
        use terminal::Colorize;

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
}

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Accept {
    /// Accept all suspect links
    #[arg(long)]
    all: bool,

    /// Apply changes (write to disk). Without this flag, shows preview only.
    #[arg(long, conflicts_with = "dry_run")]
    apply: bool,

    /// Preview changes without writing (default for --all)
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, alias = "force")]
    yes: bool,

    /// Accept a specific link from child to parent
    #[arg(value_name = "CHILD", required_unless_present = "all", value_parser = parse_hrid)]
    child: Option<Hrid>,

    /// Parent requirement HRID
    #[arg(value_name = "PARENT", required_unless_present = "all", value_parser = parse_hrid)]
    parent: Option<Hrid>,
}

impl Accept {
    #[instrument]
    #[allow(clippy::too_many_lines)]
    fn run(self, path: PathBuf) -> anyhow::Result<()> {
        use dialoguer::Confirm;
        use terminal::Colorize;

        let mut directory = Directory::new(path)?;
        let digits = directory.config().digits();

        if self.all {
            let suspect_links = directory.suspect_links();

            if suspect_links.is_empty() {
                println!("Nothing to update. All suspect links are already accepted.");
                return Ok(());
            }

            let count = suspect_links.len();

            // Count unique files (children that have suspect links)
            let mut files = std::collections::HashSet::new();
            for link in &suspect_links {
                files.insert(link.child_hrid.display(digits).to_string());
            }
            let file_count = files.len();

            // Determine if we're in dry-run or apply mode
            // Default is dry-run for --all unless --apply is specified
            let is_dry_run = !self.apply;

            if is_dry_run {
                // Dry-run mode: show preview
                println!("Pending updates: {count} suspect links");
                println!("\nPreview:");
                for link in &suspect_links {
                    println!(
                        "  {} ← {}",
                        link.child_hrid.display(digits),
                        link.parent_hrid.display(digits)
                    );
                }
                println!("\n{}", "Use --apply to write changes.".dim());
                std::process::exit(2);
            }

            // Apply mode: confirm and execute
            if !self.yes {
                let prompt = format!(
                    "Apply updates to {count} suspect links across {file_count} files? (y/N)"
                );

                let confirmed = Confirm::new()
                    .with_prompt(prompt)
                    .default(false)
                    .interact()?;

                if !confirmed {
                    println!("Cancelled.");
                    std::process::exit(130);
                }
            }

            // Apply the updates
            let start = std::time::Instant::now();
            for link in &suspect_links {
                println!(
                    "Updating {} ← {}",
                    link.child_hrid.display(digits),
                    link.parent_hrid.display(digits)
                );
            }

            let updated = directory.accept_all_suspect_links();
            directory.flush()?;
            let duration = start.elapsed();

            println!(
                "\n{} | Links updated: {} | Files touched: {} | Duration: {:.1}s",
                "Complete".success(),
                updated.len(),
                file_count,
                duration.as_secs_f64()
            );
        } else {
            let child = self.child.expect("child is required when --all is not set");
            let parent = self
                .parent
                .expect("parent is required when --all is not set");

            // For single link, check if it's suspect
            let suspect_links = directory.suspect_links();
            let link = suspect_links
                .iter()
                .find(|l| l.child_hrid == child && l.parent_hrid == parent);

            if let Some(link) = link {
                // Show confirmation banner if link is suspect
                if !self.yes {
                    println!(
                        "Reviewing: {} → {}",
                        child.display(digits),
                        parent.display(digits)
                    );
                    println!("Stored:    {}", link.stored_fingerprint);
                    println!("Current:   {}", link.current_fingerprint);

                    let confirmed = Confirm::new()
                        .with_prompt("Accept this link? (y/N)")
                        .default(false)
                        .interact()?;

                    if !confirmed {
                        println!("Cancelled.");
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
                            "Accepted {} ← {}",
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

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, clap::Parser)]
enum ConfigCommand {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key to set
        key: String,

        /// Value to set
        value: String,
    },
}

impl Config {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        use terminal::Colorize;

        let config_path = root.join(".req/config.toml");

        match self.command {
            ConfigCommand::Show => {
                let config = if config_path.exists() {
                    requiem::Config::load(&config_path).map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    requiem::Config::default()
                };

                println!("Configuration:");
                println!(
                    "  subfolders_are_namespaces: {} ({})",
                    config.subfolders_are_namespaces,
                    if config.subfolders_are_namespaces {
                        "path mode".dim()
                    } else {
                        "filename mode".dim()
                    }
                );
                println!("  digits: {}", config.digits());
                println!("  allow_unrecognised: {}", config.allow_unrecognised);
                if !config.allowed_kinds().is_empty() {
                    println!("  allowed_kinds: {:?}", config.allowed_kinds());
                }
            }
            ConfigCommand::Set { key, value } => {
                let mut config = if config_path.exists() {
                    requiem::Config::load(&config_path).map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    requiem::Config::default()
                };

                match key.as_str() {
                    "subfolders_are_namespaces" => {
                        let bool_value = value
                            .parse::<bool>()
                            .map_err(|_| anyhow::anyhow!("Value must be 'true' or 'false'"))?;

                        config.set_subfolders_are_namespaces(bool_value);
                        config
                            .save(&config_path)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;

                        println!(
                            "{}",
                            format!(
                                "Directory mode: {}",
                                if bool_value {
                                    "path-based"
                                } else {
                                    "filename-based"
                                }
                            )
                            .success()
                        );

                        if bool_value {
                            println!("\n{}", "Path-based mode:".info());
                            println!(
                                "  • Filenames inside namespace folders should contain KIND-ID \
                                 (e.g., USR/003.md)."
                            );
                            println!(
                                "  • You will need to manually reorganize existing files to match \
                                 the new structure."
                            );
                        } else {
                            println!("\n{}", "Filename-based mode:".info());
                            println!("  • Namespaces will no longer be inferred from folders.");
                            println!(
                                "  • Full HRID must be in filename (e.g., system-auth-USR-003.md)."
                            );
                        }

                        println!(
                            "\n{}",
                            "See docs/src/requirements/SPC-004.md for migration guide"
                                .to_string()
                                .dim()
                        );
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unknown configuration key: '{key}'\nSupported keys: \
                             subfolders_are_namespaces",
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Diagnose {
    #[command(subcommand)]
    command: DiagnoseCommand,
}

#[derive(Debug, clap::Parser)]
enum DiagnoseCommand {
    /// Diagnose path-related issues
    Paths,
}

impl Diagnose {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        use terminal::Colorize;

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

#[cfg(test)]
mod tests {
    use requiem::{Directory, RequirementView};
    use tempfile::tempdir;

    use super::*;

    fn collect_child<'a>(directory: &'a Directory, kind: &'a str) -> RequirementView<'a> {
        directory
            .requirements()
            .find(|req| req.hrid.kind() == kind)
            .expect("expected requirement for kind")
    }

    #[test]
    fn add_run_creates_requirement_and_links_parents() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush parent requirement");

        let add = Add {
            kind: "USR".to_string(),
            parent: vec![parent.hrid().clone()],
            title: Some("Child".to_string()),
            body: Some("body text".to_string()),
        };

        add.run(root.clone()).expect("add command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let child = collect_child(&directory, "USR");

        assert!(child
            .parents
            .iter()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
        assert_eq!(child.title, "Child");
        assert_eq!(child.body, "body text");
    }

    #[test]
    fn add_run_uses_template_when_no_content_provided() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let template_dir = root.join(".req").join("templates");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("USR.md"), "## Template body").unwrap();

        let add = Add {
            kind: "USR".to_string(),
            parent: Vec::new(),
            title: None,
            body: None,
        };

        add.run(root.clone()).expect("add command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let child = collect_child(&directory, "USR");
        assert_eq!(child.body, "## Template body");
    }

    #[test]
    fn add_run_creates_namespaced_requirement() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let add = Add {
            kind: "SYSTEM-AUTH-USR".to_string(),
            parent: Vec::new(),
            title: Some("Namespaced Requirement".to_string()),
            body: Some("test body".to_string()),
        };

        add.run(root.clone()).expect("add command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");

        // Find the requirement by kind USR with namespace SYSTEM-AUTH
        let requirements: Vec<_> = directory
            .requirements()
            .filter(|r| r.hrid.kind() == "USR" && r.hrid.namespace() == ["SYSTEM", "AUTH"])
            .map(|view| view.to_requirement())
            .collect();

        assert_eq!(requirements.len(), 1);
        let req = &requirements[0];

        // Verify namespace and kind
        assert_eq!(req.hrid().namespace(), &["SYSTEM", "AUTH"]);
        assert_eq!(req.hrid().kind(), "USR");
        assert_eq!(req.title(), "Namespaced Requirement");
    }

    #[test]
    fn link_run_updates_child_parent_relationship() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR", "# Child".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush initial requirements");

        let link = Link {
            child: child.hrid().clone(),
            parent: parent.hrid().clone(),
        };

        link.run(root.clone()).expect("link command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let reloaded_child = collect_child(&directory, "USR");
        assert!(reloaded_child
            .parents
            .iter()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
    }

    #[test]
    fn clean_run_succeeds_on_empty_directory() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        Clean::run(root).expect("clean should succeed on empty directory");
    }

    #[test]
    fn suspect_run_exits_early_when_no_suspect_links() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let suspect = Suspect {
            detail: false,
            format: SuspectFormat::default(),
            stats: false,
            quiet: false,
            child: None,
            parent: None,
            kind: None,
            group_by: None,
        };

        suspect
            .run(root)
            .expect("suspect should succeed when no links");
    }

    #[test]
    fn accept_run_all_reports_when_no_links_found() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let accept = Accept {
            all: true,
            apply: true,
            dry_run: false,
            yes: true,
            child: None,
            parent: None,
        };

        accept
            .run(root)
            .expect("accept --all should succeed with no suspect links");
    }

    #[test]
    fn accept_run_handles_already_up_to_date_link() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR", "# Child".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush initial requirements");

        let mut directory = Directory::new(root.clone()).unwrap();
        directory
            .link_requirement(child.hrid(), parent.hrid())
            .unwrap();
        directory.flush().unwrap();

        let accept = Accept {
            all: false,
            apply: true,
            dry_run: false,
            yes: true,
            child: Some(child.hrid().clone()),
            parent: Some(parent.hrid().clone()),
        };

        accept
            .run(root)
            .expect("accept should treat up-to-date link as success");
    }

    #[test]
    fn status_run_reports_counts_without_exit() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR", "# Child".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush initial requirements");

        // Create a parent-child relationship to ensure we exercise counting logic.
        let mut directory = Directory::new(root.clone()).unwrap();
        directory
            .link_requirement(child.hrid(), parent.hrid())
            .unwrap();
        directory.flush().unwrap();

        Status::default()
            .run(root)
            .expect("status should succeed when no suspect links exist");
    }
}
