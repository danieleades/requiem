# Using with MdBook

[MdBook](https://github.com/rust-lang/mdBook) is a popular tool for creating documentation from Markdown files. Requiem integrates with MdBook so that requirements live alongside documentation and publish as part of the same book — with navigation that is generated from the requirements themselves, not maintained by hand.

## Overview

Requiem requirements are Markdown files, making them naturally compatible with MdBook. The integration has three parts:

1. **Generated navigation**: `req export summary` keeps a marked section of `SUMMARY.md` in sync with the requirements on disk
2. **Frontmatter stripping**: the `mdbook-yml-header` preprocessor removes YAML frontmatter from rendered pages
3. **Embedding**: MdBook's `\{{#include}}` can transclude requirements into narrative chapters

This book is itself the working example: the [Example Project](../requirements.md) section's navigation is generated, and every requirement page you can browse here is a real requirement file.

## Basic Setup

### Project Structure

```
my-book/
├── book.toml              ← MdBook configuration
└── src/
    ├── SUMMARY.md         ← Table of contents (with generated section)
    ├── .req/
    │   └── config.toml    ← Requiem configuration
    ├── chapter1.md        ← Documentation
    ├── USR-001.md         ← Requirement
    └── USR-002.md         ← Requirement
```

### Requiem Configuration

Since the `src/` directory contains both requirements and documentation, configure Requiem to allow non-requirement files:

**src/.req/config.toml**:
```toml
_version = "1"
allow_unrecognised = true  # Important: allows chapter1.md, SUMMARY.md, etc.
```

### MdBook Configuration

**book.toml**:
```toml
[book]
title = "My Project Documentation"
authors = ["Your Name"]
language = "en"
src = "src"

# Strips YAML frontmatter from rendered pages
[preprocessor.yml-header]
```

Install the preprocessor alongside MdBook:

```sh
cargo install mdbook mdbook-yml-header@0.1.4
```

## Generated Navigation

MdBook only renders chapters listed in `SUMMARY.md`. Maintaining that list by hand is brittle: every requirement you add, rename, or move needs a matching edit, and the navigation silently rots when you forget.

Instead, mark a region of `SUMMARY.md` as owned by Requiem:

**src/SUMMARY.md**:
```markdown
# Summary

[Introduction](./introduction.md)

# User Guide

- [Getting Started](./getting-started.md)

# Requirements

<!-- requiem:summary:start -->
<!-- requiem:summary:end -->
```

Then generate the navigation:

```sh
req --root src export summary
```

Requiem fills the marked region with an entry for every requirement, grouped by namespace and kind:

```markdown
<!-- requiem:summary:start -->
- [CORE]()
  - [USR]()
    - [CORE-USR-001: Plain Text Storage](./requirements/CORE/USR/001.md)
    - [CORE-USR-002: Unique and Stable Identifiers](./requirements/CORE/USR/002.md)
  - [SYS]()
    - [CORE-SYS-001: Markdown File Format](./requirements/CORE/SYS/001.md)
<!-- requiem:summary:end -->
```

Everything **outside** the markers is never touched — Requiem does not own your book, only the requirements section of it. Hand-written chapters, part titles, and ordering are preserved exactly. If the markers are missing, the command tells you what to add rather than guessing where the section belongs.

The group headers (`[CORE]()`, `[USR]()`) are MdBook [draft chapters](https://rust-lang.github.io/mdBook/format/summary.html): they appear in the navigation as unclickable section labels.

Re-run the command whenever requirements change, or let a file-watcher run it for you. The command is idempotent — if nothing changed, the file is left alone.

### Keeping Navigation Honest in CI

Use `--check` to fail the build when the generated section has drifted (exits with code 2):

```yaml
- name: Check mdBook navigation is up to date
  run: req --root src export summary --check
```

If the file lives somewhere other than `<root>/SUMMARY.md`, point at it explicitly:

```sh
req --root docs/src/requirements export summary --file docs/src/SUMMARY.md --check
```

(This is how this repository's own CI is configured.)

## Frontmatter

MdBook does not parse YAML frontmatter, so without help the metadata block renders as visible text at the top of every requirement page. The [`mdbook-yml-header`](https://crates.io/crates/mdbook-yml-header) preprocessor strips it at build time:

```toml
[preprocessor.yml-header]
```

With the preprocessor enabled, requirement pages render from their first heading (`# USR-001 Title`), which MdBook uses as the page title — the HRID stays visible and linkable.

> **Note**: pin `mdbook-yml-header@0.1.4`; version 0.1.5 is incompatible with current MdBook releases.

## Embedding Requirements

Use MdBook's include feature to embed a requirement in a narrative chapter:

**src/user-guide.md**:
```markdown
# User Guide

## Authentication

Our authentication system satisfies the following requirement:

\{{#include ./requirements/USR-001.md}}

To log in, navigate to...
```

The `yml-header` preprocessor does not process include expansions, so prefer anchors or line ranges that skip the frontmatter when embedding, e.g. `\{{#include ./requirements/USR-001.md:6:}}`. Be aware that line offsets are fragile: adding a parent link or tag to a requirement grows its frontmatter and shifts the line numbers. Where possible, link to the requirement's page instead of transcluding it.

## Linking to Requirements

Link between requirements and documentation using standard Markdown links:

```markdown
This behaviour is specified by [CORE-SYS-001](./requirements/CORE/SYS/001.md).
```

With namespace folders enabled (`subfolders_are_namespaces = true`), a requirement's path is derived from its HRID: `CORE-SYS-001` lives at `CORE/SYS/001.md`. `req sync` keeps files at their canonical paths, and the generated navigation always reflects the current layout.

## Working Example

See the complete example in the Requiem repository:

```bash
git clone https://github.com/danieleades/requiem
cd requiem/examples/mdbook
```

Build it:

```bash
cargo install mdbook mdbook-yml-header@0.1.4
mdbook build
```

To regenerate its navigation after changing requirements:

```bash
req --root src export summary
```

## CI Pipeline

A typical CI job validates the requirements and the navigation before building:

```yaml
- name: Validate requirements
  run: req --root src validate

- name: Check navigation is up to date
  run: req --root src export summary --check

- name: Build documentation
  run: mdbook build
```

## Troubleshooting

### "no '<!-- requiem:summary:start -->' marker found"

**Problem**: `req export summary` refuses to run.

**Explanation**: Requiem only writes into an explicitly marked region, because `SUMMARY.md` usually contains hand-written content it must not overwrite.

**Solution**: Add the marker pair to `SUMMARY.md` where the generated entries should go:

```markdown
<!-- requiem:summary:start -->
<!-- requiem:summary:end -->
```

### Frontmatter Renders as Text

**Problem**: YAML frontmatter shows at the top of requirement pages.

**Solution**: Enable the `yml-header` preprocessor in `book.toml` and install `mdbook-yml-header@0.1.4`.

### Requiem Validation Fails

**Problem**: `req validate` reports errors about non-requirement files.

**Solution**: Set `allow_unrecognised = true` in `src/.req/config.toml`:

```toml
_version = "1"
allow_unrecognised = true
```

### Requirements Not Appearing in Book

**Problem**: Requirement files exist but don't appear in the built book.

**Explanation**: MdBook only renders chapters listed in `SUMMARY.md`.

**Solution**: Run `req export summary` so every requirement is listed in the generated section.

## Summary

**Key Points**:

- Add `<!-- requiem:summary:start/end -->` markers to `SUMMARY.md` and run `req export summary` — never hand-maintain requirement navigation
- Content outside the markers is yours; Requiem never touches it
- Use `req export summary --check` in CI to catch drift
- Use the `mdbook-yml-header` preprocessor (pinned to 0.1.4) to strip frontmatter
- Set `allow_unrecognised = true` in `.req/config.toml` when mixing requirements with docs

**Limitations**:

- Rendered pages don't yet show parent/child traceability links (planned: an `mdbook-requiem` preprocessor rendering traceability footers)
- Embedding via `\{{#include}}` still requires care with frontmatter line offsets

## Next Steps

- See [Using with Sphinx](./sphinx.md) for Python documentation
- Review [Version Control Best Practices](./version-control.md) for managing requirements in Git
