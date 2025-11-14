# Using with MdBook

[MdBook](https://github.com/rust-lang/mdBook) is a popular tool for creating documentation from Markdown files. Requiem integrates seamlessly with MdBook, allowing requirements to live alongside documentation.

## Overview

Requiem requirements are Markdown files, making them naturally compatible with MdBook. You can:

- Include requirements in your MdBook table of contents
- Embed requirements in documentation chapters
- Mix requirements with narrative documentation
- Generate complete documentation sites with embedded requirements

**HRID in Headings**: Requiem now stores HRIDs in the first markdown heading (e.g., `# USR-001 Title`) rather than in YAML frontmatter. This change was specifically made to improve compatibility with MdBook, as MdBook uses the first heading as the page title.

## Basic Setup

### Project Structure

```
my-book/
├── book.toml          ← MdBook configuration
└── src/
    ├── SUMMARY.md     ← Table of contents
    ├── config.toml    ← Requiem configuration
    ├── chapter1.md    ← Documentation
    ├── USR-001.md     ← Requirement
    └── USR-002.md     ← Requirement
```

### Requiem Configuration

Since the `src/` directory contains both requirements and documentation, configure Requiem to allow non-requirement files:

**src/config.toml**:
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

[build]
build-dir = "book"
```

No special MdBook configuration needed for Requiem requirements!

## Including Requirements in Table of Contents

Requirements can be listed in `SUMMARY.md` like any other chapter:

**src/SUMMARY.md**:
```markdown
# Summary

[Introduction](./introduction.md)

# Requirements

- [User Requirements](./user-requirements.md)
  - [USR-001: User Authentication](./USR-001.md)
  - [USR-002: Data Export](./USR-002.md)
  - [USR-003: Email Validation](./USR-003.md)

- [System Requirements](./system-requirements.md)
  - [SYS-001: Authentication Service](./SYS-001.md)
  - [SYS-002: Export API](./SYS-002.md)

# User Guide

- [Getting Started](./getting-started.md)
- [Features](./features.md)
```

Requirements appear as chapters in the generated book. The HRID in the first heading (e.g., `# USR-001 User Authentication`) becomes the page title automatically.

## Embedding Requirements

Use MdBook's include feature to embed requirements in documentation chapters:

**src/user-guide.md**:
```markdown
# User Guide

## Authentication

Our authentication system satisfies the following requirement:

\{{#include USR-001.md}}

To log in, navigate to...
```

When MdBook builds, `USR-001.md` content is embedded directly.

### Selective Inclusion

Include only the requirement body (skip frontmatter but keep the heading):

```markdown
\{{#include USR-001.md:6:}}
```

This skips the YAML frontmatter and includes the heading and body.

**Line counting**:
```markdown
---                    ← Line 1
_version: '1'
uuid: ...
created: ...
---                    ← Line 5
# USR-001 Title        ← Line 6 (keep this!)

Requirement text...    ← Line 7 onwards
```

Adjust the line number based on your frontmatter length. The heading with the HRID should typically be kept as it provides context.

## Formatting Requirements

### Display Frontmatter

To show frontmatter in documentation:

**src/requirements-format.md**:
```markdown
# Requirement Format

Requirements include metadata in YAML frontmatter:

\{{#include USR-001.md}}
```

MdBook renders the entire file, including frontmatter as a code block.

### Hide Frontmatter

To show only the requirement text:

**Option 1: Use line ranges**
```markdown
\{{#include USR-001.md:7:}}
```

**Option 2: Create requirement summary files**

**src/usr-001-summary.md**:
```markdown
<!-- Manually maintained summary without frontmatter -->
The system shall validate user email addresses according to RFC 5322.
```

Then include the summary file in documentation.

### Custom Formatting with Preprocessors

For advanced formatting (e.g., extracting specific fields), use MdBook preprocessors:

- [mdbook-linkcheck](https://github.com/Michael-F-Bryan/mdbook-linkcheck) - Validate links
- [mdbook-toc](https://github.com/badboy/mdbook-toc) - Generate table of contents
- Custom preprocessor - Extract HRID, UUID, parent links from frontmatter

## Working Example

See the complete example in the Requiem repository:

```bash
git clone https://github.com/danieleades/requirements-manager
cd requirements-manager/examples/mdbook
```

### Example Structure

```
examples/mdbook/
├── book.toml
├── src/
│   ├── SUMMARY.md
│   ├── chapter_1.md
│   ├── USR-001.md
│   └── USR-002.md
└── README.md
```

### Build the Example

1. Install MdBook:
```bash
cargo install mdbook
```

2. Build the book:
```bash
mdbook build
```

3. View output:
```bash
mdbook serve --open
```

Your browser opens showing the documentation with requirements included.

## Best Practices

### 1. Organize by Type

Group requirements in `SUMMARY.md`:

```markdown
# Summary

# User Requirements
- [USR-001](./USR-001.md)
- [USR-002](./USR-002.md)

# System Requirements
- [SYS-001](./SYS-001.md)
- [SYS-002](./SYS-002.md)
```

### 2. Use Subdirectories

For large projects, organize requirements in subdirectories:

```
src/
├── SUMMARY.md
├── requirements/
│   ├── user/
│   │   ├── USR-001.md
│   │   └── USR-002.md
│   └── system/
│       ├── SYS-001.md
│       └── SYS-002.md
└── guides/
    └── user-guide.md
```

**SUMMARY.md**:
```markdown
# Summary

# Requirements
- [User Requirements](./requirements/user/USR-001.md)
- [System Requirements](./requirements/system/SYS-001.md)
```

### 3. Link Requirements

Link between requirements using standard Markdown links:

**USR-001.md**:
```markdown
This requirement is implemented by [SYS-001](./SYS-001.md).
```

MdBook generates clickable links in the output.

### 4. Embed in Context

Embed requirements in relevant documentation sections:

**user-guide.md**:
```markdown
# User Guide

## Email Validation

\{{#include requirements/USR-003.md:7:}}

To validate emails, the system checks...
```

Keeps requirements and documentation synchronized.

### 5. Separate Config

Use separate `config.toml` for Requiem:

```
src/
├── config.toml      ← Requiem config (allow_unrecognised = true)
├── book.toml        ← Don't confuse with MdBook config
└── ...
```

Note: MdBook's config is `book.toml` in the root, not in `src/`.

## Limitations

### Frontmatter Rendering

MdBook doesn't parse YAML frontmatter specially. It renders as:

```
---
_version: '1'
uuid: 4bfeb7d5-...
created: 2025-07-22T12:19:56.950194157Z
---

Requirement text...
```

The frontmatter appears as a code block (triple-dash markers).

**Workaround**: Use line ranges to skip frontmatter (see [Formatting Requirements](#formatting-requirements)).

### No Dynamic Traceability

MdBook doesn't generate traceability matrices or parent-child diagrams automatically.

**Workaround**: Generate diagrams with external tools and include as images:

```markdown
# Traceability

![Requirement Hierarchy](./diagrams/traceability.svg)
```

### No HRID Validation

MdBook doesn't validate requirement references.

**Workaround**: Use CI to validate:

```yaml
# .github/workflows/docs.yml
- name: Validate requirements
  run: req clean
  working-directory: ./src

- name: Build documentation
  run: mdbook build
```

## Advanced Techniques

### Generating Traceability Pages

Use scripts to generate traceability documentation:

**generate-traceability.sh**:
```bash
#!/bin/bash
# Generate a markdown page showing parent-child relationships

echo "# Requirement Traceability" > traceability.md
echo "" >> traceability.md

for req in USR-*.md; do
    hrid=$(basename "$req" .md)
    echo "## $hrid" >> traceability.md
    # Extract and format parent links
    # ...
done
```

Run before MdBook build:

```bash
./generate-traceability.sh
mdbook build
```

### Custom MdBook Preprocessor

Create a preprocessor to enhance requirement rendering:

**Rust preprocessor example**:
```rust
// Extract HRID and UUID from frontmatter
// Add custom formatting
// Generate cross-reference links
```

See [MdBook documentation](https://rust-lang.github.io/mdBook/format/configuration/preprocessors.html) for details.

### GitHub Pages Deployment

Host your requirements documentation:

```yaml
# .github/workflows/deploy.yml
name: Deploy Documentation

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install MdBook
        run: cargo install mdbook
      - name: Build book
        run: mdbook build
        working-directory: ./docs
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book
```

## Example Documentation Structure

Comprehensive documentation with requirements:

```
docs/
├── book.toml
└── src/
    ├── SUMMARY.md
    ├── config.toml             ← Requiem config
    │
    ├── introduction.md          ← Narrative docs
    ├── architecture.md
    ├── getting-started.md
    │
    ├── requirements/            ← Requirements
    │   ├── user/
    │   │   ├── USR-001.md
    │   │   └── USR-002.md
    │   ├── system/
    │   │   ├── SYS-001.md
    │   │   └── SYS-002.md
    │   └── traceability.md      ← Generated
    │
    └── user-guide/              ← User guides
        ├── authentication.md    ← Embeds USR-001, SYS-001
        └── export.md            ← Embeds USR-002, SYS-002
```

## Troubleshooting

### Requirements Not Appearing in Book

**Problem**: Requirements listed in `SUMMARY.md` don't appear.

**Diagnosis**:
1. Check file paths in `SUMMARY.md` are correct
2. Ensure files exist at specified paths
3. Run `mdbook build -v` for verbose output

**Solution**: Verify paths match file structure exactly.

### Frontmatter Renders as Code Block

**Problem**: YAML frontmatter shows as markdown code block.

**Explanation**: Expected behavior. MdBook doesn't parse YAML frontmatter.

**Solution**: Use line ranges to skip frontmatter (see [Selective Inclusion](#selective-inclusion)).

### Requiem Validation Fails

**Problem**: `req clean` reports errors about non-requirement files.

**Solution**: Set `allow_unrecognised = true` in `config.toml`:

```toml
_version = "1"
allow_unrecognised = true
```

### Includes Don't Work

**Problem**: `\{{#include USR-001.md}}` doesn't embed content.

**Diagnosis**:
1. Check file path is correct relative to including file
2. Check MdBook version (includes supported in 0.3.0+)

**Solution**: Use correct relative paths:
```markdown
\{{#include ./USR-001.md}}  # If in same directory
\{{#include ../requirements/USR-001.md}}  # If in subdirectory
```

## Summary

**Key Points**:

- Requiem requirements are compatible with MdBook out of the box
- Set `allow_unrecognised = true` in Requiem config when mixing with docs
- Include requirements in `SUMMARY.md` or embed with `\{{#include}}`
- Use line ranges to skip frontmatter if desired
- Combine requirements with narrative documentation for comprehensive docs

**Benefits**:
- Single source of truth
- Requirements stay synchronized with docs
- Easy navigation and search
- Professional-looking documentation

**Limitations**:
- Frontmatter renders as code block
- No automatic traceability diagrams
- No HRID validation (use external tools)

## Next Steps

- See [Using with Sphinx](./sphinx.md) for Python documentation
- Review [Version Control Best Practices](./version-control.md) for managing requirements in Git
