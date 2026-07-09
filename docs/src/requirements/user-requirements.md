# User Requirements

User requirements (USR) define the high-level capabilities that Requiem must provide to its users. These requirements focus on **what** the tool enables users to accomplish, from the user's perspective.

Each user requirement is decomposed into system requirements (SYS) that specify **how** the capability is delivered. The full, always-current list of requirements is in the sidebar, generated directly from the requirements files - what follows is a guided tour of the major themes.

## Core Library (CORE)

The CORE namespace captures what the requirements engine itself must do:

- [CORE-USR-001: Plain Text Storage](./CORE/USR/001.md) - requirements are plain-text files readable in any editor
- [CORE-USR-002: Unique and Stable Identifiers](./CORE/USR/002.md) - dual identifiers: UUIDs for machines, HRIDs for humans
- [CORE-USR-003: Requirement Relationships and Traceability](./CORE/USR/003.md) - parent-child relationships forming traceable hierarchies
- [CORE-USR-004: Graph Analysis and Validation](./CORE/USR/004.md) - cycle detection and change impact analysis
- [CORE-USR-005: Static Site Generator Integration](./CORE/USR/005.md) - requirements publishable with MdBook and Sphinx
- [CORE-USR-006: Requirement Templates](./CORE/USR/006.md) - templates for consistent new requirements
- [CORE-USR-007: Requirement Visibility and Navigation](./CORE/USR/007.md) - locating and summarizing requirements without opening files
- [CORE-USR-008: Directory Organization Flexibility](./CORE/USR/008.md) - filename-based or namespace-folder layouts
- [CORE-USR-012: Lowercase Namespace Support](./CORE/USR/012.md) - case-insensitive namespace handling

## Command Line Interface (CLI)

The CLI namespace captures what the `req` tool must let users do:

- [CLI-USR-009: Command Line Interface Lifecycle Management](./CLI/USR/009.md) - create, delete, move, and rename requirements without manual file editing
- [CLI-USR-010: Repository Validation and Health](./CLI/USR/010.md) - a unified command for comprehensive repository health checking
- [CLI-USR-011: Requirement Kind Management](./CLI/USR/011.md) - explicit CLI management of requirement kinds

## MCP Server (MCP)

The MCP namespace covers the Model Context Protocol server that exposes requirements to AI agents:

- [MCP-USR-001: Self-Documenting Requirements API](./MCP/USR/001.md) - tools an agent can discover and use without external documentation

## Tracing Down to System Requirements

Every user requirement lists its children on its page (and each child links back to its parents in its frontmatter). For example, [CORE-USR-004: Graph Analysis and Validation](./CORE/USR/004.md) is implemented by system requirements for [cycle detection](./CORE/SYS/004.md), [suspect link detection](./CORE/SYS/005.md), and the [review](./CLI/SYS/008.md)/[accept](./CLI/SYS/009.md) commands.

To explore the graph interactively, use the CLI:

```sh
req --root docs/src/requirements list --kind USR
req --root docs/src/requirements show CORE-USR-004 --children
```

## Next Steps

- [View System Requirements →](./system-requirements.md)
- [Back to Requirements Overview →](../requirements.md)
