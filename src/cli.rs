use std::path::{Path, PathBuf};

mod list;
mod status;
mod terminal;

use clap::ArgAction;
use list::List;
use requiem::{Directory, Hrid};
use status::Status;
use tracing::instrument;

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
            Self::Add(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Clean => Clean::run(root)?,
            Self::Suspect(command) => command.run(root)?,
            Self::Accept(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
            Self::Config(command) => command.run(&root)?,
            Self::Diagnose(command) => command.run(&root)?,
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Add {
    /// The kind of requirement to create.
    ///
    /// eg. 'USR' or 'SYS'.
    kind: String,

    /// The human-readable IDs of the parent requirements.
    #[clap(long, short, value_delimiter = ',')]
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

        // Construct content from title and body
        let content = match (&self.title, &self.body) {
            (Some(title), Some(body)) => format!("# {title}\n\n{body}"),
            (Some(title), None) => format!("# {title}"),
            (None, Some(body)) => body.clone(),
            (None, None) => String::new(),
        };

        let requirement = directory.add_requirement(self.kind, content)?;

        for parent in self.parent {
            // TODO: the linkage should be done before the requirement is saved by the
            // 'add_requirement' method to avoid unnecessary IO.
            directory.link_requirement(requirement.hrid().clone(), parent)?;
        }

        println!("Added requirement {}", requirement.hrid());
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Link {
    /// The human-readable ID of the child document
    child: Hrid,

    /// The human-readable ID of the parent document
    parent: Hrid,
}

impl Link {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root)?;
        let child = &self.child;
        let parent = &self.parent;
        let msg = format!("Linked {child} to {parent}");

        directory.link_requirement(self.child, self.parent)?;

        println!("{msg}");

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Clean {}

impl Clean {
    #[instrument]
    fn run(path: PathBuf) -> anyhow::Result<()> {
        Directory::new(path)?.update_hrids()?;
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
    #[arg(long)]
    child: Option<Hrid>,

    /// Filter by parent requirement HRID
    #[arg(long)]
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
        let mut suspect_links = directory.suspect_links();

        // Apply filters
        if let Some(ref child_filter) = self.child {
            suspect_links.retain(|link| &link.child_hrid == child_filter);
        }
        if let Some(ref parent_filter) = self.parent {
            suspect_links.retain(|link| &link.parent_hrid == parent_filter);
        }
        if let Some(ref kind_filter) = self.kind {
            let kind_lower = kind_filter.to_ascii_lowercase();
            suspect_links.retain(|link| link.child_hrid.kind().to_ascii_lowercase() == kind_lower);
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
                println!("{} {}", link.child_hrid, link.parent_hrid);
            }
            std::process::exit(2);
        }

        // Show stats if requested
        if self.stats {
            Self::output_stats(&suspect_links, &directory);
            println!();
        }

        match self.format {
            SuspectFormat::Json => {
                Self::output_json(&suspect_links, &directory)?;
            }
            SuspectFormat::Ndjson => {
                Self::output_ndjson(&suspect_links, &directory)?;
            }
            SuspectFormat::Table => {
                self.output_table(&suspect_links, &directory)?;
            }
        }

        // Exit with code 2 to indicate suspect links exist (for CI)
        std::process::exit(2);
    }

    fn extract_title(content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(stripped) = trimmed.strip_prefix("# ") {
                return Some(stripped.trim().to_string());
            }

            if let Some(stripped) = trimmed.strip_prefix("## ") {
                return Some(stripped.trim().to_string());
            }

            if let Some(stripped) = trimmed.strip_prefix("### ") {
                return Some(stripped.trim().to_string());
            }

            return Some(trimmed.to_string());
        }
        None
    }

    fn output_stats(suspect_links: &[requiem::storage::SuspectLink], _directory: &Directory) {
        use std::collections::{HashMap, HashSet};

        let unique_parents: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.parent_hrid.to_string())
            .collect();
        let unique_children: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.child_hrid.to_string())
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
                .entry(link.parent_hrid.to_string())
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
        suspect_links: &[requiem::storage::SuspectLink],
        directory: &Directory,
    ) -> anyhow::Result<()> {
        use std::collections::{HashMap, HashSet};

        use serde_json::json;

        let unique_parents: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.parent_hrid.to_string())
            .collect();
        let unique_children: HashSet<_> = suspect_links
            .iter()
            .map(|l| l.child_hrid.to_string())
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
                        "hrid": link.child_hrid.to_string(),
                        "title": child_req.and_then(|r| Self::extract_title(r.content())),
                        "path": directory.path_for(&link.child_hrid).display().to_string(),
                        "kind": link.child_hrid.kind(),
                    },
                    "parent": {
                        "hrid": link.parent_hrid.to_string(),
                        "title": parent_req.and_then(|r| Self::extract_title(r.content())),
                        "path": directory.path_for(&link.parent_hrid).display().to_string(),
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
        suspect_links: &[requiem::storage::SuspectLink],
        directory: &Directory,
    ) -> anyhow::Result<()> {
        use serde_json::json;

        for link in suspect_links {
            let child_req = directory.requirement_by_hrid(&link.child_hrid);
            let parent_req = directory.requirement_by_hrid(&link.parent_hrid);

            let obj = json!({
                "child": {
                    "hrid": link.child_hrid.to_string(),
                    "title": child_req.and_then(|r| Self::extract_title(r.content())),
                    "path": directory.path_for(&link.child_hrid).display().to_string(),
                    "kind": link.child_hrid.kind(),
                },
                "parent": {
                    "hrid": link.parent_hrid.to_string(),
                    "title": parent_req.and_then(|r| Self::extract_title(r.content())),
                    "path": directory.path_for(&link.parent_hrid).display().to_string(),
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
        suspect_links: &[requiem::storage::SuspectLink],
        directory: &Directory,
    ) -> anyhow::Result<()> {
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

                let child_title = child_req
                    .and_then(|r| Self::extract_title(r.content()))
                    .unwrap_or_default();
                let parent_title = parent_req
                    .and_then(|r| Self::extract_title(r.content()))
                    .unwrap_or_default();

                println!("  CHILD:   {}  {}", link.child_hrid, child_title);
                println!(
                    "           Path:     {}",
                    directory.path_for(&link.child_hrid).display()
                );
                println!();
                println!("  PARENT:  {}  {}", link.parent_hrid, parent_title);
                println!(
                    "           Path:     {}",
                    directory.path_for(&link.parent_hrid).display()
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
                    link.child_hrid, link.parent_hrid
                );
                println!("{}", "━".repeat(70).dim());
            }
        } else if self.group_by.is_some() {
            self.output_grouped(suspect_links, directory)?;
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

                let child_title = child_req
                    .and_then(|r| Self::extract_title(r.content()))
                    .unwrap_or_else(|| String::from("(no title)"));
                let parent_title = parent_req
                    .and_then(|r| Self::extract_title(r.content()))
                    .unwrap_or_else(|| String::from("(no title)"));

                println!(
                    "{:<12} {} {:<12}     {} {} {}",
                    link.child_hrid,
                    "→".dim(),
                    link.parent_hrid,
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

        Ok(())
    }

    fn output_grouped(
        &self,
        suspect_links: &[requiem::storage::SuspectLink],
        directory: &Directory,
    ) -> anyhow::Result<()> {
        use std::collections::HashMap;

        match self.group_by {
            Some(GroupBy::Parent) => {
                let mut by_parent: HashMap<String, Vec<&requiem::storage::SuspectLink>> =
                    HashMap::new();
                for link in suspect_links {
                    by_parent
                        .entry(link.parent_hrid.to_string())
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
                        .and_then(|r| Self::extract_title(r.content()))
                        .unwrap_or_default();

                    println!("{parent_hrid_str} ({parent_title})");
                    for (idx, link) in links.iter().enumerate() {
                        let child_req = directory.requirement_by_hrid(&link.child_hrid);
                        let child_title = child_req
                            .and_then(|r| Self::extract_title(r.content()))
                            .unwrap_or_default();

                        let prefix = if idx == links.len() - 1 {
                            "└─"
                        } else {
                            "├─"
                        };
                        println!("{}  {}  {}", prefix, link.child_hrid, child_title);
                    }
                    println!();
                }
            }
            Some(GroupBy::Child) => {
                let mut by_child: HashMap<String, Vec<&requiem::storage::SuspectLink>> =
                    HashMap::new();
                for link in suspect_links {
                    by_child
                        .entry(link.child_hrid.to_string())
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
                    let child_title = child_req
                        .and_then(|r| Self::extract_title(r.content()))
                        .unwrap_or_default();

                    println!("{child_hrid_str} ({child_title})");
                    for (idx, link) in links.iter().enumerate() {
                        let parent_req = directory.requirement_by_hrid(&link.parent_hrid);
                        let parent_title = parent_req
                            .and_then(|r| Self::extract_title(r.content()))
                            .unwrap_or_default();

                        let prefix = if idx == links.len() - 1 {
                            "└─"
                        } else {
                            "├─"
                        };
                        println!("{}  {}  {}", prefix, link.parent_hrid, parent_title);
                    }
                    println!();
                }
            }
            _ => {
                // Fallback to normal table
                return self.output_table(suspect_links, directory);
            }
        }

        Ok(())
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
    #[arg(value_name = "CHILD", required_unless_present = "all")]
    child: Option<Hrid>,

    /// Parent requirement HRID
    #[arg(value_name = "PARENT", required_unless_present = "all")]
    parent: Option<Hrid>,
}

impl Accept {
    #[instrument]
    fn run(self, path: PathBuf) -> anyhow::Result<()> {
        use dialoguer::Confirm;
        use terminal::Colorize;

        let mut directory = Directory::new(path)?;

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
                files.insert(link.child_hrid.to_string());
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
                    println!("  {} ← {}", link.child_hrid, link.parent_hrid);
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
                println!("Updating {} ← {}", link.child_hrid, link.parent_hrid);
            }

            let updated = directory.accept_all_suspect_links()?;
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
                    println!("Reviewing: {child} → {parent}");
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
                requiem::storage::AcceptResult::Updated => {
                    println!("{}", format!("Accepted {child} ← {parent}").success());
                }
                requiem::storage::AcceptResult::AlreadyUpToDate => {
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

        let config_path = root.join("config.toml");

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
                println!("  allow_invalid: {}", config.allow_invalid);
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
                let config_path = root.join("config.toml");
                let config = if config_path.exists() {
                    requiem::Config::load(&config_path).map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    requiem::Config::default()
                };

                let directory = Directory::new(root.to_path_buf())?;
                let mut issues: Vec<String> = Vec::new();

                for req in directory.requirements() {
                    let current_path = directory.path_for(req.hrid());

                    if config.subfolders_are_namespaces {
                        let expected_path =
                            compute_path_based_location(root, req.hrid(), config.digits());

                        if current_path != expected_path {
                            let hrid = req.hrid();
                            let expected_display = expected_path
                                .strip_prefix(root)
                                .unwrap_or(&expected_path)
                                .display();
                            let current_display = current_path
                                .strip_prefix(root)
                                .unwrap_or(&current_path)
                                .display();
                            issues.push(format!(
                                "{hrid}: Expected '{expected_display}', found '{current_display}'"
                            ));
                        }
                    } else {
                        // In filename mode, check that HRID is fully in filename
                        if let Some(filename) = current_path.file_name() {
                            let filename_str = filename.to_string_lossy();
                            if !filename_str.contains(req.hrid().kind()) {
                                let hrid = req.hrid();
                                issues.push(format!(
                                    "{hrid}: Filename '{filename_str}' should contain full HRID"
                                ));
                            }
                        }
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

fn compute_path_based_location(root: &Path, hrid: &requiem::Hrid, digits: usize) -> PathBuf {
    let mut path = root.to_path_buf();

    // Add namespace folders
    for segment in hrid.namespace() {
        path.push(segment);
    }

    // Add KIND folder
    path.push(hrid.kind());

    // Add ID as filename
    let id_str = format!("{:0width$}.md", hrid.id(), width = digits);
    path.push(id_str);

    path
}

#[cfg(test)]
mod tests {
    use requiem::{Directory, Requirement};
    use tempfile::tempdir;

    use super::*;

    fn collect_child<'a>(directory: &'a Directory, kind: &str) -> &'a Requirement {
        directory
            .requirements()
            .find(|req| req.hrid().kind() == kind)
            .expect("expected requirement for kind")
    }

    #[test]
    fn add_run_creates_requirement_and_links_parents() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();

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
            .parents()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
        assert_eq!(child.content(), "# Child\n\nbody text");
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
        assert_eq!(child.content(), "## Template body");
    }

    #[test]
    fn link_run_updates_child_parent_relationship() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR".to_string(), "# Child".to_string())
            .unwrap();

        let link = Link {
            child: child.hrid().clone(),
            parent: parent.hrid().clone(),
        };

        link.run(root.clone()).expect("link command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let reloaded_child = collect_child(&directory, "USR");
        assert!(reloaded_child
            .parents()
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
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR".to_string(), "# Child".to_string())
            .unwrap();

        Directory::new(root.clone())
            .unwrap()
            .link_requirement(child.hrid().clone(), parent.hrid().clone())
            .unwrap();

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
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR".to_string(), "# Child".to_string())
            .unwrap();

        // Create a parent-child relationship to ensure we exercise counting logic.
        Directory::new(root.clone())
            .unwrap()
            .link_requirement(child.hrid().clone(), parent.hrid().clone())
            .unwrap();

        Status::default()
            .run(root)
            .expect("status should succeed when no suspect links exist");
    }
}
