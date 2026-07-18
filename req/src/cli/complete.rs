//! Generate shell completions for the req CLI.
//!
//! This command generates shell completion scripts for bash, zsh, fish, and
//! `PowerShell`. The generated scripts should be saved to the appropriate shell
//! configuration directory.

use std::io;

use clap::CommandFactory;
use clap_complete::Shell;

/// Generate shell completion scripts
#[derive(Debug, clap::Parser)]
pub struct Command {
    /// Shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

impl Command {
    /// Generate and output the completion script to stdout.
    pub fn run(self) {
        let mut cmd = crate::cli::Cli::command();
        clap_complete::generate(self.shell, &mut cmd, "req", &mut io::stdout());
    }
}
