# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Requiem (package: `requirements-manager`, CLI: `req`) is a plain-text requirements management tool. It's a spiritual successor to Doorstop, designed to be much faster and support multiple parents per requirement. Requirements are stored as markdown files with YAML frontmatter, identified by Human-Readable IDs (HRIDs) like `USR-001` or `COMPONENT-SYS-005`.

## Build and Test Commands

```bash
# Build the project
cargo build

# Run all tests
cargo test --all-features

# Run a single test by name
cargo test <test_name>

# Run tests in a specific file
cargo test --test <test_file_name>

# Run doctests
cargo test --doc

# Format code (requires nightly for project-specific rustfmt config)
cargo +nightly fmt

# Lint with clippy
cargo clippy --all-features --all-targets

# Generate documentation
cargo doc --no-deps

# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench add_many
cargo bench --bench update_hrids

# Check license compliance
cargo deny check
```

## Running CLI Commands in Development

**IMPORTANT**: When working on this codebase, always use `cargo run --` to execute CLI commands. This ensures you're testing the current development version rather than an installed version.

```bash
# Correct - uses current codebase
cargo run -- --help
cargo run -- add USR
cargo run -- link SYS-001 USR-001
cargo run -- clean
cargo run -- -v add SYS --parents USR-001

# Avoid - uses installed version (may be outdated)
req --help
req add USR
```

When working in the `reqs/` directory or other subdirectories, specify the root:

```bash
cd reqs/
cargo run -- --root . add USR
cargo run -- --root . link SYS-001 USR-001
```

## Code Architecture

### Domain Layer (`src/domain/`)

The core domain model consists of:

- **`Requirement`** (`requirement.rs`): The central entity representing a requirement document. Contains:
  - `Content`: The markdown body and tags (contributes to fingerprint for change detection)
  - `Metadata`: UUID (stable), HRID (human-readable), creation timestamp, and parent relationships
  - Requirements support multiple parents via a HashMap keyed by UUID
  - Fingerprinting via SHA256 of borsh-serialized content enables change detection for review triggers

- **`Hrid`** (`hrid.rs`): Human-Readable ID with format `{NAMESPACE*}-{KIND}-{ID}`
  - Examples: `USR-001`, `SYS-042`, `COMPONENT-SUBCOMPONENT-SYS-005`
  - Namespaces are optional and can be nested
  - IDs are zero-padded integers (default 3 digits)
  - Implements `FromStr` and `Display` for parsing/formatting

- **`Config`** (`config.rs`): TOML-based configuration supporting versioned schemas
  - Controls allowed requirement kinds, HRID digit padding
  - Flags for allowing unrecognised/invalid markdown files
  - Versioned serialization format for future compatibility

### Storage Layer (`src/storage/`)

Filesystem abstraction with typestate pattern:

- **`Directory<S>`** (`directory.rs`): Filesystem-backed requirement store
  - `Directory<Unloaded>`: Initial state, can load requirements from disk
  - `Directory<Loaded>`: Contains a `Tree`, supports adding/linking requirements
  - Uses parallel loading via rayon for performance
  - Respects `config.toml` in requirements root directory

- **`Tree`** (`tree.rs`): In-memory requirement graph, filesystem-agnostic
  - Stores requirements in `Vec` with UUID-to-index HashMap
  - Tracks next available ID per requirement kind
  - `update_hrids()` method corrects parent HRIDs after renaming

### CLI Layer (`src/cli.rs`)

Three main commands:
- `add <KIND> [--parent HRID,...]`: Create new requirement with optional parent links
- `link <CHILD> <PARENT>`: Create parent-child relationship
- `clean`: Correct parent HRIDs across all requirements

Supports `-v` flags for increasing verbosity (WARN/INFO/DEBUG/TRACE).

## Key Design Patterns

**Dual Identifiers**: Requirements use both UUIDs (stable, internal) and HRIDs (human-readable, potentially changing). Parent relationships are keyed by UUID but store HRID for user-facing references.

**Fingerprinting**: Content changes generate new SHA256 fingerprints. Parent links store fingerprints to detect when upstream changes should trigger reviews (future feature).

**Typestate Pattern**: `Directory<Unloaded>` → `Directory<Loaded>` ensures requirements are loaded before operations that need them.

**Parallel Processing**: Uses rayon for concurrent file loading and HRID updates to maintain performance at scale.

## Testing Notes

- Unit tests are co-located in each module using `#[cfg(test)]`
- Tests cover HRID parsing roundtrips, fingerprint stability, and validation edge cases
- Use `cargo test <module_name>::tests::<test_name>` to run specific tests

## Integration with Documentation Tools

The project supports integration with Sphinx and MdBook (see `examples/`). CI validates these integrations. Requirements can be referenced in documentation via their HRIDs.

## Linting Standards

Cargo.toml enforces strict linting:
- `clippy::pedantic`, `clippy::nursery`, `clippy::cargo` all set to `deny`
- `missing_docs` warns on undocumented public items
- Rustfmt config uses 2021 style edition with aggressive formatting options
