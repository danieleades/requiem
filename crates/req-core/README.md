# req-core

Core types and logic for plain-text requirements management.

This crate provides the foundational domain models and storage abstractions for managing requirements as markdown documents in a directory structure.

## Key Types

- `Hrid` - Human-readable identifiers (e.g., `USR-001`, `auth-SYS-042`)
- `Requirement` - Requirements with metadata and content
- `Tree` - In-memory graph of requirements relationships
- `Directory` - Filesystem storage with parallel loading
- `Config` - Repository configuration

## Features

- Requirements stored as markdown with YAML frontmatter
- Parent-child relationships forming a DAG
- Fingerprint-based change detection
- Parallel loading with `rayon`
- Path mode and filename mode support

## Usage

See the parent `requirements-manager` crate for CLI usage, or `req-mcp` for MCP server usage.
