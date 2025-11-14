# Introduction

Welcome to Requiem, a modern plain-text requirements management tool designed for software and systems engineers who value simplicity, speed, and integration with their existing workflows.

## What is Requiem?

Requiem (package name: `requirements-manager`, CLI: `req`) is a spiritual successor to [Doorstop](https://github.com/doorstop-dev/doorstop), reimagined for the modern development landscape. It enables teams to:

- **Manage requirements as plain text** - Store requirements as markdown files with YAML frontmatter, making them human-readable, version-controllable, and diff-friendly
- **Build traceable requirement hierarchies** - Link requirements together to form directed acyclic graphs (DAGs) that trace from high-level user needs down to detailed specifications
- **Support multiple parents** - Unlike many tools, Requiem allows a single requirement to satisfy multiple parent requirements, reflecting real-world complexity
- **Integrate seamlessly** - Works alongside documentation tools like [Sphinx](https://github.com/sphinx-doc/sphinx) and [MdBook](https://github.com/rust-lang/mdBook), fitting naturally into your existing documentation workflow
- **Scale with performance** - Built in Rust with parallel processing, Requiem is designed to be much, much faster than its predecessors

## When to Use Requirements Management

Requirements management is essential when:

- **Traceability is critical** - Regulated industries (aerospace, medical devices, automotive) often require proof that each requirement is implemented and tested
- **Teams need alignment** - Multiple stakeholders (users, developers, testers, managers) need a shared understanding of what's being built
- **Systems are complex** - Large projects with many interconnected components benefit from formal requirement tracking
- **Change management matters** - Understanding the impact of requirement changes across dependent systems is crucial
- **Documentation must be maintained** - Requirements serve as the foundation for design documents, test plans, and user manuals

## Project Status

Requiem is in **early development** and not yet ready for production use. Current status:

**Implemented:**
- ✅ Manage requirements, specifications, and documents in plain text
- ✅ Create and link requirements with human-readable IDs
- ✅ Support multiple parent relationships
- ✅ Content fingerprinting for change detection
- ✅ Integration with MdBook and Sphinx
- ✅ Parallel loading for performance

**Planned:**
- 🚧 Detect cycles in requirement graphs
- 🚧 Trigger reviews when dependent requirements change
- 🚧 Generate coverage reports (requirement → implementation → test)
- 🚧 Import and export in standard formats

Contributions are welcome! See the [GitHub repository](https://github.com/danieleades/requirements-manager) for more information.

## Design Philosophy

Requiem is built on several core principles:

1. **Plain text first** - Requirements are markdown files that can be read, edited, and reviewed without special tools
2. **Git-friendly** - Every requirement change creates a meaningful diff that's easy to review in pull requests
3. **Dual identifiers** - Stable UUIDs for machine processing, human-readable IDs (like `USR-001`) for people
4. **Fast by default** - Parallel processing and efficient data structures mean Requiem scales to large projects
5. **Composable** - Works alongside your existing documentation tools rather than replacing them
6. **Documentation tool friendly** - HRIDs are stored in markdown headings (e.g., `# USR-001 Title`) for seamless integration with Sphinx and MdBook

## Who This Guide Is For

This guide is designed for:

- **Requirements engineers** managing formal requirement sets
- **Technical writers** documenting software systems
- **Developers** working in regulated environments
- **Project managers** needing traceability and impact analysis
- **QA engineers** mapping requirements to test cases

You should be comfortable with:
- Command-line tools
- Text editors
- Basic version control (Git)
- Markdown formatting

## What's in This Guide

This guide includes:
- **User Guide**: Learn how to use Requiem effectively
- **Reference**: Detailed CLI and file format specifications
- **Example Project**: See Requiem managing its own requirements as a real-world example

The [Example Project](./requirements.md) contains all 21 requirements that define what Requiem does, demonstrating traceability, proper structure, and best practices.

Let's get started!
