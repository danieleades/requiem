# User Requirements

User requirements (USR) define the high-level capabilities that Requiem must provide to its users. These requirements focus on **what** the tool enables users to accomplish, from the user's perspective.

## Overview

Requiem has 11 user requirements that establish the core value proposition:

| ID | Title | Summary |
|----|-------|---------|
| [USR-001](./USR-001.md) | Plain Text Storage | Requirements stored as plain text files readable in any editor |
| [USR-002](./USR-002.md) | Unique and Stable Identifiers | Dual identifiers: UUIDs for machines, HRIDs for humans |
| [USR-003](./USR-003.md) | Requirement Relationships and Traceability | Parent-child relationships forming traceable hierarchies |
| [USR-004](./USR-004.md) | Graph Analysis and Validation | Cycle detection and change impact analysis |
| [USR-005](./USR-005.md) | Static Site Generator Integration | Compatibility with MdBook and Sphinx |
| [USR-006](./USR-006.md) | Requirement Templates | Support for templates when creating new requirements |
| [USR-007](./USR-007.md) | Requirement Visibility and Navigation | CLI listing and graph exploration for working sets |
| [USR-009](./USR-009.md) | Command Line Interface Lifecycle Management | CLI commands for complete requirement lifecycle without manual editing |
| [USR-010](./USR-010.md) | Repository Validation and Health | Unified command for comprehensive repository health checking |
| [USR-011](./USR-011.md) | Requirement Kind Management | CLI commands for managing requirement kinds explicitly |

## Detailed Requirements

### USR-001: Plain Text Storage

The tool shall store requirements as plain-text files that can be read and edited with any text editor.

**Why this matters**: Plain text enables version control integration, human review without specialized software, long-term archival, and integration with existing text-based workflows.

**Child requirements**: SYS-001, SYS-002

[Read full requirement →](./USR-001.md)

---

### USR-002: Unique and Stable Identifiers

The tool shall assign each requirement both a UUID (for technical stability) and a human-readable ID/HRID (for human reference).

**Why this matters**: UUIDs prevent conflicts and enable merging, while HRIDs like "USR-001" make requirements easy to discuss, link, and remember.

**Child requirements**: SYS-002

[Read full requirement →](./USR-002.md)

---

### USR-003: Requirement Relationships and Traceability

The tool shall support defining parent-child relationships between requirements, enabling traceability from high-level needs to detailed specifications.

**Why this matters**: Traceability is essential for understanding requirement flow, impact analysis, coverage verification, and compliance in regulated industries.

**Child requirements**: SYS-003

[Read full requirement →](./USR-003.md)

---

### USR-004: Graph Analysis and Validation

The tool shall analyze the requirement graph to detect invalid structures (cycles) and identify the impact of changes.

**Why this matters**: Requirements form a directed acyclic graph (DAG). Cycles are errors. Impact analysis shows what's affected when requirements change.

**Child requirements**: SYS-004, SYS-005, SYS-008, SYS-009, SYS-010

[Read full requirement →](./USR-004.md)

---

### USR-005: Static Site Generator Integration

The tool shall integrate with documentation frameworks (Sphinx, MdBook) so requirements can be included in generated documentation.

**Why this matters**: Requirements shouldn't live in isolation. They should integrate with design docs, user guides, and API documentation in a single published site.

**Child requirements**: SYS-006, SYS-007

[Read full requirement →](./USR-005.md)

---

### USR-006: Requirement Templates

The tool shall support defining templates that provide default content and structure for new requirements.

**Why this matters**: Templates ensure consistency, reduce boilerplate, encode best practices, and make requirement creation faster.

**Child requirements**: SYS-011, SYS-012, SYS-013, SYS-014, SYS-015

[Read full requirement →](./USR-006.md)

---

### USR-007: Requirement Visibility and Navigation

The tool shall let users quickly locate, inspect, and summarize requirements from the command line without opening individual files.

