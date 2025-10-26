use std::path::PathBuf;

mod list;
mod status;

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
            .unwrap_or(Command::Status(Status))
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
    Suspect,

    /// Accept suspect links after review
    ///
    /// Updates fingerprints to mark requirements as reviewed and valid.
    Accept(Accept),

    /// List requirements with filters and relationship views
    List(List),
}

impl Command {
    fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Status(command) => command.run(root)?,
            Self::Add(command) => command.run(root)?,
            Self::Link(command) => command.run(root)?,
            Self::Clean => Clean::run(root)?,
            Self::Suspect => Suspect::run(root)?,
            Self::Accept(command) => command.run(root)?,
            Self::List(command) => command.run(root)?,
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
pub struct Suspect {}

impl Suspect {
    #[instrument]
    fn run(path: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(path).load_all()?;
        let suspect_links = directory.suspect_links();

        if suspect_links.is_empty() {
            println!("No suspect links found.");
            return Ok(());
        }

        println!("Found {} suspect link(s):\n", suspect_links.len());

        for link in &suspect_links {
            println!("  {} → {}", link.child_hrid, link.parent_hrid);
            println!(
                "    Stored fingerprint:  {}",
                &link.stored_fingerprint[..16]
            );
            println!(
                "    Current fingerprint: {}\n",
                &link.current_fingerprint[..16]
            );
        }

        // Exit with non-zero status to indicate suspect links exist (for CI)
        std::process::exit(1);
    }
}

#[derive(Debug, clap::Parser)]
pub struct Accept {
    /// Accept all suspect links
    #[arg(long)]
    all: bool,

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
        let mut directory = Directory::new(path).load_all()?;

        if self.all {
            let updated = directory.accept_all_suspect_links()?;

            if updated.is_empty() {
                println!("No suspect links to accept.");
            } else {
                println!("Accepted {} suspect link(s):", updated.len());
                for (child, parent) in &updated {
                    println!("  {child} → {parent}");
                }
            }
        } else {
            let child = self.child.expect("child is required when --all is not set");
            let parent = self
                .parent
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

#[cfg(test)]
mod tests {
    use super::*;
    use requiem::{Directory, Requirement};
    use tempfile::tempdir;

    fn load_directory(path: &PathBuf) -> Directory<requiem::storage::directory::Loaded> {
        Directory::new(path.clone())
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

        let directory = load_directory(&root.to_path_buf());
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

        Suspect::run(root).expect("suspect should succeed when no links");
    }

    #[test]
    fn accept_run_all_reports_when_no_links_found() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let accept = Accept {
            all: true,
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
