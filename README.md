# Requiem

[![codecov](https://codecov.io/gh/danieleades/requiem/graph/badge.svg?token=xZLcLKU4D8)](https://codecov.io/gh/danieleades/requiem)
[![Continuous integration](https://github.com/danieleades/requirements/actions/workflows/CI.yml/badge.svg)](https://github.com/danieleades/requirements/actions/workflows/CI.yml)

Requiem is a plain-text requirements management tool. It is a spiritual successor to [Doorstop](https://github.com/doorstop-dev/doorstop), but aims to-

- be much, much faster
- support multiple parents per requirement
- integrate with existing plain-text documentation tools such as [Sphinx](https://github.com/sphinx-doc/sphinx) and [MdBook](https://github.com/rust-lang/mdBook).

This project is in its early stages and not ready for production use yet. There will be bugs and errors in the documentation. Contributions are very welcome.

This project also serves as an experiment in using structured specifications to drive AI agent based development workflows. To that end, this project also provides a Model Context Protocol (MCP) server in `req-mcp/` which exposes the project requirements to AI agents such as [Claude](https://claude.ai) and [GPT-4](https://openai.com). See `req-mcp/README.md` for details.

Workspace layout:

- `req-core`: domain and storage library (HRIDs, Directory, fingerprinting)
- `req`: CLI binary (`req`)
- `req-mcp`: Model Context Protocol server (read-only MVP; editing tools stubbed)
- `docs/`: mdBook docs and the project's own requirements (`docs/src/requirements/`)

Naming note:

The name of the package is `requirements-manager`, but the name of this project is `Requiem` (a contraction). The tool is invoked on the command line as `req`.

## Features

- [x] Manage requirements as Markdown with YAML frontmatter (UUIDs, timestamps, tags, parents)
- [x] HRIDs stored in headings (Sphinx/MdBook friendly) with optional namespaces and configurable digit width
- [x] Multiple parents per requirement with fingerprint-based suspect link detection
- [x] Templates for new requirements in `.req/templates/` (matched by kind or namespace)
- [x] Sync HRID drift and path drift (`req sync`), detailed path diagnostics (`req sync --apply`)
- [x] Rich querying (`req list`, `req show`) with filters, relationship views, and machine-readable output
- [x] Cycle detection, structural validation, and broken-reference checks (`req validate`)
- [ ] Coverage reports; import/export in standard formats

## File Format

Requirements are stored as Markdown files with YAML frontmatter. The HRID (Human-Readable ID) appears as the first token in the document's title, making requirements compatible with Sphinx and MdBook:

```markdown
---
_version: '1'
uuid: 12345678-1234-5678-1234-567812345678
created: 2025-01-01T12:00:00Z
tags:
- api
- auth
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
If you're using an AI agent, see the MCP server guide in `req-mcp/README.md` to connect the project requirements directly to your client.

## Installation

```sh
cargo install requirements-manager
```

## CLI

The most up-to-date reference is available via `--help`:

```sh
req --help
```

Quick start:

```sh
# Initialize a repository (creates .req/config.toml and templates folder)
req init

# Register kinds (optional; skip if you allow all kinds)
req kind add USR SYS

# Create requirements (HRIDs are assigned automatically)
req create USR --title "User Login" --body "Users shall be able to log in"
req create SYS --parent USR-001 --title "Authentication Service"

# Inspect and navigate
req status               # default command: counts + suspect/path drift summary
req list --view parents  # filter/list with relationship views
req show USR-001         # pretty detail view

# Review fingerprint drift (parents changed since linking)
req review               # exits 2 if suspects exist
req review --accept --all --yes

# Keep metadata tidy
req sync                 # update stored parent HRIDs
req sync --what paths    # move files to canonical locations
req diagnose paths       # print detailed path drift issues
req validate             # currently checks path/HRID drift and suspect links
```

## Project Requirements (Dogfooding)

This repository manages its own requirements under `docs/src/requirements/`. Common tasks:

```sh
cargo run -r -- -r docs/src/requirements status
cargo run -r -- -r docs/src/requirements list
cargo run -r -- -r docs/src/requirements review
```

---

*Was this useful? [Buy me a coffee](https://github.com/sponsors/danieleades/sponsorships?sponsor=danieleades&preview=true&frequency=recurring&amount=5)*
