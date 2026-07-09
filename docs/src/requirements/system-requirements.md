# System Requirements

System requirements (SYS) define the technical implementation details for Requiem. While user requirements describe **what** users need, system requirements specify **how** the tool delivers those capabilities. Some outcomes include dedicated specifications (SPC) that capture detailed interaction or visual design, and known deviations from the requirements are recorded as defects (DFT).

The complete, always-current set is in the sidebar, generated directly from the requirements files and grouped by namespace (CORE, CLI, MCP) and kind. This page is a functional map of where to look.

## File Format & Identifiers

How requirements are stored on disk: [CORE-SYS-001](./CORE/SYS/001.md) specifies the Markdown-with-YAML-frontmatter file format, [CORE-SYS-002](./CORE/SYS/002.md) the UUID and HRID identifier fields, and [CORE-SYS-003](./CORE/SYS/003.md) how parent links (UUID, HRID, fingerprint) are recorded.

## Graph Analysis & Validation

The requirement graph must stay healthy: [CORE-SYS-004](./CORE/SYS/004.md) requires cycle detection, and [CORE-SYS-005](./CORE/SYS/005.md) fingerprint-based suspect link detection. The CLI surfaces these through the review workflow ([CLI-SYS-008](./CLI/SYS/008.md), [CLI-SYS-009](./CLI/SYS/009.md), [CLI-SYS-010](./CLI/SYS/010.md) — spec: [CLI-SPC-001](./CLI/SPC/001.md)) and the validate command ([CLI-SYS-028](./CLI/SYS/028.md) — spec: [CLI-SPC-006](./CLI/SPC/006.md)).

## Static Site Integration

Requirements must publish cleanly with documentation tools: [CORE-SYS-006](./CORE/SYS/006.md) covers Sphinx/MyST compatibility, [CORE-SYS-007](./CORE/SYS/007.md) MdBook compatibility, and [CORE-SYS-035](./CORE/SYS/035.md) keeping MdBook navigation synchronized without manual `SUMMARY.md` edits (implemented by `req export summary` — see [Using with MdBook](../integration/mdbook.md)).

## Template System

New requirements start from templates: storage in `.req/templates/` ([CORE-SYS-011](./CORE/SYS/011.md)), default application ([CORE-SYS-012](./CORE/SYS/012.md)), CLI overrides ([CLI-SYS-013](./CLI/SYS/013.md)), template format ([CORE-SYS-014](./CORE/SYS/014.md)), and namespace-specific templates ([CORE-SYS-015](./CORE/SYS/015.md)).

## Repository Organisation

[CORE-SYS-016](./CORE/SYS/016.md) defines the two directory layout modes (full HRID in the filename, or namespace folders — spec: [CLI-SPC-004](./CLI/SPC/004.md)) with path diagnostics in [CLI-SYS-023](./CLI/SYS/023.md), and [CORE-SYS-033](./CORE/SYS/033.md) covers lowercase namespace handling.

## CLI Commands

Lifecycle management without manual file editing: create ([CLI-SYS-022](./CLI/SYS/022.md)), delete ([CLI-SYS-024](./CLI/SYS/024.md)), move ([CLI-SYS-025](./CLI/SYS/025.md)), rename ([CLI-SYS-026](./CLI/SYS/026.md)), unlink ([CLI-SYS-027](./CLI/SYS/027.md)), and HRID/path sync ([CLI-SYS-021](./CLI/SYS/021.md), [CLI-SYS-030](./CLI/SYS/030.md)).

Visibility and navigation: listing ([CLI-SYS-017](./CLI/SYS/017.md)) with filters ([CLI-SYS-018](./CLI/SYS/018.md)) and relationship views ([CLI-SYS-019](./CLI/SYS/019.md)), the status dashboard ([CLI-SYS-020](./CLI/SYS/020.md)), show ([CLI-SYS-031](./CLI/SYS/031.md)), and graph visualization ([CLI-SYS-032](./CLI/SYS/032.md)). Detailed CLI experience is specified in [CLI-SPC-002](./CLI/SPC/002.md), [CLI-SPC-003](./CLI/SPC/003.md), and [CLI-SPC-005](./CLI/SPC/005.md). Kind management is covered by [CLI-SYS-029](./CLI/SYS/029.md).

## MCP Server

The Model Context Protocol server exposes the same data to AI agents: self-documenting tools ([MCP-SYS-001](./MCP/SYS/001.md)), kind metadata ([MCP-SYS-002](./MCP/SYS/002.md)), implementation status ([MCP-SYS-003](./MCP/SYS/003.md)), consistent responses ([MCP-SYS-004](./MCP/SYS/004.md)), and live directory reload ([MCP-SYS-005](./MCP/SYS/005.md)).

## Next Steps

- [View User Requirements →](./user-requirements.md)
- [Back to Requirements Overview →](../requirements.md)
