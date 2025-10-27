# System Requirements

System requirements (SYS) define the technical implementation details for Requiem. While user requirements describe **what** users need, system requirements specify **how** the tool delivers those capabilities. Some outcomes include dedicated specifications (SPC) that capture detailed interaction or visual design.

## Overview

Requiem has 20 system requirements organized by functional area:

### File Format & Identifiers

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-001](./SYS-001.md) | Markdown File Format with YAML Frontmatter | USR-001 | Files contain YAML metadata block and Markdown body |
| [SYS-002](./SYS-002.md) | UUID and HRID Fields | USR-001, USR-002 | Frontmatter includes UUID, HRID, version, created timestamp |
| [SYS-003](./SYS-003.md) | Parent Requirement Links | USR-003 | Parent links stored as arrays with UUID, HRID, fingerprint |

### Graph Analysis & Validation

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-004](./SYS-004.md) | Cycle Detection in Requirement Graph | USR-004 | Tool validates requirements form a DAG with no cycles |
| [SYS-005](./SYS-005.md) | Suspect Link Detection | USR-004 | Detects when parent fingerprint doesn't match stored value |
| [SYS-008](./SYS-008.md) | Suspect Links CLI Command | USR-004 | `req suspect` command lists all suspect links — spec: [SPC-001](./SPC-001.md) |
| [SYS-009](./SYS-009.md) | Accept Individual Suspect Links | USR-004 | `req accept <child> <parent>` updates one suspect link — spec: [SPC-001](./SPC-001.md) |
| [SYS-010](./SYS-010.md) | Accept All Suspect Links in Bulk | USR-004 | `req accept --all` updates all suspect links with safeguards — spec: [SPC-001](./SPC-001.md) |

### Static Site Integration

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-006](./SYS-006.md) | Sphinx and MyST Parser Compatibility | USR-005 | Requirements compatible with Sphinx + MyST Parser |
| [SYS-007](./SYS-007.md) | MdBook Compatibility | USR-005 | Requirements render correctly in MdBook |

### Template System

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-011](./SYS-011.md) | Template File Storage | USR-006 | Templates stored as markdown files in `.req/templates/` |
| [SYS-012](./SYS-012.md) | Default Template Application | USR-006 | Creating requirement uses template content as default body |
| [SYS-013](./SYS-013.md) | Template Override via CLI | USR-006 | `-t` and `-b` flags override template content |
| [SYS-014](./SYS-014.md) | Template Format | USR-006 | Templates support full CommonMark markdown |
| [SYS-015](./SYS-015.md) | Namespace-Specific Templates | USR-006 | Different templates for same KIND with different namespaces |

### Repository Organisation

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-016](./SYS-016.md) | Directory Structure Modes | — | Toggle between filename and path-based HRID conventions — spec: [SPC-004](./SPC-004.md) |

### CLI Visibility & Navigation

| ID | Title | Parent | Summary |
|----|-------|--------|---------|
| [SYS-017](./SYS-017.md) | Requirements Listing CLI Command | USR-007 | `req list` enumerates requirements with key metadata — spec: [SPC-002](./SPC-002.md) |
| [SYS-018](./SYS-018.md) | Listing Filters and Scopes | USR-007 | Filters restrict results by kind, namespace, tags, text — spec: [SPC-002](./SPC-002.md) |
| [SYS-019](./SYS-019.md) | Relationship Navigation Views | USR-007 | Options expose parents, children, ancestors, descendants — spec: [SPC-002](./SPC-002.md) |
| [SYS-020](./SYS-020.md) | Status Dashboard CLI Command | USR-007 | `req status` shows counts by kind and suspect total — spec: [SPC-003](./SPC-003.md) |
## Detailed Requirements

### File Format & Identifiers

#### SYS-001: Markdown File Format with YAML Frontmatter

Each requirement shall be stored as a single plain-text Markdown file containing a YAML frontmatter block and a Markdown body. Files are named `{HRID}.md` with frontmatter delimited by `---` markers.

[Read full requirement →](./SYS-001.md)

---

#### SYS-002: UUID and HRID Fields

The YAML frontmatter shall include required fields: `_version` (format version), `uuid` (globally unique identifier), `created` (ISO 8601 timestamp). The HRID is derived from the filename.

[Read full requirement →](./SYS-002.md)

---

#### SYS-003: Parent Requirement Links

Parent links shall be stored in a `parents` array, where each link contains `uuid`, `hrid`, and `fingerprint` (SHA256 hash of parent content) for change detection.

[Read full requirement →](./SYS-003.md)

---

### Graph Analysis & Validation

#### SYS-004: Cycle Detection in Requirement Graph

The tool shall validate that requirements form a directed acyclic graph (DAG). Cycles are invalid because they create circular dependencies. Detection uses depth-first traversal.

[Read full requirement →](./SYS-004.md)

---

#### SYS-005: Suspect Link Detection

When a parent's fingerprint doesn't match the stored value in a child's frontmatter, the link is "suspect" - indicating the parent changed. The tool identifies these for review.

