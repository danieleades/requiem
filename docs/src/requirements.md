# Example: Requiem's Own Requirements

This section contains Requiem's actual project requirements - the specification that defines what this tool does. These requirements serve a dual purpose:

1. **Project Specification**: Formal requirements that guide Requiem's development
2. **Worked Example**: A real-world demonstration of Requiem managing its own requirements

## Why This Matters

Most requirements management tools provide toy examples with a handful of simple requirements. By using Requiem to manage Requiem's own requirements, we demonstrate:

- **Dogfooding**: We trust our own tool for serious work
- **Real traceability**: User requirements flow down to system requirements
- **Best practices**: Professional structure, clear writing, proper linking
- **Scale**: A complete requirements set, not just a demo
- **Integration**: Requirements living alongside documentation (MdBook)

## Requirements Structure

Requiem's requirements are organized into three namespaces, reflecting the workspace layout:

- **CORE**: the domain and storage library (`req-core`)
- **CLI**: the `req` command-line tool
- **MCP**: the Model Context Protocol server (`req-mcp`)

Within each namespace, requirements follow a hierarchy of kinds:

- **USR** (User Requirements): **what** users need from the tool, e.g. [CORE-USR-001: Plain Text Storage](./requirements/CORE/USR/001.md)
- **SYS** (System Requirements): **how** the system meets user needs, e.g. [CORE-SYS-001: Markdown File Format with YAML Frontmatter](./requirements/CORE/SYS/001.md)
- **SPC** (Specifications): detailed design for features that need it, e.g. [CLI-SPC-002: Requirement Listing CLI Experience](./requirements/CLI/SPC/002.md)
- **DFT** (Defects): recorded deviations between requirements and implementation, e.g. [CORE-DFT-016: MdBook navigation drift](./requirements/CORE/DFT/016.md)

Every requirement is browsable from the sidebar, grouped by namespace and kind. That navigation is **generated** from the requirements themselves with `req export summary`, so it never drifts from the actual set of files - see [Using with MdBook](./integration/mdbook.md).

## Traceability in Action

Notice how system requirements trace back to user requirements. For example, [CORE-USR-006: Requirement Templates](./requirements/CORE/USR/006.md) is decomposed into child system requirements covering [template storage](./requirements/CORE/SYS/011.md), [default application](./requirements/CORE/SYS/012.md), [CLI overrides](./requirements/CLI/SYS/013.md), [template format](./requirements/CORE/SYS/014.md), and [namespace-specific templates](./requirements/CORE/SYS/015.md).

This demonstrates the USR→SYS traceability that Requiem was built to support. Each user need is decomposed into specific technical requirements.

## Using This as a Learning Resource

As you read through the user guide, refer back to these requirements to see concepts in practice:

- **File Format**: See [CORE-SYS-001](./requirements/CORE/SYS/001.md) for the actual specification
- **HRIDs**: See [CORE-SYS-002](./requirements/CORE/SYS/002.md) for identifier format rules
- **Parent Links**: See [CORE-SYS-003](./requirements/CORE/SYS/003.md) for linking structure
- **Fingerprints**: See [CORE-SYS-005](./requirements/CORE/SYS/005.md) for suspect link detection
- **Directory Layout**: See [CORE-SYS-016](./requirements/CORE/SYS/016.md) for the namespace folder modes used here

## Configuration

This requirements directory includes:

- `.req/config.toml`: Requiem configuration (allowed kinds, namespace folders, HRID digits)
- `.req/templates/`: Template files for new requirements (`USR.md`, `SYS.md`, `SPC.md`)

## Exploring Further

Browse the requirements from the sidebar, or start with the overviews:

- [User Requirements Overview](./requirements/user-requirements.md)
- [System Requirements Overview](./requirements/system-requirements.md)

---

**Note**: These requirements are managed using Requiem itself. Any changes go through the same review process documented in [Maintaining Requirements](./maintaining.md).
