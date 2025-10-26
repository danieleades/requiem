use std::path::PathBuf;

use clap::ArgAction;
use requiem::{Directory, Hrid};
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
    command: Command,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        Self::setup_logging(self.verbose);

        self.command.run(self.root)
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
    Suspect,

    /// Accept suspect links after review
    ///
    /// Updates fingerprints to mark requirements as reviewed and valid.
    Accept(Accept),
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Add(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Clean => Clean::run(root)?,
            Self::Suspect => Suspect::run(root)?,
            Self::Accept(command) => command.run(root)?,
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

        let content = self.build_content();
        let requirement = directory.add_requirement(self.kind, content)?;

        for parent in self.parent {
            // TODO: the linkage should be done before the requirement is saved by the
            // 'add_requirement' method to avoid unnecessary IO.
            directory.link_requirement(requirement.hrid().clone(), parent)?;
        }

        println!("Added requirement {}", requirement.hrid());
        Ok(())
    }

    /// Build content string from title and body options.
    fn build_content(&self) -> String {
        match (&self.title, &self.body) {
            (Some(title), Some(body)) => format!("# {title}\n\n{body}"),
            (Some(title), None) => format!("# {title}"),
            (None, Some(body)) => body.clone(),
            (None, None) => String::new(),
        }
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
        Directory::new(path)?.update_hrids()?;
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Suspect {}

impl Suspect {
    #[instrument]
    fn run(path: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(path)?;
        let suspect_links = directory.suspect_links();

        if suspect_links.is_empty() {
            println!("No suspect links found.");
            return Ok(());
        }

        println!("Found {} suspect link(s):\n", suspect_links.len());
        print_suspect_links_detailed(&suspect_links);

        // Exit with non-zero status to indicate suspect links exist (for CI)
        std::process::exit(1);
    }
}

#[derive(Debug, clap::Parser)]
pub struct Accept {
    /// Accept all suspect links
    #[arg(long)]
    all: bool,

    /// Preview changes without applying them (only with --all)
    #[arg(long, requires = "all")]
    dry_run: bool,

    /// Skip confirmation prompt (only with --all)
    #[arg(long, requires = "all")]
    force: bool,

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
        let mut directory = Directory::new(path)?;

        if self.all {
            // Get suspect links
            let suspect_links = directory.suspect_links();

            if suspect_links.is_empty() {
                println!("No suspect links to accept.");
                return Ok(());
            }

            // Show what will be updated
            println!("Found {} suspect link(s):", suspect_links.len());
            print_suspect_links(&suspect_links);
            println!();

            // Dry-run mode: preview only
            if self.dry_run {
                println!("Dry-run mode: no changes made.");
                return Ok(());
            }

            // Confirmation prompt (unless --force)
            if !self.force {
                let prompt = format!("Accept all {} suspect link(s)?", suspect_links.len());
                if !confirm_action(&prompt)? {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            // Accept all suspect links
            let updated = directory.accept_all_suspect_links()?;

            println!("Accepted {} suspect link(s):", updated.len());
            for (child, parent) in &updated {
                println!("  {child} → {parent}");
            }
        } else {
            let child = self
                .child
                .as_ref()
                .expect("child is required when --all is not set");
            let parent = self
                .parent
                .as_ref()
                .expect("parent is required when --all is not set");

            match directory.accept_suspect_link(child.clone(), parent.clone())? {
                requiem::storage::AcceptResult::Updated => {
                    println!("Accepted suspect link: {child} → {parent}");
                }
                requiem::storage::AcceptResult::AlreadyUpToDate => {
                    println!("Link {child} → {parent} is already up to date (not suspect).");
                }
            }
        }

        Ok(())
    }
}

/// Print a list of suspect links in a consistent format.
fn print_suspect_links(links: &[requiem::storage::SuspectLink]) {
    for link in links {
        let child = &link.child_hrid;
        let parent = &link.parent_hrid;
        println!("  {child} → {parent}");
    }
}

/// Print detailed information about suspect links including fingerprints.
fn print_suspect_links_detailed(links: &[requiem::storage::SuspectLink]) {
    for link in links {
        let child = &link.child_hrid;
        let parent = &link.parent_hrid;
        println!("  {child} → {parent}");
        println!(
            "    Stored fingerprint:  {}",
            &link.stored_fingerprint[..16]
        );
        println!(
            "    Current fingerprint: {}\n",
            &link.current_fingerprint[..16]
        );
    }
}

/// Prompt the user for confirmation to proceed with an action.
///
/// Returns `Ok(true)` if user confirms, `Ok(false)` if user declines.
fn confirm_action(prompt: &str) -> anyhow::Result<bool> {
    use std::io::{self, Write};

    print!("{prompt} [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}
