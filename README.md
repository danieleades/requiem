# Requiem

[![codecov](https://codecov.io/gh/danieleades/requirements/graph/badge.svg?token=xZLcLKU4D8)](https://codecov.io/gh/danieleades/requirements)
[![Continuous integration](https://github.com/danieleades/requirements/actions/workflows/CI.yml/badge.svg)](https://github.com/danieleades/requirements/actions/workflows/CI.yml)

Requiem is a plain-text requirements management tool. It is a spiritual successor to [Doorstop](https://github.com/doorstop-dev/doorstop), but aims to-

- be much, much faster
- support multiple parents per requirement
- integrate with existing plain-text documentation tools such as [Sphinx](https://github.com/sphinx-doc/sphinx) and [MdBook](https://github.com/rust-lang/mdBook).

This project is in its early stages, and is not yet ready for production use. It is currently being developed as a personal project, but contributions are welcome.

A note on naming:

The name of the package is `requirements-manager`, but the name of this project is `Requiem` (a contraction). The tool is invoked on the command line as `req`.

## Features

- [x] Manage requirements, specifications, and other documents in plain text (Markdown with YAML frontmatter)
- [x] Link documents together to form a directed acyclic graph (DAG)
- [x] Support multiple parent requirements per document
- [x] Detect cycles in the graph and report them
- [x] Trigger reviews when parent requirements are changed (suspect link detection via fingerprinting)
- [x] Human-readable IDs (HRIDs) stored in document titles for Sphinx/MdBook compatibility
- [x] Flexible directory organization (filename-based or path-based with namespaces)
- [x] Template system for new requirements
- [x] Path diagnostics and validation
- [ ] Generate coverage reports
- [ ] Import and export requirements in standard formats

## File Format

Requirements are stored as Markdown files with YAML frontmatter. The HRID (Human-Readable ID) appears as the first token in the document's title, making requirements compatible with Sphinx and MdBook:

```markdown
---
_version: '1'
uuid: 12345678-1234-5678-1234-567812345678
created: 2025-01-01T12:00:00Z
parents:
- uuid: parent-uuid-here
  fingerprint: sha256-hash-of-parent-content
  hrid: USR-001
---
# SYS-001 Authentication Service

## Statement

The system shall provide an authentication service...
```

Key features:
- **HRID in title**: First token of the first heading (e.g., `# USR-001 Title`)
- **UUID**: Immutable identifier for internal linking
- **Fingerprinting**: SHA256 hashes detect when parent requirements change
- **Multiple parents**: Requirements can trace to multiple parent requirements

## Documentation

For comprehensive documentation, visit the [Requiem documentation site](https://danieleades.github.io/requirements/).

## Installation

```sh
cargo install requirements-manager
```

## Cli

The most up-to-date documentation for the command line interface can be found by running:

```sh
req --help
```

Quick start:

Requiem does not require a dedicated initialization command—create a directory (optionally a Git repository) and start adding requirements.

```sh
# Create a new requirements repository directory
mkdir my-requirements && cd my-requirements

# (Optional) create git repository alongside your requirements
# git init && git commit --allow-empty -m "Start requirements repo"

# Add a couple of user requirements
req add USR --title "User Login" --body "Users shall be able to log in"
# Creates USR-001.md with HRID in the title: # USR-001 User Login

req add USR --title "Password Reset" --body "Users shall be able to reset passwords"
# Creates USR-002.md

# Add a system requirement that implements a user requirement
req add SYS --parent USR-001 --title "Authentication Service"
# Creates SYS-001.md with parent link to USR-001

# Link an existing requirement to a parent
req link SYS-001 USR-002

# View repository status at any time
req status

# Check for suspect links (parent requirements that have changed)
req suspect

# Accept and update suspect links after review
req accept SYS-001 USR-001

# Validate that files are in the correct locations
req diagnose paths

# Clean up outdated parent HRIDs after moving requirements
req clean
```

---

*Was this useful? [Buy me a coffee](https://github.com/sponsors/danieleades/sponsorships?sponsor=danieleades&preview=true&frequency=recurring&amount=5)*
