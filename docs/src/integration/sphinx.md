# Using with Sphinx

[Sphinx](https://www.sphinx-doc.org/) is a powerful documentation generator widely used in Python projects. Requiem requirements integrate seamlessly with Sphinx using the MyST Parser extension.

## Overview

Sphinx traditionally uses reStructuredText (RST), but with the [MyST Parser](https://myst-parser.readthedocs.io/) extension, Sphinx can process Markdown files including Requiem requirements.

**Key compatibility features**:
- MyST Parser treats YAML frontmatter as page metadata (doesn't render it)
- HRIDs in headings (e.g., `# USR-001 Title`) work naturally as page titles
- Requirements can be included in toctrees or embedded in documentation

## Setup

### Prerequisites

- Python 3.7 or later
- Sphinx
- MyST Parser extension

### Installation

Using pip:

```bash
pip install sphinx myst-parser
```

Using uv (recommended):

```bash
uv pip install sphinx myst-parser
```

### Project Structure

```
docs/
├── conf.py             ← Sphinx configuration
├── index.md            ← Main documentation
├── requirements/       ← Requirements directory
│   ├── config.toml     ← Requiem configuration
│   ├── USR-001.md
│   ├── USR-002.md
│   ├── SYS-001.md
│   └── SYS-002.md
└── guides/
    └── user-guide.md
```

### Sphinx Configuration

Configure Sphinx to use MyST Parser for Markdown:

**conf.py**:
```python
project = 'My Project'
copyright = '2025, Your Name'
author = 'Your Name'

# Add MyST Parser extension
extensions = ["myst_parser"]

# Exclude non-documentation files
exclude_patterns = [
    '_build',
    'Thumbs.db',
    '.DS_Store',
    'requirements/config.toml',  # Exclude Requiem config
]

html_theme = 'alabaster'  # Or your preferred theme
```

### Requiem Configuration

Since requirements live in a subdirectory with other Sphinx content, configure Requiem appropriately:

**requirements/config.toml**:
```toml
_version = "1"
# Strict mode OK here since requirements are in dedicated directory
allow_unrecognised = false
```

## Including Requirements

### Option 1: Direct Inclusion via Table of Contents

Create a toctree including requirement files:

**index.md**:
```markdown
# Project Documentation

## Contents

\```{toctree}
:maxdepth: 2

guides/user-guide
requirements/USR-001
requirements/USR-002
requirements/SYS-001
\```
```

Requirements appear as pages in generated documentation with the HRID-containing heading as the page title.

### Option 2: Include Directive

Use the MyST `{include}` directive to embed requirements:

**guides/user-guide.md**:
```markdown
# User Guide

## Authentication

Our authentication system satisfies this requirement:

\```{include} ../requirements/USR-001.md
\```
```

The entire requirement (excluding frontmatter) is embedded in the guide.

### Option 3: Literalinclude for Examples

To show requirements as code examples:

**requirements-format.md**:
```markdown
# Requirement Format

Requirements are Markdown files with YAML frontmatter:

\```{literalinclude} requirements/USR-001.md
:language: markdown
\```
```

## HRID in Headings

Requiem stores HRIDs in the first markdown heading (e.g., `# USR-001 Title`) rather than in YAML frontmatter.

### Benefits for Sphinx

- The first heading becomes the natural page title
- Sphinx uses the heading text for navigation and cross-references
- The HRID is visible in the rendered documentation
- No special processing needed to extract the HRID

### Example

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---
# USR-001 Plain Text Storage

Requirements are stored as plain-text files...
```

Sphinx renders this with "USR-001 Plain Text Storage" as the page title. The YAML frontmatter is treated as page metadata and doesn't appear in the rendered output.

## Working Example

The Requiem repository includes a complete Sphinx example:

```bash
git clone https://github.com/danieleades/requirements-manager
cd requirements-manager/examples/sphinx
```

### Build the Example

1. Install dependencies:
```bash
uv pip install -r requirements.txt
```

Or with pip:
```bash
pip install sphinx myst-parser
```

2. Build documentation:
```bash
make html
```

3. View output:
```bash
# Open _build/html/index.html in your browser
```

## Best Practices

### 1. Dedicated Requirements Directory

Keep requirements in a separate directory:

```
docs/
├── conf.py
├── requirements/          ← Requirements
│   ├── config.toml
│   └── *.md
└── guides/               ← Other docs
    └── *.md
```

**Benefits**:
- Clear separation between requirements and narrative docs
- Easier Requiem configuration
- Simpler to manage

### 2. Create Requirement Indexes

Generate index pages for requirement types:

**requirements/user-requirements.md**:
```markdown
# User Requirements

\```{toctree}
:maxdepth: 1

USR-001
USR-002
USR-003
\```
```

### 3. Cross-Reference with Sphinx Roles

Use Sphinx's cross-referencing:

**guides/user-guide.md**:
```markdown
See {doc}`requirements/USR-001` for authentication requirements.
```

Sphinx generates proper links automatically.

### 4. Organize by Type

Structure your toctree to group requirements:

```markdown
\```{toctree}
:maxdepth: 2

User Requirements <requirements/user-requirements>
System Requirements <requirements/system-requirements>
Specifications <requirements/specifications>
\```
```

## Troubleshooting

### MyST Parser Not Found

**Error**: `Extension error: Could not import extension myst_parser`

**Solution**: Install MyST Parser:
```bash
pip install myst-parser
```

### Markdown Files Not Processed

**Problem**: Markdown files don't appear in generated docs.

**Solution**: Ensure MyST Parser is in `extensions` list in **conf.py**:
```python
extensions = ["myst_parser"]
```

### Include Directive Not Working

**Problem**: `{include}` directive doesn't embed content.

**Solution**: Check file path is correct relative to the source file:
```markdown
\```{include} ../requirements/USR-001.md
\```
```

### Requirements Not in Table of Contents

**Problem**: Requirements don't appear in navigation.

**Solution**: Add requirements to a toctree directive in your index or section files.

## Summary

**Key Points**:

- Use MyST Parser extension for Markdown support
- MyST treats YAML frontmatter as metadata (doesn't render it)
- HRIDs in headings work naturally as page titles
- Include requirements via toctree or include directives
- Keep requirements in dedicated directory

**Benefits**:

- Seamless integration with Python documentation
- Requirements alongside API docs and guides
- Professional HTML output with themes
- Cross-referencing and search

**Limitations**:

- Requires MyST Parser (additional dependency)
- Less "native" than MdBook (Sphinx primarily RST-focused)

## Next Steps

- Review the [Sphinx example](https://github.com/danieleades/requirements-manager/tree/main/examples/sphinx) in the repository
- Compare with [Using with MdBook](./mdbook.md) for Rust projects
- See [Version Control Best Practices](./version-control.md) for managing requirements
