# Using with Sphinx

[Sphinx](https://www.sphinx-doc.org/) is a powerful documentation generator widely used in Python projects. Requiem requirements can be integrated with Sphinx documentation.

## Overview

Sphinx traditionally uses reStructuredText (RST), but with the [MyST Parser](https://myst-parser.readthedocs.io/) extension, Sphinx can process Markdown files including Requiem requirements.

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

# Optional: Configure MyST Parser
myst_enable_extensions = [
    "colon_fence",
    "deflist",
    "tasklist",
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

**index.md** (or **index.rst**):
```markdown
# Project Documentation

## Contents

```{toctree}
:maxdepth: 2

guides/user-guide
requirements/USR-001
requirements/USR-002
requirements/SYS-001
\```
```

Requirements appear as pages in generated documentation.

### Option 2: Include Directive

Use the MyST `{include}` directive to embed requirements:

**guides/user-guide.md**:
```markdown
# User Guide

## Authentication

Our authentication system satisfies this requirement:

```{include} ../requirements/USR-001.md
:start-line: 6
\```

The `:start-line: 6` skips the YAML frontmatter (adjust based on your frontmatter length).
```

### Option 3: Literalinclude for Requirements

To show requirements as examples:

**requirements-format.md**:
```markdown
# Requirement Format

Requirements are Markdown files with YAML frontmatter:

```{literalinclude} requirements/USR-001.md
:language: markdown
\```
```

## Working Example

The Requiem repository includes a complete Sphinx example:

```bash
git clone https://github.com/danieleades/requirements-manager
cd requirements-manager/examples/sphinx
```

### Example Structure

```
examples/sphinx/
├── conf.py
├── index.md
├── requirements/
│   ├── USR-001.md
│   └── USR-002.md
├── pyproject.toml
├── Makefile
└── README.md
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
uv run make html
```

Or:
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
└── source/               ← Other docs
    └── *.md
```

**Benefits**:
- Clear separation
- Easier Requiem configuration
- Simpler to manage

### 2. Use MyST Directives

Leverage MyST's powerful directives:

**Include with line selection**:
```markdown
```{include} requirements/USR-001.md
:start-line: 7
:end-line: 20
\```
```

**Add captions**:
```markdown
```{literalinclude} requirements/USR-001.md
:language: markdown
:caption: USR-001: Email Validation Requirement
\```
```

### 3. Create Requirement Indexes

Generate index pages for requirement types:

**requirements/user-requirements.md**:
```markdown
# User Requirements

```{toctree}
:maxdepth: 1

USR-001
USR-002
USR-003
\```
```

### 4. Cross-Reference with Sphinx Roles

Use Sphinx's cross-referencing:

**guides/user-guide.md**:
```markdown
See {doc}`requirements/USR-001` for authentication requirements.
```

Sphinx generates proper links.

### 5. Integrate with Autodoc

For Python projects, link code documentation to requirements:

**api/auth.py docstring**:
```python
def validate_email(email: str) -> bool:
    """Validate email address.

    Implements requirement :doc:`requirements/USR-001`.
    """
    ...
```

## HRID in Headings - Sphinx Compatibility

**Important**: Requiem now stores HRIDs in the first markdown heading (e.g., `# USR-001 Title`) rather than in the YAML frontmatter. This change was specifically made to improve compatibility with Sphinx and MdBook.

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

Sphinx renders this with "USR-001 Plain Text Storage" as the page title, which is exactly what you want.

## Handling Frontmatter

### Problem: YAML Frontmatter Renders as Code Block

Sphinx/MyST renders YAML frontmatter as a code block:

```
---
_version: '1'
uuid: 4bfeb7d5-...
---
```

This is usually undesirable in rendered documentation.

### Solution 1: Skip Frontmatter with Line Selection

```markdown
```{include} requirements/USR-001.md
:start-line: 7
\```
```

Only includes the requirement body, skipping frontmatter.

**Determining start line**:
Typical frontmatter structure:
```
Line 1: ---
Line 2: _version: '1'
Line 3: uuid: ...
Line 4: created: ...
Line 5: ---
Line 6: (empty line)
Line 7: Requirement text starts here
```

Use `:start-line: 7` to skip to content.

### Solution 2: Custom MyST Extension

Create a custom directive that parses frontmatter:

**conf.py**:
```python
from docutils import nodes
from docutils.parsers.rst import Directive
import yaml

class RequirementDirective(Directive):
    required_arguments = 1  # Requirement file path

    def run(self):
        # Parse requirement file
        # Extract frontmatter and body
        # Render as desired
        ...

def setup(app):
    app.add_directive("requirement", RequirementDirective)
```

Usage:
```markdown
```{requirement} requirements/USR-001.md
\```
```

This allows custom rendering of requirements with frontmatter metadata.

### Solution 3: Preprocessing Script

Generate requirement markdown without frontmatter:

**generate-docs.sh**:
```bash
#!/bin/bash
mkdir -p _generated

for req in requirements/*.md; do
    # Skip frontmatter (lines 1-6), extract body
    tail -n +7 "$req" > "_generated/$(basename "$req")"
done
```

Then include from `_generated/`:

```markdown
```{include} _generated/USR-001.md
\```
```

## Advanced Integration

### Requirement Traceability Matrix

Generate traceability tables with Python scripts:

**generate-traceability.py**:
```python
#!/usr/bin/env python3
import glob
import yaml
import re

def parse_requirement(path):
    with open(path) as f:
        content = f.read()
    # Extract frontmatter
    match = re.match(r'^---\n(.*?)\n---\n(.*)$', content, re.DOTALL)
    if match:
        frontmatter = yaml.safe_load(match.group(1))
        body = match.group(2)
        return frontmatter, body
    return None, content

def generate_matrix():
    matrix = []
    for req_file in glob.glob("requirements/*.md"):
        frontmatter, body = parse_requirement(req_file)
        if frontmatter:
            hrid = req_file.split('/')[-1].replace('.md', '')
            parents = frontmatter.get('parents', [])
            parent_hrids = [p['hrid'] for p in parents]
            matrix.append((hrid, parent_hrids))

    # Generate Markdown table
    print("| Child | Parents |")
    print("|-------|---------|")
    for child, parents in matrix:
        print(f"| {child} | {', '.join(parents) if parents else '-'} |")

if __name__ == '__main__':
    generate_matrix()
```

Run before Sphinx build:

```bash
python generate-traceability.py > traceability.md
```

Include in documentation:

**index.md**:
```markdown
```{include} traceability.md
\```
```

### Graphviz Diagrams

Generate requirement hierarchy diagrams:

**generate-graph.py**:
```python
import glob
import yaml
import re

def generate_dot():
    print("digraph requirements {")
    for req_file in glob.glob("requirements/*.md"):
        with open(req_file) as f:
            content = f.read()
        match = re.match(r'^---\n(.*?)\n---', content, re.DOTALL)
        if match:
            frontmatter = yaml.safe_load(match.group(1))
            hrid = req_file.split('/')[-1].replace('.md', '')
            parents = frontmatter.get('parents', [])
            for parent in parents:
                print(f'    "{parent["hrid"]}" -> "{hrid}";')
    print("}")

if __name__ == '__main__':
    generate_dot()
```

Generate diagram:

```bash
python generate-graph.py | dot -Tpng > hierarchy.png
```

Include in Sphinx:

```markdown
![Requirement Hierarchy](hierarchy.png)
```

Or use Sphinx's `graphviz` directive:

**conf.py**:
```python
extensions = ["myst_parser", "sphinx.ext.graphviz"]
```

**docs/hierarchy.md**:
```markdown
```{graphviz}
:caption: Requirement Hierarchy

digraph requirements {
    "USR-001" -> "SYS-001";
    "USR-002" -> "SYS-001";
    "USR-002" -> "SYS-002";
}
\```
```

## CI/CD Integration

Validate requirements and build docs in CI:

**.github/workflows/docs.yml**:
```yaml
name: Documentation

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          pip install sphinx myst-parser

      - name: Validate requirements
        run: |
          cargo install requirements-manager
          req clean
        working-directory: ./docs/requirements

      - name: Build documentation
        run: make html
        working-directory: ./docs

      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/_build/html
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

**Diagnosis**: Check if MyST Parser is in `extensions` list.

**Solution**: Add to **conf.py**:
```python
extensions = ["myst_parser"]
```

### Frontmatter Renders Incorrectly

**Problem**: YAML frontmatter shows as code block.

**Explanation**: Expected behavior. MyST doesn't parse YAML frontmatter.

**Solution**: Use line selection to skip frontmatter (see [Handling Frontmatter](#handling-frontmatter)).

### Include Directive Not Working

**Problem**: `{include}` directive doesn't embed content.

**Diagnosis**: Check file path is correct relative to source file.

**Solution**: Use correct relative paths:
```markdown
```{include} ../requirements/USR-001.md
\```
```

## Comparison: Sphinx vs. MdBook

| Feature | Sphinx | MdBook |
|---------|--------|--------|
| **Language** | Python ecosystem | Rust ecosystem |
| **Markup** | RST + Markdown (via MyST) | Markdown only |
| **Extensions** | Extensive ecosystem | Growing ecosystem |
| **API Docs** | Excellent (autodoc) | Limited |
| **Themes** | Many available | Fewer options |
| **Speed** | Slower (Python) | Faster (Rust) |
| **Requirement Integration** | Good with MyST | Native (both Markdown) |

**Choose Sphinx if**:
- Python project
- Need autodoc/API documentation
- Want extensive extension ecosystem

**Choose MdBook if**:
- Rust project
- Simpler setup
- Pure Markdown workflow

## Summary

**Key Points**:

- Use MyST Parser extension for Markdown support
- Requirements integrate naturally with MyST directives
- Skip frontmatter with line selection (`:start-line:`)
- Generate traceability matrices and diagrams with scripts
- Validate requirements in CI before building docs

**Best Practices**:
- Dedicated requirements directory
- Use MyST directives for rich formatting
- Create requirement indexes
- Cross-reference with Sphinx roles
- Integrate with CI/CD

**Limitations**:
- Frontmatter renders as code block (use line selection)
- Requires MyST Parser (additional dependency)
- Less "native" than MdBook (Sphinx primarily RST)

## Next Steps

- Review the [Sphinx example](https://github.com/danieleades/requirements-manager/tree/main/examples/sphinx) in the repository
- See [Version Control Best Practices](./version-control.md) for managing requirements
- Compare with [Using with MdBook](./mdbook.md) for Rust projects
