# req - Plain-text Requirements Management

A command-line tool for managing requirements as markdown documents stored in a directory structure.

## Features

- 📝 **Plain-text**: Requirements are markdown files with YAML frontmatter
- 🔗 **Relationships**: Parent-child links form a directed acyclic graph
- 🔍 **Discovery**: List, search, filter by kind, namespace, tags
- 📊 **Validation**: Detect cycles, orphans, invalid links
- 🔄 **Synchronization**: Track changes with fingerprint-based suspect links
- ⚡ **Fast**: Parallel loading with rayon

## Installation

```bash
cargo install requirements-manager
```

## Quick Start

```bash
# Initialize a new requirements repository
req init

# Create a user requirement
req create USR

# List all requirements
req list

# Show a specific requirement
req show USR-001

# Link child to parent
req link SYS-001 USR-001

# Check repository health
req validate
```

## Shell Completions

Shell completions for bash, zsh, fish, and PowerShell are available. Generate and install them using the `complete` command:

### Bash

```bash
# Generate completion script
req complete bash > req.bash

# Install to system completion directory
sudo install -Dm644 req.bash /usr/share/bash-completion/completions/req

# Or manually add to ~/.bashrc
source /path/to/req.bash
```

### Zsh

```bash
# Generate completion script
req complete zsh > _req

# Install to a directory in your fpath
# Option 1: System-wide (if you have sudo access)
sudo install -Dm644 _req /usr/share/zsh/site-functions/_req

# Option 2: User-level
mkdir -p ~/.local/share/zsh/site-functions
mv _req ~/.local/share/zsh/site-functions/

# Make sure the directory is in your fpath (~/.zshrc)
export fpath=($HOME/.local/share/zsh/site-functions $fpath)
```

### Fish

```bash
# Generate completion script
req complete fish > req.fish

# Install to fish completion directory
mkdir -p ~/.config/fish/completions
mv req.fish ~/.config/fish/completions/
```

### PowerShell

```powershell
# Generate completion script
req complete powershell | Out-String | Out-File -FilePath $PROFILE -Append

# Or manually add to your PowerShell profile
req complete powershell >> $PROFILE
```

After installation, start a new shell session or source your shell configuration file for completions to take effect.

## Documentation

See the main repository for full documentation.

## License

MIT
