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

## Documentation

See the main repository for full documentation.

## License

MIT