**Why this matters**: Reviewers and systems engineers need instant visibility into requirement sets during audits, change control, and release prep. Native listing and navigation commands eliminate manual grepping and ad-hoc spreadsheets.

**Child requirements**: SYS-017, SYS-018, SYS-019

[Read full requirement →](./USR-007.md)

---

### USR-009: Command Line Interface Lifecycle Management

The tool shall provide CLI commands for managing the complete lifecycle of requirements (create, delete, move, rename) without requiring manual file editing.

**Why this matters**: Manual file editing is error-prone and can break metadata integrity. CLI commands enforce invariants, prevent data corruption, and enable automation in CI/CD pipelines.

**Child requirements**: SYS-024, SYS-025, SYS-026, SYS-027

[Read full requirement →](./USR-009.md)

---

### USR-010: Repository Validation and Health

The tool shall provide a unified CLI command for validating repository health and integrity across all requirements.

**Why this matters**: Requirements repositories can become inconsistent through manual editing, file moves, deletions, or circular dependencies. A single unified validation command makes it clear where to check for problems and is essential for CI/CD gating.

**Child requirements**: SYS-028

[Read full requirement →](./USR-010.md)

---

### USR-011: Requirement Kind Management

The tool shall provide CLI commands for managing requirement kinds (types) explicitly.

**Why this matters**: Explicit kind management commands make the tool more discoverable and consistent, eliminating the need for manual config file editing and reducing errors.

**Child requirements**: SYS-029

[Read full requirement →](./USR-011.md)

---

## Traceability

Each USR requirement traces down to one or more SYS (system) requirements that provide technical implementation details:

```
USR-001 (Plain Text Storage)
  ├── SYS-001 (Markdown File Format with YAML Frontmatter)
  └── SYS-002 (UUID and HRID Fields)

USR-002 (Unique and Stable Identifiers)
  └── SYS-002 (UUID and HRID Fields)

USR-003 (Requirement Relationships and Traceability)
  └── SYS-003 (Parent Requirement Links)

USR-004 (Graph Analysis and Validation)
  ├── SYS-004 (Cycle Detection in Requirement Graph)
  ├── SYS-005 (Suspect Link Detection)
  ├── SYS-008 (Suspect Links CLI Command)
  ├── SYS-009 (Accept Individual Suspect Links)
  └── SYS-010 (Accept All Suspect Links in Bulk)

USR-005 (Static Site Generator Integration)
  ├── SYS-006 (Sphinx and MyST Parser Compatibility)
  └── SYS-007 (MdBook Compatibility)

USR-006 (Requirement Templates)
  ├── SYS-011 (Template File Storage)
  ├── SYS-012 (Default Template Application)
  ├── SYS-013 (Template Override via CLI)
  ├── SYS-014 (Template Format)
  └── SYS-015 (Namespace-Specific Templates)

USR-007 (Requirement Visibility and Navigation)
  ├── SYS-017 (Requirements Listing CLI Command)
  ├── SYS-018 (Listing Filters and Scopes)
  ├── SYS-019 (Relationship Navigation Views)
  ├── SYS-020 (Status Dashboard CLI Command)
  ├── SYS-031 (Show Command)
  └── SYS-032 (Graph Visualization Command)

USR-009 (Command Line Interface Lifecycle Management)
  ├── SYS-024 (Delete Requirement Command)
  ├── SYS-025 (Move Requirement Command)
  ├── SYS-026 (Rename Requirement Command)
  └── SYS-027 (Unlink Command)

USR-010 (Repository Validation and Health)
  └── SYS-028 (Validate Command)

USR-011 (Requirement Kind Management)
  └── SYS-029 (Kind Management Command)
```

This hierarchy demonstrates requirement decomposition from user needs down to technical implementation.

## Next Steps

- [View System Requirements →](./system-requirements.md)
- [Back to Requirements Overview →](../requirements.md)
