# Advanced Topics

This chapter covers advanced features and techniques for requirements management with Requiem.

## Overview

Advanced topics include:

- **[Coverage Reports](./advanced/coverage.md)** - Traceability coverage analysis (planned)
- **[Cycle Detection](./advanced/cycles.md)** - Finding circular dependencies (planned)
- **[Import and Export](./advanced/import-export.md)** - Interoperability with other tools (planned)

## Current Capabilities

Requiem currently provides core functionality:

- Plain-text requirements in Markdown
- Human-readable IDs with namespace support
- Parent-child relationships with multiple parents
- Content fingerprinting for change detection
- Fast parallel loading
- Integration with MdBook and Sphinx

## Planned Capabilities

Future releases will add:

- **Coverage reports**: Analyze requirement traceability
- **Cycle detection**: Find and report circular dependencies
- **Review workflows**: Automated review triggering
- **Import/export**: ReqIF, JSON, CSV formats
- **Validation rules**: Custom requirement quality checks
- **Query language**: Advanced filtering and searching

## Contributing

These features are planned but not yet implemented. Interested in contributing? See the [GitHub repository](https://github.com/danieleades/requirements-manager) for:

- Feature roadmap
- Open issues
- Contribution guidelines
- Development setup

## Workarounds

Until advanced features are implemented, consider these approaches:

### Manual Coverage Analysis

Use scripts to analyze traceability:

```bash
#!/bin/bash
# Find USR requirements without SYS children

comm -23 \
  <(ls USR-*.md | sed 's/.md//' | sort) \
  <(grep -oh "USR-[0-9]*" SYS-*.md | sort -u)
```

### Manual Cycle Detection

Trace requirement chains manually:

```bash
# Follow parent chain
grep "uuid:" USR-001.md  # Get UUID
grep "<uuid>" *.md        # Find children
# Repeat for each child
```

### Export with Scripts

Generate reports in various formats:

```python
import glob
import yaml
import re
import json

requirements = []
for path in glob.glob("*.md"):
    with open(path) as f:
        content = f.read()
    # Parse frontmatter and body
    # Add to requirements list

# Export as JSON
with open("requirements.json", "w") as f:
    json.dump(requirements, f, indent=2)
```

## Next Steps

Explore planned features:

- **[Coverage Reports](./advanced/coverage.md)** - What coverage analysis will provide
- **[Cycle Detection](./advanced/cycles.md)** - How cycle detection will work
- **[Import and Export](./advanced/import-export.md)** - Planned interoperability formats