[Read full requirement →](./SYS-005.md)

---

#### SYS-008: Suspect Links CLI Command

The `req suspect` command lists all suspect links in the graph, showing child HRID and suspect parent HRID. Exits with non-zero status if suspect links found (useful for CI).

[Read full requirement →](./SYS-008.md)
Specification: [SPC-001](./SPC-001.md)

---

#### SYS-009: Accept Individual Suspect Links

The `req accept <child-hrid> <parent-hrid>` command accepts a specific suspect link by updating the fingerprint in the child's frontmatter to match the parent's current content hash.

[Read full requirement →](./SYS-009.md)
Specification: [SPC-001](./SPC-001.md)

---

#### SYS-010: Accept All Suspect Links in Bulk

The `req accept --all` command accepts all suspect links in bulk. Supports `--dry-run` to preview changes and `--force` to bypass confirmation prompt.

[Read full requirement →](./SYS-010.md)
Specification: [SPC-001](./SPC-001.md)

---

### Static Site Integration

#### SYS-006: Sphinx and MyST Parser Compatibility

Requirement files shall be compatible with Sphinx using the MyST Parser, rendering frontmatter as metadata and body content correctly. YAML syntax must be MyST-compatible.

[Read full requirement →](./SYS-006.md)

---

#### SYS-007: MdBook Compatibility

Requirement files shall render correctly in MdBook. YAML frontmatter is ignored (not rendered), and markdown body content displays properly following CommonMark specification.

[Read full requirement →](./SYS-007.md)

---

### Template System

#### SYS-011: Template File Storage

Templates are stored as individual markdown files in the `.req/templates/` directory, named after requirement kind: `{KIND}.md`. Namespace-specific templates use `{NAMESPACE}-{KIND}.md`.

[Read full requirement →](./SYS-011.md)

---

#### SYS-012: Default Template Application

When creating a requirement via `req add <KIND>`, if a template file exists for that kind, the tool uses the template content as the default body content.

[Read full requirement →](./SYS-012.md)

---

#### SYS-013: Template Override via CLI

The `-t/--title` and `-b/--body` flags allow users to override template content. If either flag is provided, the template is completely ignored.

[Read full requirement →](./SYS-013.md)

---

#### SYS-014: Template Format

Template files contain plain markdown text supporting standard markdown formatting: headings, lists, code blocks, links, etc. No special template syntax required - content is inserted verbatim.

[Read full requirement →](./SYS-014.md)

---

#### SYS-015: Namespace-Specific Templates

Different templates can be configured for the same KIND with different namespaces (e.g., `AUTH-USR.md` vs `USR.md`). Template lookup tries full prefix first, then falls back to KIND only.

[Read full requirement →](./SYS-015.md)

---

### Repository Organisation

#### SYS-016: Directory Structure Modes

Repositories can opt into filename-based or path-based HRID conventions, ensuring teams keep traceability intact while adopting folder structures that fit their workflow.

[Read full requirement →](./SYS-016.md)
Specification: [SPC-004](./SPC-004.md)

---

### CLI Visibility & Navigation

#### SYS-017: Requirements Listing CLI Command

The `req list` command enumerates requirements with key metadata, supporting multiple output formats for human and machine consumption.

[Read full requirement →](./SYS-017.md)
Specification: [SPC-002](./SPC-002.md)

---

#### SYS-018: Listing Filters and Scopes

The listing command provides filters (kind, namespace, tag, text search) and pagination controls so users can focus on relevant subsets.

[Read full requirement →](./SYS-018.md)
Specification: [SPC-002](./SPC-002.md)

---

#### SYS-019: Relationship Navigation Views

Relationship-centric options expose parents, children, ancestors, descendants, and tree views to assist with impact analysis and reviews.

[Read full requirement →](./SYS-019.md)
Specification: [SPC-002](./SPC-002.md)

---

#### SYS-020: Status Dashboard CLI Command

The `req status` command prints requirement counts per kind, reports the overall total, and highlights the suspect link count with a non-zero exit when attention is required.

[Read full requirement →](./SYS-020.md)
Specification: [SPC-003](./SPC-003.md)

---

## Implementation Status

**Implemented** ✅:
- All file format requirements (SYS-001, SYS-002, SYS-003)
- Fingerprints and suspect link detection (SYS-005, SYS-008)
- Static site integration (SYS-006, SYS-007)
- Complete template system (SYS-011 through SYS-015)
- Individual suspect link acceptance (SYS-009)
- Status dashboard command (SYS-020)

**In Progress** 🚧:
- Cycle detection (SYS-004)
- Bulk suspect link acceptance (SYS-010)

**Planned** 📝:
- Requirements listing and navigation (SYS-017, SYS-018, SYS-019)

## Traceability

Each system requirement traces back to one or more user requirements. See the "Parent" column in the tables above, or view the [User Requirements page](./user-requirements.md) for the complete traceability tree.

## Next Steps

- [View User Requirements →](./user-requirements.md)
- [Back to Requirements Overview →](../requirements.md)
