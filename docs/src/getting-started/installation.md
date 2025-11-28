# Installation

Requiem is distributed as a Rust crate and can be installed using Cargo, Rust's package manager.

## Prerequisites

You'll need Rust installed on your system. If you don't have it yet:

### Installing Rust

Visit [rustup.rs](https://rustup.rs/) or run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, verify Rust is available:

```bash
rustc --version
cargo --version
```

## Installing Requiem

### From crates.io

Once Rust is installed, install Requiem using Cargo:

```bash
cargo install requirements-manager
```

This will download, compile, and install the `req` command-line tool.

### From Source

To install the latest development version from the GitHub repository:

```bash
cargo install --git https://github.com/danieleades/requirements-manager
```

### Verify Installation

Confirm Requiem is installed correctly:

```bash
req --version
```

You should see output like:

```
req 0.1.0
```

## Shell Completions

Shell completions for bash, zsh, fish, and PowerShell are available. These provide intelligent auto-completion when you press Tab in your shell.

### Setting Up Completions

Generate completion scripts using the `complete` command and install them to your shell's completion directory.

#### Bash

Install completions for bash:

```bash
# Generate and install
req complete bash | sudo tee /usr/share/bash-completion/completions/req > /dev/null

# Or manually if you prefer
req complete bash > req.bash
sudo install -Dm644 req.bash /usr/share/bash-completion/completions/req
```

The next time you open a new bash shell, completions will be available. Test with:

```bash
req <TAB>                    # Shows available commands
req create <TAB>             # Shows command-specific options
req show US<TAB>             # Completes requirement HRIDs
```

#### Zsh

Install completions for zsh:

```bash
# Generate completion script
req complete zsh > _req

# Option 1: System-wide installation (requires sudo)
sudo mv _req /usr/share/zsh/site-functions/

# Option 2: User-level installation (recommended)
mkdir -p ~/.local/share/zsh/site-functions
mv _req ~/.local/share/zsh/site-functions/

# Ensure the directory is in your fpath (~/.zshrc)
export fpath=($HOME/.local/share/zsh/site-functions $fpath)
```

#### Fish

Install completions for fish:

```bash
# Generate and install
req complete fish | tee ~/.config/fish/completions/req.fish > /dev/null
```

#### PowerShell

Install completions for PowerShell:

```powershell
# Append to your PowerShell profile
req complete powershell >> $PROFILE

# Or use Out-File for more control
req complete powershell | Out-String | Out-File -FilePath $PROFILE -Append
```

If you don't have a profile yet, create one:

```powershell
New-Item -Path $PROFILE -Type File -Force
```

### After Installation

Start a new shell session (or source your shell configuration) and try completion:

```bash
# Press Tab after typing
req <TAB>
req create <TAB>
req show USR<TAB>
```

## Getting Help

To see all available commands:

```bash
req --help
```

For help with a specific command:

```bash
req create --help
req link --help
```

## Updating Requiem

To update to the latest version:

```bash
cargo install requirements-manager --force
```

The `--force` flag tells Cargo to reinstall even if a version is already present.

## Uninstalling

To remove Requiem:

```bash
cargo uninstall requirements-manager
```

## Next Steps

Now that Requiem is installed, proceed to the [Quick Start Tutorial](./quick-start.md) to learn the basic commands.
