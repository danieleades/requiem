use std::{
    io::{self, BufRead},
    path::{Path, PathBuf},
};

mod list;
mod show;
mod status;
mod terminal;
mod validate;

use clap::ArgAction;
use list::List;
use requiem::{Directory, Hrid};
use show::Show;
use status::Status;
use tracing::instrument;
use validate::Validate;

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
    Init(Init),

    /// Create a new requirement
    Create(Create),

    /// Delete a requirement
    Delete(Delete),

    /// Create a link between two requirements
    ///
    /// Links are parent-child relationships.
    Link(Link),

    /// Remove a link between two requirements
    Unlink(Unlink),

    /// Synchronize parent HRIDs and file paths
    Sync(Sync),

    /// Review suspect links
    ///
    /// Suspect links are those where the parent requirement has changed
    /// since the link was created or last reviewed.
    Review(Review),

    /// Validate repository health
    Validate(Validate),

    /// Show detailed information about a requirement
    Show(Show),

    /// List requirements with filters and relationship views
    List(List),

    /// Show or modify configuration settings
    Config(Config),

    /// Manage requirement kinds
    Kind(Kind),

    /// Rename a requirement's HRID
    Rename(Rename),

    /// Move a requirement to a new file path
    Move(Move),

    /// Diagnose path-related issues
    Diagnose(Diagnose),
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Status(command) => command.run(root)?,
            Self::Init(command) => command.run(&root)?,
            Self::Create(command) => command.run(root)?,
            Self::Delete(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Unlink(command) => command.run(root)?,
            Self::Sync(command) => command.run(root)?,
            Self::Review(command) => command.run(root)?,
            Self::Validate(command) => command.run(root)?,
            Self::Show(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
            Self::Config(command) => command.run(&root)?,
            Self::Kind(command) => command.run(&root)?,
            Self::Rename(command) => command.run(root)?,
            Self::Move(command) => command.run(&root)?,
            Self::Diagnose(command) => command.run(&root)?,
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Init {
    /// Requirement kinds to create templates for
    #[arg(long, value_name = "KIND", num_args = 0..)]
    kinds: Vec<String>,
}

impl Init {
    #[instrument]
    fn run(self, root: &PathBuf) -> anyhow::Result<()> {
        use std::fs;

        // Create .req directory
        let req_dir = root.join(".req");
        if req_dir.exists() {
            anyhow::bail!("Repository already initialized (found existing .req directory)");
        }

        fs::create_dir_all(&req_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create .req directory: {e}"))?;

        // Create config.toml with defaults (no kinds configured)
        let config_path = req_dir.join("config.toml");
        let config = requiem::Config::default();
        config
            .save(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to create config.toml: {e}"))?;

        // Create templates directory
        let templates_dir = req_dir.join("templates");
        fs::create_dir_all(&templates_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create templates directory: {e}"))?;

        println!("Initialized requirements repository in {}", root.display());
        println!("  Created: .req/config.toml");
        println!("  Created: .req/templates/ (empty)");

        // Create templates for specified kinds
        let mut created_templates = Vec::new();
        for kind in &self.kinds {
            let kind_upper = kind.to_uppercase();

            // Validate kind format
            if !kind_upper.chars().all(|c| c.is_ascii_uppercase()) {
                anyhow::bail!("Invalid kind '{kind}': kinds must contain only letters (A-Z)");
            }

            let template_path = templates_dir.join(format!("{kind_upper}.md"));
            let template_content = Self::default_template_for_kind(&kind_upper);

            fs::write(&template_path, template_content)
                .map_err(|e| anyhow::anyhow!("Failed to create {kind_upper} template: {e}"))?;

            created_templates.push(kind_upper);
        }

        if !created_templates.is_empty() {
            for kind in &created_templates {
                println!("  Created: .req/templates/{kind}.md");
            }
        }

        println!();
        println!("Next steps:");
        if created_templates.is_empty() {
            println!("  req kind add USR SYS  # Register requirement kinds");
            println!("  req create USR --title \"Your First Requirement\"");
        } else {
            println!(
                "  req create {} --title \"Your First Requirement\"",
                created_templates[0]
            );
        }

        Ok(())
    }

    /// Returns default template content for a given kind.
    fn default_template_for_kind(kind: &str) -> String {
        match kind {
            "USR" => "## Statement\n\nThe system shall [describe what must be accomplished from \
                      user perspective].\n\n## Rationale\n\n[Explain why this requirement \
                      exists]\n\n## Acceptance Criteria\n\n- [Criterion 1: Specific, measurable \
                      condition that must be met]\n- [Criterion 2: Observable behavior or \
                      outcome]\n"
                .to_string(),
            "SYS" => "## Description\n\n[Describe the system-level requirement or implementation \
                      approach]\n\n## Technical Details\n\n[Technical specifications, \
                      constraints, or implementation notes]\n"
                .to_string(),
            _ => {
                format!(
                    "## Description\n\n[Description of {kind} requirement]\n\n## \
                     Details\n\n[Additional details and specifications]\n"
                )
            }
        }
    }
}

#[derive(Debug, clap::Parser)]
pub struct Create {
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

impl Create {
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
#[allow(clippy::struct_excessive_bools)]
pub struct Delete {
    /// The human-readable ID of the requirement to delete
    #[clap(value_parser = parse_hrid)]
    hrid: Hrid,

    /// Delete requirement and orphaned descendants (children with no other
    /// parents)
    #[arg(long)]
    cascade: bool,

    /// Delete requirement and unlink from children (children remain)
    #[arg(long, conflicts_with = "cascade")]
    orphan: bool,

    /// Show what would be deleted without deleting
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Delete {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.hrid) else {
            anyhow::bail!("Requirement {} not found", self.hrid.display(digits));
        };

        let hrid = req.hrid.clone();
        let children = directory.children_of(&hrid);

        // Check if requirement has children
        if !children.is_empty() && !self.cascade && !self.orphan {
            eprintln!(
                "{}",
                format!(
                    "⚠️  Cannot delete {}: requirement has {} children",
                    hrid.display(digits),
                    children.len()
                )
                .warning()
            );
            eprintln!("\nChildren:");
            for child in &children {
                eprintln!("  • {}", child.display(digits));
            }
            eprintln!(
                "\n{}",
                "Use --cascade to delete with orphaned descendants, or --orphan to unlink children"
                    .dim()
            );
            anyhow::bail!("Cannot delete requirement with children");
        }

        // Determine what will be deleted
        let to_delete = if self.cascade {
            // Smart cascade: find descendants that would become orphans
            directory.find_orphaned_descendants(&hrid)
        } else {
            vec![hrid.clone()]
        };

        // Show preview
        if !self.yes && !self.dry_run {
            use std::io::{self, BufRead};

            println!("Will delete {} requirement(s):", to_delete.len());
            for delete_hrid in &to_delete {
                println!("  • {}", delete_hrid.display(digits));
            }

            if self.orphan && !children.is_empty() {
                println!("\nWill unlink from {} children:", children.len());
                for child in &children {
                    println!("  • {}", child.display(digits));
                }
            }

            // Get confirmation
            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        if self.dry_run {
            println!(
                "{}",
                format!("Would delete {} requirement(s)", to_delete.len()).dim()
            );
            return Ok(());
        }

        // Perform deletion
        if self.orphan {
            directory.delete_and_orphan(&hrid)?;
        } else if self.cascade {
            for delete_hrid in &to_delete {
                directory.delete_requirement(delete_hrid)?;
            }
        } else {
            directory.delete_requirement(&hrid)?;
        }

        directory.flush()?;

        println!(
            "{}",
            format!("✅ Deleted {} requirement(s)", to_delete.len()).success()
        );
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
pub struct Unlink {
    /// The human-readable ID of the child document
    #[clap(value_parser = parse_hrid)]
    child: Hrid,

    /// The human-readable ID of the parent document to remove
    #[clap(value_parser = parse_hrid)]
    parent: Hrid,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Unlink {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Validate both requirements exist
        let Some(_child_req) = directory.find_by_hrid(&self.child) else {
            anyhow::bail!("Child requirement {} not found", self.child.display(digits));
        };

        let Some(_parent_req) = directory.find_by_hrid(&self.parent) else {
            anyhow::bail!(
                "Parent requirement {} not found",
                self.parent.display(digits)
            );
        };

        // Show confirmation prompt unless --yes was specified
        if !self.yes {
            use std::io::{self, BufRead};

            println!(
                "Will unlink {} from parent {}",
                self.child.display(digits),
                self.parent.display(digits)
            );
            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Perform the unlink
        directory.unlink_requirement(&self.child, &self.parent)?;
        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Unlinked {} from {}",
                self.child.display(digits),
                self.parent.display(digits)
            )
            .success()
        );

        Ok(())
    }
}

/// What to synchronize
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum SyncWhat {
    /// Update parent HRIDs in child requirements (default)
    Parents,
    /// Move files to canonical locations
    Paths,
    /// Update both parent HRIDs and file paths
    All,
}

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Sync {
    /// What to synchronize
    #[arg(long, default_value = "parents")]
    what: SyncWhat,

    /// Check for drift without making changes (exits with code 2 if drift
    /// found)
    #[arg(long)]
    check: bool,

    /// Show what would be changed without making changes
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,

    /// Suppress output
    #[arg(long, short)]
    quiet: bool,
}

impl Sync {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;

        match (self.what, self.check, self.dry_run) {
            (SyncWhat::Parents, true, _) => {
                self.check_parent_drift(&directory);
                Ok(())
            }
            (SyncWhat::Parents, false, dry_run) => self.sync_parents(&mut directory, dry_run),
            (SyncWhat::Paths, true, _) => {
                self.check_path_drift(&directory);
                Ok(())
            }
            (SyncWhat::Paths, false, dry_run) => self.sync_paths(&mut directory, dry_run),
            (SyncWhat::All, check, dry_run) => self.sync_all(&mut directory, check, dry_run),
        }
    }

    fn check_parent_drift(&self, directory: &Directory) {
        use terminal::Colorize;

        let would_update = directory.check_hrid_drift();
        if would_update.is_empty() {
            if !self.quiet {
                println!("{}", "✅ No HRID drift detected.".success());
            }
        } else {
            if !self.quiet {
                println!(
                    "{}",
                    format!(
                        "⚠️  {} requirements have stale parent HRIDs",
                        would_update.len()
                    )
                    .warning()
                );
                for hrid in &would_update {
                    println!("  • {}", hrid.display(directory.config().digits()));
                }
            }
            std::process::exit(2);
        }
    }

    fn sync_parents(&self, directory: &mut Directory, dry_run: bool) -> anyhow::Result<()> {
        use terminal::Colorize;

        let updated = directory.update_hrids();

        if updated.is_empty() {
            if !self.quiet {
                println!("{}", "✅ All parent HRIDs are current.".success());
            }
            return Ok(());
        }

        if dry_run {
            if !self.quiet {
                println!("Would update {} parent HRIDs:", updated.len());
                for hrid in &updated {
                    println!("  • {}", hrid.display(directory.config().digits()));
                }
            }
            return Ok(());
        }

        directory.flush()?;

        if !self.quiet {
            println!(
                "{}",
                format!("✅ Updated {} parent HRIDs", updated.len()).success()
            );
        }
        Ok(())
    }

    fn check_path_drift(&self, directory: &Directory) {
        use terminal::Colorize;

        let misplaced = directory.check_path_drift();
        if misplaced.is_empty() {
            if !self.quiet {
                println!(
                    "{}",
                    "✅ All requirements are in canonical locations.".success()
                );
            }
        } else {
            if !self.quiet {
                println!(
                    "{}",
                    format!("⚠️  {} requirements are misplaced", misplaced.len()).warning()
                );
                for (hrid, current, canonical) in &misplaced {
                    println!(
                        "  • {} ({} → {})",
                        hrid.display(directory.config().digits()),
                        current.display(),
                        canonical.display()
                    );
                }
            }
            std::process::exit(2);
        }
    }

    fn sync_paths(&self, directory: &mut Directory, dry_run: bool) -> anyhow::Result<()> {
        use terminal::Colorize;

        let misplaced = directory.check_path_drift();

        if misplaced.is_empty() {
            if !self.quiet {
                println!(
                    "{}",
                    "✅ All requirements are in canonical locations.".success()
                );
            }
            return Ok(());
        }

        if dry_run {
            if !self.quiet {
                println!("Would move {} files:", misplaced.len());
                for (hrid, current, canonical) in &misplaced {
                    println!(
                        "  • {}: {} → {}",
                        hrid.display(directory.config().digits()),
                        current.display(),
                        canonical.display()
                    );
                }
            }
            return Ok(());
        }

        // Confirm before moving files
        if !self.yes {
            use std::io::{self, BufRead};

            println!(
                "Will move {} files to canonical locations:",
                misplaced.len()
            );
            for (hrid, current, canonical) in &misplaced {
                println!(
                    "  • {}: {} → {}",
                    hrid.display(directory.config().digits()),
                    current.display(),
                    canonical.display()
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

        let moved = directory.sync_paths()?;

        if !self.quiet {
            println!("{}", format!("✅ Moved {} files", moved.len()).success());
        }
        Ok(())
    }

    fn sync_all(
        &self,
        directory: &mut Directory,
        check: bool,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        use terminal::Colorize;

        let hrid_drift = directory.check_hrid_drift();
        let path_drift = directory.check_path_drift();

        if check {
            self.check_all_drift(&hrid_drift, &path_drift);
            return Ok(());
        }

        if dry_run {
            self.dry_run_all(&hrid_drift, &path_drift);
            return Ok(());
        }

        // Confirm before making changes
        if !self.yes && (!hrid_drift.is_empty() || !path_drift.is_empty()) {
            Self::confirm_sync_all(&hrid_drift, &path_drift)?;
        }

        // Perform both updates
        let updated_hrids = directory.update_hrids();
        if !updated_hrids.is_empty() {
            directory.flush()?;
        }

        let moved = directory.sync_paths()?;

        if !self.quiet {
            if !updated_hrids.is_empty() {
                println!(
                    "{}",
                    format!("✅ Updated {} parent HRIDs", updated_hrids.len()).success()
                );
            }
            if !moved.is_empty() {
                println!("{}", format!("✅ Moved {} files", moved.len()).success());
            }
            if updated_hrids.is_empty() && moved.is_empty() {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }
        Ok(())
    }

    fn check_all_drift(
        &self,
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) {
        use terminal::Colorize;

        let has_drift = !hrid_drift.is_empty() || !path_drift.is_empty();

        if !self.quiet {
            if !hrid_drift.is_empty() {
                println!(
                    "{}",
                    format!(
                        "⚠️  {} requirements have stale parent HRIDs",
                        hrid_drift.len()
                    )
                    .warning()
                );
            }
            if !path_drift.is_empty() {
                println!(
                    "{}",
                    format!("⚠️  {} requirements are misplaced", path_drift.len()).warning()
                );
            }
            if !has_drift {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }

        if has_drift {
            std::process::exit(2);
        }
    }

    fn dry_run_all(
        &self,
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) {
        use terminal::Colorize;

        if !self.quiet {
            if !hrid_drift.is_empty() {
                println!("Would update {} parent HRIDs", hrid_drift.len());
            }
            if !path_drift.is_empty() {
                println!("Would move {} files", path_drift.len());
            }
            if hrid_drift.is_empty() && path_drift.is_empty() {
                println!("{}", "✅ Everything is synchronized.".success());
            }
        }
    }

    fn confirm_sync_all(
        hrid_drift: &[requiem::Hrid],
        path_drift: &[(requiem::Hrid, std::path::PathBuf, std::path::PathBuf)],
    ) -> anyhow::Result<()> {
        use std::io::{self, BufRead};

        println!("Will synchronize:");
        if !hrid_drift.is_empty() {
            println!("  • Update {} parent HRIDs", hrid_drift.len());
        }
        if !path_drift.is_empty() {
            println!("  • Move {} files", path_drift.len());
        }

        eprint!("\nProceed? (y/N) ");
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            std::process::exit(130);
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Review {
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

impl Review {
    #[instrument]
    fn run(self, path: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

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

    /// Handle accepting suspect links
    #[instrument]
    fn run_accept(self, path: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

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
                    let stdin = io::stdin();
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

#[derive(Debug, clap::Parser)]
/// Show or modify repository configuration
///
/// Configuration is stored in .req/config.toml and controls repository
/// behavior.
///
/// Available configuration keys:
///   `subfolders_are_namespaces`  Path mode (true) vs filename mode (false)
///   digits                      Number of digits for HRID padding (default: 3)
///   `allow_unrecognised`         Allow non-HRID markdown files (default:
/// false)
///
/// Note: Use 'req kind' commands to manage `allowed_kinds` configuration.
pub struct Config {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, clap::Parser)]
enum ConfigCommand {
    /// Show all configuration values
    Show,

    /// Get a specific configuration value
    Get {
        /// Configuration key to retrieve
        ///
        /// Available keys: `subfolders_are_namespaces`, digits,
        /// `allow_unrecognised`, `allowed_kinds`
        key: String,
    },

    /// Set a configuration value
    ///
    /// Examples:
    ///   req config set `subfolders_are_namespaces` true
    ///   req config set `allow_unrecognised` false
    Set {
        /// Configuration key to set
        ///
        /// Settable keys: `subfolders_are_namespaces`, `allow_unrecognised`
        key: String,

        /// Value to set
        value: String,
    },
}

impl Config {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        let config_path = root.join(".req/config.toml");

        match self.command {
            ConfigCommand::Show => Self::show_config(&config_path),
            ConfigCommand::Get { key } => Self::get_config(&config_path, &key),
            ConfigCommand::Set { key, value } => Self::set_config(&config_path, &key, &value),
        }
    }

    fn show_config(config_path: &std::path::Path) -> anyhow::Result<()> {
        use terminal::Colorize;

        let config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
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
        if config.allowed_kinds().is_empty() {
            println!("  allowed_kinds: {} (all kinds allowed)", "[]".dim());
        } else {
            println!("  allowed_kinds: {:?}", config.allowed_kinds());
        }
        Ok(())
    }

    fn get_config(config_path: &std::path::Path, key: &str) -> anyhow::Result<()> {
        let config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem::Config::default()
        };

        match key {
            "subfolders_are_namespaces" => {
                println!("{}", config.subfolders_are_namespaces);
            }
            "digits" => {
                println!("{}", config.digits());
            }
            "allow_unrecognised" => {
                println!("{}", config.allow_unrecognised);
            }
            "allowed_kinds" => {
                if config.allowed_kinds().is_empty() {
                    println!("[]");
                } else {
                    for kind in config.allowed_kinds() {
                        println!("{kind}");
                    }
                }
            }
            _ => {
                anyhow::bail!(
                    "Unknown configuration key: '{key}'\n\nAvailable keys:\n  \
                     subfolders_are_namespaces\n  digits\n  allow_unrecognised\n  allowed_kinds",
                );
            }
        }
        Ok(())
    }

    fn set_config(config_path: &std::path::Path, key: &str, value: &str) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem::Config::default()
        };

        match key {
            "subfolders_are_namespaces" => {
                let bool_value = value
                    .parse::<bool>()
                    .map_err(|_| anyhow::anyhow!("Value must be 'true' or 'false'"))?;

                config.set_subfolders_are_namespaces(bool_value);
                config
                    .save(config_path)
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
                        "  • Filenames inside namespace folders should contain KIND-ID (e.g., \
                         USR/003.md)."
                    );
                    println!(
                        "  • You will need to manually reorganize existing files to match the new \
                         structure."
                    );
                } else {
                    println!("\n{}", "Filename-based mode:".info());
                    println!("  • Namespaces will no longer be inferred from folders.");
                    println!("  • Full HRID must be in filename (e.g., system-auth-USR-003.md).");
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
                    "Unknown configuration key: '{key}'\nSupported keys: subfolders_are_namespaces",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Kind {
    #[command(subcommand)]
    command: KindCommand,
}

#[derive(Debug, clap::Parser)]
enum KindCommand {
    /// Add one or more requirement kinds to the allowed list
    Add {
        /// The kinds to add (e.g., USR, SYS, TST)
        kinds: Vec<String>,
    },

    /// Remove one or more requirement kinds from the allowed list
    Remove {
        /// The kinds to remove
        kinds: Vec<String>,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// List all registered requirement kinds
    List,
}

impl Kind {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        let config_path = root.join(".req/config.toml");

        match self.command {
            KindCommand::Add { kinds } => Self::add_kinds(&config_path, kinds),
            KindCommand::Remove { kinds, yes } => {
                Self::remove_kinds(&config_path, root, kinds, yes)
            }
            KindCommand::List => Self::list_kinds(&config_path),
        }
    }

    fn add_kinds(config_path: &std::path::Path, kinds: Vec<String>) -> anyhow::Result<()> {
        use terminal::Colorize;

        if kinds.is_empty() {
            anyhow::bail!("At least one kind must be specified");
        }

        // Load config
        let mut config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            anyhow::bail!(
                "Repository not initialized. Run 'req init' first or ensure you're in a \
                 requirements repository"
            );
        };

        // Validate and add kinds
        let mut added = Vec::new();
        let mut already_exists = Vec::new();

        for kind in kinds {
            // Validate kind format (must be uppercase alphabetic)
            let kind_upper = kind.to_uppercase();
            if !kind_upper.chars().all(|c| c.is_ascii_uppercase()) {
                anyhow::bail!("Invalid kind '{kind}': kinds must contain only letters (A-Z)");
            }

            if config.add_kind(&kind_upper) {
                added.push(kind_upper);
            } else {
                already_exists.push(kind_upper);
            }
        }

        // Save config if any kinds were added
        if !added.is_empty() {
            config
                .save(config_path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!(
                "{}",
                format!("✅ Added {} kind(s): {}", added.len(), added.join(", ")).success()
            );
        }

        if !already_exists.is_empty() {
            println!(
                "{}",
                format!("ℹ️  Already registered: {}", already_exists.join(", ")).dim()
            );
        }

        Ok(())
    }

    fn remove_kinds(
        config_path: &std::path::Path,
        root: &Path,
        kinds: Vec<String>,
        yes: bool,
    ) -> anyhow::Result<()> {
        use terminal::Colorize;

        if kinds.is_empty() {
            anyhow::bail!("At least one kind must be specified");
        }

        // Load config
        let mut config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            anyhow::bail!(
                "Repository not initialized. Run 'req init' first or ensure you're in a \
                 requirements repository"
            );
        };

        // Load directory to check for existing requirements
        let directory = Directory::new(root.to_path_buf())?;

        // Check if requirements exist for these kinds
        let mut warnings = Vec::new();
        for kind in &kinds {
            let kind_upper = kind.to_uppercase();
            let count = directory
                .requirements()
                .filter(|req| req.hrid.kind() == kind_upper)
                .count();

            if count > 0 {
                warnings.push(format!("  • {kind_upper}: {count} requirements exist"));
            }
        }

        // Show warnings if requirements exist
        if !warnings.is_empty() && !yes {
            use std::io::{self, BufRead};

            println!(
                "{}",
                "⚠️  The following kinds have existing requirements:".warning()
            );
            for warning in &warnings {
                println!("{warning}");
            }
            println!(
                "\n{}",
                "Removing these kinds will NOT delete the requirements, but they will be \
                 considered invalid."
                    .dim()
            );

            eprint!("\nProceed? (y/N) ");
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                std::process::exit(130);
            }
        }

        // Remove kinds
        let mut removed = Vec::new();
        let mut not_found = Vec::new();

        for kind in kinds {
            let kind_upper = kind.to_uppercase();
            if config.remove_kind(&kind_upper) {
                removed.push(kind_upper);
            } else {
                not_found.push(kind_upper);
            }
        }

        // Save config if any kinds were removed
        if !removed.is_empty() {
            config
                .save(config_path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!(
                "{}",
                format!(
                    "✅ Removed {} kind(s): {}",
                    removed.len(),
                    removed.join(", ")
                )
                .success()
            );
        }

        if !not_found.is_empty() {
            println!(
                "{}",
                format!("ℹ️  Not found: {}", not_found.join(", ")).dim()
            );
        }

        Ok(())
    }

    fn list_kinds(config_path: &std::path::Path) -> anyhow::Result<()> {
        use terminal::Colorize;

        let config = if config_path.exists() {
            requiem::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            requiem::Config::default()
        };

        let kinds = config.allowed_kinds();

        if kinds.is_empty() {
            println!("{}", "No kinds configured (all kinds allowed)".dim());
        } else {
            println!("Registered requirement kinds:");
            for kind in kinds {
                println!("  • {kind}");
            }
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Rename {
    /// The current HRID of the requirement to rename
    #[clap(value_parser = parse_hrid)]
    old_hrid: Hrid,

    /// The new HRID for the requirement
    #[clap(value_parser = parse_hrid)]
    new_hrid: Hrid,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Rename {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.old_hrid) else {
            anyhow::bail!("Requirement {} not found", self.old_hrid.display(digits));
        };

        // Check if children exist
        let children = directory.children_of(&self.old_hrid);

        // Show confirmation if there are children or --yes not specified
        if !self.yes {
            println!(
                "Renaming {} → {}",
                self.old_hrid.display(digits),
                self.new_hrid.display(digits)
            );
            println!("  Title: {}", req.title);

            if !children.is_empty() {
                println!(
                    "\n{} will be updated in {} children:",
                    "Parent HRID".dim(),
                    children.len()
                );
                for child_hrid in &children {
                    println!("  • {}", child_hrid.display(digits));
                }
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

        // Perform rename
        let children_updated = directory.rename_requirement(&self.old_hrid, &self.new_hrid)?;
        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Renamed {} → {}",
                self.old_hrid.display(digits),
                self.new_hrid.display(digits)
            )
            .success()
        );

        if !children_updated.is_empty() {
            println!(
                "{}",
                format!("   Updated {} children", children_updated.len()).dim()
            );
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Move {
    /// The HRID of the requirement to move
    #[clap(value_parser = parse_hrid)]
    hrid: Hrid,

    /// The new file path (relative to repository root)
    new_path: PathBuf,

    /// Skip confirmation prompts
    #[arg(long, short)]
    yes: bool,
}

impl Move {
    #[instrument]
    fn run(self, root: &Path) -> anyhow::Result<()> {
        use terminal::Colorize;

        let mut directory = Directory::new(root.to_path_buf())?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.hrid) else {
            anyhow::bail!("Requirement {} not found", self.hrid.display(digits));
        };

        // Get current path
        let old_path = directory.path_for(&self.hrid).ok_or_else(|| {
            anyhow::anyhow!("Cannot find current path for {}", self.hrid.display(digits))
        })?;

        // Make new path absolute if relative
        let new_path = if self.new_path.is_absolute() {
            self.new_path.clone()
        } else {
            root.join(&self.new_path)
        };

        // Extract HRID from new path to see if it will change
        let new_hrid = requiem::hrid_from_path(&new_path, root, directory.config())
            .map_err(|e| anyhow::anyhow!("Failed to parse HRID from path: {e}"))?;

        // Check if children exist
        let children = directory.children_of(&self.hrid);

        // Show confirmation if --yes not specified
        if !self.yes {
            use std::io::{self, BufRead};

            println!(
                "Moving {} from {} to {}",
                self.hrid.display(digits),
                old_path.strip_prefix(root).unwrap_or(old_path).display(),
                self.new_path.display()
            );
            println!("  Title: {}", req.title);

            if new_hrid != self.hrid {
                println!(
                    "\n{} HRID will change: {} → {}",
                    "⚠️".warning(),
                    self.hrid.display(digits),
                    new_hrid.display(digits)
                );

                if !children.is_empty() {
                    println!(
                        "   {} will be updated in {} children",
                        "Parent HRID".dim(),
                        children.len()
                    );
                }
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

        // Perform move
        let children_updated = directory.move_requirement(&self.hrid, new_path.clone())?;

        // Create parent directories if needed
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        directory.flush()?;

        println!(
            "{}",
            format!(
                "✅ Moved {} to {}",
                self.hrid.display(digits),
                self.new_path.display()
            )
            .success()
        );

        if let Some(children) = children_updated {
            if !children.is_empty() {
                println!(
                    "{}",
                    format!(
                        "   Updated HRID {} → {} in {} children",
                        self.hrid.display(digits),
                        new_hrid.display(digits),
                        children.len()
                    )
                    .dim()
                );
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
    fn create_run_creates_requirement_and_links_parents() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush parent requirement");

        let create = Create {
            kind: "USR".to_string(),
            parent: vec![parent.hrid().clone()],
            title: Some("Child".to_string()),
            body: Some("body text".to_string()),
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

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
    fn create_run_uses_template_when_no_content_provided() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let template_dir = root.join(".req").join("templates");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("USR.md"), "## Template body").unwrap();

        let create = Create {
            kind: "USR".to_string(),
            parent: Vec::new(),
            title: None,
            body: None,
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let child = collect_child(&directory, "USR");
        assert_eq!(child.body, "## Template body");
    }

    #[test]
    fn create_run_creates_namespaced_requirement() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let create = Create {
            kind: "SYSTEM-AUTH-USR".to_string(),
            parent: Vec::new(),
            title: Some("Namespaced Requirement".to_string()),
            body: Some("test body".to_string()),
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

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
