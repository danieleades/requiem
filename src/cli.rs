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

    /// List all requirements
    List(List),

    /// Show details of a specific requirement
    Show(Show),

    /// Remove a requirement
    Remove(Remove),

    /// Remove a link between two requirements
    Unlink(Unlink),

    /// Edit the content of a requirement
    Edit(Edit),

    /// Correct parent HRIDs
    Clean,
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Add(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
            Self::Show(command) => command.run(root)?,
            Self::Remove(command) => command.run(root)?,
            Self::Unlink(command) => command.run(root)?,
            Self::Edit(command) => command.run(root)?,
            Self::Clean => Clean::run(root)?,
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
}

impl Add {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root).load_all()?;
        let requirement = directory.add_requirement(self.kind)?;

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
pub struct List {
    /// Filter by requirement kind (e.g., USR, SYS)
    #[clap(long, short)]
    kind: Option<String>,

    /// Show verbose output including content preview
    #[clap(long, short = 'V')]
    verbose: bool,
}

impl List {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root).load_all()?;

        let mut requirements: Vec<_> = directory.requirements().collect();
        requirements.sort_by_key(|r| r.hrid().to_string());

        if let Some(kind) = &self.kind {
            requirements.retain(|r| r.hrid().kind() == kind);
        }

        if requirements.is_empty() {
            println!("No requirements found");
            return Ok(());
        }

        for req in requirements {
            let hrid = req.hrid();
            let parent_count = req.parents().count();

            if self.verbose {
                let content_preview = req.content()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect::<String>();
                let preview = if content_preview.is_empty() {
                    "(empty)".to_string()
                } else {
                    content_preview
                };
                println!("{} [{}] - {}", hrid, parent_count, preview);
            } else {
                println!("{} [{}]", hrid, parent_count);
            }
        }

        println!("\nTotal: {} requirements", requirements.len());

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Show {
    /// The human-readable ID of the requirement to show
    hrid: Hrid,

    /// Show raw content without formatting
    #[clap(long)]
    raw: bool,
}

impl Show {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let requirement = requiem::Requirement::load(&root, self.hrid)?;

        if self.raw {
            print!("{}", requirement.content());
        } else {
            println!("ID: {}", requirement.hrid());
            println!("UUID: {}", requirement.uuid());
            println!("Created: {}", requirement.created());
            println!("Fingerprint: {}", requirement.fingerprint());

            let tags: Vec<_> = requirement.tags().iter().collect();
            if !tags.is_empty() {
                println!("Tags: {}", tags.join(", "));
            }

            let parents: Vec<_> = requirement.parents().collect();
            if !parents.is_empty() {
                println!("\nParents:");
                for (_, parent) in parents {
                    println!("  - {}", parent.hrid);
                }
            }

            println!("\nContent:");
            println!("{}", requirement.content());
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Remove {
    /// The human-readable ID of the requirement to remove
    hrid: Hrid,

    /// Skip confirmation prompt
    #[clap(long, short)]
    force: bool,
}

impl Remove {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        if !self.force {
            println!("Are you sure you want to remove {}? (y/N)", self.hrid);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled");
                return Ok(());
            }
        }

        let mut directory = Directory::new(root).load_all()?;
        directory.remove_requirement(self.hrid.clone())?;

        println!("Removed requirement {}", self.hrid);
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Unlink {
    /// The human-readable ID of the child requirement
    child: Hrid,

    /// The human-readable ID of the parent requirement to unlink
    parent: Hrid,
}

impl Unlink {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root);
        directory.unlink_requirement(self.child.clone(), self.parent.clone())?;

        println!("Unlinked {} from {}", self.child, self.parent);
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
pub struct Edit {
    /// The human-readable ID of the requirement to edit
    hrid: Hrid,

    /// The new content for the requirement. If not provided, opens $EDITOR
    #[clap(long, short)]
    content: Option<String>,
}

impl Edit {
    #[instrument]
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let new_content = if let Some(content) = self.content {
            content
        } else {
            // Load current content
            let requirement = requiem::Requirement::load(&root, self.hrid.clone())?;

            // Create temp file with current content
            let temp_file = std::env::temp_dir().join(format!("{}.md", self.hrid));
            std::fs::write(&temp_file, requirement.content())?;

            // Open in editor
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let status = std::process::Command::new(&editor)
                .arg(&temp_file)
                .status()?;

            if !status.success() {
                anyhow::bail!("Editor exited with non-zero status");
            }

            // Read back the content
            let content = std::fs::read_to_string(&temp_file)?;
            std::fs::remove_file(&temp_file)?;

            content
        };

        let mut directory = Directory::new(root).load_all()?;
        directory.edit_requirement(self.hrid.clone(), new_content)?;

        println!("Updated requirement {}", self.hrid);
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
