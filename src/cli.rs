use std::path::PathBuf;

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
            .unwrap_or(Command::Status(Status::default()))
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
            Self::Config(command) => command.run(root)?,
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
        let mut directory = Directory::new(root).load_all()?;

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
        let directory = Directory::new(root);
        let msg = format!("Linked {} to {}", self.child, self.parent);

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
        Directory::new(path).load_all()?.update_hrids()?;
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
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum SuspectFormat {
    #[default]
    Table,
    Json,
    Ndjson,
}

impl Suspect {
    #[instrument]
    fn run(self, path: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let directory = Directory::new(path).load_all()?;
        let suspect_links = directory.suspect_links();

        if suspect_links.is_empty() {
            println!("{}", "✅ No suspect links detected.".success());
            return Ok(());
        }

        match self.format {
            SuspectFormat::Json => {
                self.output_json(&suspect_links)?;
            }
            SuspectFormat::Ndjson => {
                self.output_ndjson(&suspect_links)?;
            }
            SuspectFormat::Table => {
                self.output_table(&suspect_links)?;
            }
        }

        // Exit with code 2 to indicate suspect links exist (for CI)
        std::process::exit(2);
    }

    fn output_json(&self, suspect_links: &[requiem::storage::SuspectLink]) -> anyhow::Result<()> {
        use serde_json::json;

        let links: Vec<_> = suspect_links
            .iter()
            .map(|link| {
                json!({
                    "child": link.child_hrid.to_string(),
                    "parent": link.parent_hrid.to_string(),
                    "status": "fingerprint drift",
                    "stored_fingerprint": &link.stored_fingerprint,
                    "current_fingerprint": &link.current_fingerprint,
                })
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&links)?);
        Ok(())
    }

    fn output_ndjson(
        &self,
        suspect_links: &[requiem::storage::SuspectLink],
    ) -> anyhow::Result<()> {
        use serde_json::json;

        for link in suspect_links {
            let obj = json!({
                "child": link.child_hrid.to_string(),
                "parent": link.parent_hrid.to_string(),
                "status": "fingerprint drift",
                "stored_fingerprint": &link.stored_fingerprint,
                "current_fingerprint": &link.current_fingerprint,
            });
            println!("{}", serde_json::to_string(&obj)?);
        }
        Ok(())
    }

