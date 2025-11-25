use std::path::Path;

use tracing::instrument;

mod add;
mod list;
mod remove;

#[derive(Debug, clap::Parser)]
pub struct Command {
    #[command(subcommand)]
    command: KindCommand,
}

#[derive(Debug, clap::Parser)]
enum KindCommand {
    /// Add one or more requirement kinds to the allowed list
    Add(add::Command),

    /// Remove one or more requirement kinds from the allowed list
    Remove(remove::Command),

    /// List all registered requirement kinds
    List,
}

impl Command {
    #[instrument]
    pub fn run(self, root: &Path) -> anyhow::Result<()> {
        let config_path = root.join(".req/config.toml");

        match self.command {
            KindCommand::Add(add) => add.run(&config_path),
            KindCommand::Remove(remove) => remove.run(&config_path, root),
            KindCommand::List => list::run(&config_path),
        }
    }
}
