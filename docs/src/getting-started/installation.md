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