    fn output_table(&self, suspect_links: &[requiem::storage::SuspectLink]) -> anyhow::Result<()> {
        use terminal::Colorize;

        if self.detail {
            // Detailed block format
            for (i, link) in suspect_links.iter().enumerate() {
                if i > 0 {
                    println!("{}", "─────────────────────────────".dim());
                }
                println!("{} → {}", link.child_hrid, link.parent_hrid);
                println!("  Status:  fingerprint drift");
                println!("  Stored:  {}", link.stored_fingerprint);
                println!("  Current: {}", link.current_fingerprint);
            }
        } else {
            // Compact table format
            println!("{:<12} {:<12} {}", "CHILD", "PARENT", "STATUS");
            for link in suspect_links {
                println!(
                    "{:<12} {:<12} fingerprint drift",
                    link.child_hrid, link.parent_hrid
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
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

        let mut directory = Directory::new(path).load_all()?;

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
                println!("Pending updates: {} suspect links", count);
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
                    "Apply updates to {} suspect links across {} files? (y/N)",
                    count, file_count
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
            let link = suspect_links.iter().find(|l| {
                l.child_hrid == child && l.parent_hrid == parent
            });

            if let Some(link) = link {
                // Show confirmation banner
                if !self.yes {
                    println!("Reviewing: {} → {}", child, parent);
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

                match directory.accept_suspect_link(child.clone(), parent.clone())? {
                    requiem::storage::AcceptResult::Updated => {
                        println!("{}", format!("Accepted {} ← {}", child, parent).success());
                    }
                    requiem::storage::AcceptResult::AlreadyUpToDate => {
                        println!("No changes: link already up-to-date.");
                    }
                }
            } else {
                // Check if link exists but is not suspect
                match directory.accept_suspect_link(child.clone(), parent.clone())? {
                    requiem::storage::AcceptResult::Updated => {
                        println!("{}", format!("Accepted {} ← {}", child, parent).success());
                    }
                    requiem::storage::AcceptResult::AlreadyUpToDate => {
                        println!("No changes: link already up-to-date.");
                    }
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
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let config_path = root.join("config.toml");

        match self.command {
            ConfigCommand::Show => {
                let config = if config_path.exists() {
                    requiem::Config::load(&config_path)
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                } else {
                    requiem::Config::default()
                };

                println!("Configuration:");
                println!("  subfolders_are_namespaces: {} ({})",
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
                    requiem::Config::load(&config_path)
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                } else {
                    requiem::Config::default()
                };

                match key.as_str() {
                    "subfolders_are_namespaces" => {
                        let bool_value = value.parse::<bool>()
                            .map_err(|_| anyhow::anyhow!("Value must be 'true' or 'false'"))?;

                        config.set_subfolders_are_namespaces(bool_value);
                        config.save(&config_path)
                            .map_err(|e| anyhow::anyhow!("{}", e))?;

                        println!("{}", format!("Directory mode: {}",
                            if bool_value { "path-based" } else { "filename-based" }
                        ).success());

                        if bool_value {
                            println!("\n{}", "Path-based mode:".info());
                            println!("  • Filenames inside namespace folders should contain KIND-ID (e.g., USR/003.md).");
                            println!("  • Existing files are unchanged until you run 'req normalise-paths'.");
                        } else {
                            println!("\n{}", "Filename-based mode:".info());
                            println!("  • Namespaces will no longer be inferred from folders.");
                            println!("  • Full HRID must be in filename (e.g., system-auth-USR-003.md).");
                        }

                        println!("\n{}", format!("See docs/src/requirements/SPC-004.md for migration guide").dim());
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unknown configuration key: '{}'\nSupported keys: subfolders_are_namespaces",
                            key
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use requiem::{Directory, Requirement};
    use tempfile::tempdir;

    use super::*;

    fn load_directory(path: &Path) -> Directory<requiem::storage::directory::Loaded> {
        Directory::new(path.to_path_buf())
            .load_all()
            .expect("failed to load directory")
    }

    fn collect_child<'a>(
        directory: &'a Directory<requiem::storage::directory::Loaded>,
        kind: &str,
    ) -> &'a Requirement {
        directory
            .requirements()
            .find(|req| req.hrid().kind() == kind)
            .expect("expected requirement for kind")
    }

    #[test]
    fn add_run_creates_requirement_and_links_parents() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = load_directory(&root);
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

        let directory = load_directory(&root);
        let child = collect_child(&directory, "USR");

        assert!(child
            .parents()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
        assert_eq!(child.content(), "# Child\n\nbody text");
    }

    #[test]
    fn add_run_uses_template_when_no_content_provided() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        let template_dir = root.join(".req").join("templates");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("USR.md"), "## Template body").unwrap();

        let add = Add {
            kind: "USR".to_string(),
            parent: Vec::new(),
            title: None,
            body: None,
        };

        add.run(root.to_path_buf())
            .expect("add command should succeed");

        let directory = load_directory(root);
        let child = collect_child(&directory, "USR");
        assert_eq!(child.content(), "## Template body");
    }

    #[test]
    fn link_run_updates_child_parent_relationship() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = load_directory(&root);
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

        let directory = load_directory(&root);
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

        let mut directory = load_directory(&root);
        let parent = directory
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR".to_string(), "# Child".to_string())
            .unwrap();

        Directory::new(root.clone())
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

        let mut directory = load_directory(&root);
        let parent = directory
            .add_requirement("SYS".to_string(), "# Parent".to_string())
            .unwrap();
        let child = directory
            .add_requirement("USR".to_string(), "# Child".to_string())
            .unwrap();

        // Create a parent-child relationship to ensure we exercise counting logic.
        Directory::new(root.clone())
            .link_requirement(child.hrid().clone(), parent.hrid().clone())
            .unwrap();

        Status::default()
            .run(root)
            .expect("status should succeed when no suspect links exist");
    }
}
