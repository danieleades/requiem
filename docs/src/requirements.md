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

Requiem's requirements follow a two-level hierarchy:

### User Requirements (USR)
High-level requirements that describe **what** users need from the tool. These focus on user-facing functionality and capabilities.

- [USR-001](./requirements/USR-001.md): Plain Text Storage
- [USR-002](./requirements/USR-002.md): Unique and Stable Identifiers
- [USR-003](./requirements/USR-003.md): Requirement Relationships and Traceability
- [USR-004](./requirements/USR-004.md): Graph Analysis and Validation
- [USR-005](./requirements/USR-005.md): Static Site Generator Integration
- [USR-006](./requirements/USR-006.md): Requirement Templates
- [USR-007](./requirements/USR-007.md): Requirement Visibility and Navigation

[View all user requirements →](./requirements/user-requirements.md)

### System Requirements (SYS)
Detailed technical requirements that describe **how** the system implements user needs. These specify file formats, algorithms, CLI commands, and implementation details.

- [SYS-001](./requirements/SYS-001.md): Markdown File Format with YAML Frontmatter
- [SYS-002](./requirements/SYS-002.md): UUID and HRID Fields
- [SYS-003](./requirements/SYS-003.md): Parent Requirement Links
- ... and 17 more system requirements

[View all system requirements →](./requirements/system-requirements.md)

## Traceability in Action

Notice how system requirements trace back to user requirements. For example:

**USR-006: Requirement Templates** has five child system requirements:
- SYS-011: Template File Storage
- SYS-012: Default Template Application
- SYS-013: Template Override via CLI
- SYS-014: Template Format
- SYS-015: Namespace-Specific Templates

This demonstrates the USR→SYS traceability that Requiem was built to support. Each user need is decomposed into specific technical requirements.

## Using This as a Learning Resource

As you read through the user guide, refer back to these requirements to see concepts in practice:

- **File Format**: See [SYS-001](./requirements/SYS-001.md) for the actual specification
- **HRIDs**: See [SYS-002](./requirements/SYS-002.md) for identifier format rules
- **Parent Links**: See [SYS-003](./requirements/SYS-003.md) for linking structure
- **Fingerprints**: See [SYS-005](./requirements/SYS-005.md) for suspect link detection
- **Templates**: See SYS-011 through SYS-015 for complete template system specification
- **CLI Visibility**: See SYS-017 through SYS-019 for the listing and navigation interface

## Configuration

This requirements directory includes:
- `config.toml`: Requiem configuration (version only)
- `.req/templates/`: Template files for new requirements
  - `USR.md`: Template for user requirements
  - `SYS.md`: Template for system requirements

## Exploring Further

Browse the requirements directly:
- [User Requirements Overview](./requirements/user-requirements.md)
- [System Requirements Overview](./requirements/system-requirements.md)

Or jump to specific requirements using the links above.

---

**Note**: These requirements are managed using Requiem itself. Any changes go through the same review process documented in [Maintaining Requirements](./maintaining.md).
