# Integration

Requiem integrates with existing tools in your development workflow. This chapter covers integration strategies and best practices.

## Overview

Requiem's plain-text format enables integration with:

- **[MdBook](./integration/mdbook.md)** - Embed requirements in MdBook documentation
- **[Sphinx](./integration/sphinx.md)** - Include requirements in Sphinx-generated docs
- **[Version Control](./integration/version-control.md)** - Git workflows and best practices

## Philosophy: Compose, Don't Replace

Requiem doesn't aim to replace your existing documentation or development tools. Instead, it composes with them:

### Documentation Tools

Requirements live alongside your documentation:

```
docs/
├── user-guide.md
├── architecture.md
├── requirements/
│   ├── USR-001.md    ← Requirements
│   └── SYS-001.md
└── api-reference.md
```

Documentation can reference or embed requirements directly.

### Version Control

Requirements are plain text files that work naturally with Git:

- Meaningful diffs
- Branch and merge workflows
- Pull request reviews
- Complete history

### Static Site Generators

MdBook and Sphinx can include requirement files in generated documentation:

```markdown
# User Guide

## Requirements

{{#include ./requirements/USR-001.md}}
```

Requirements stay synchronized with documentation automatically.

### CI/CD Pipelines

Validate requirements in continuous integration:

```yaml
- name: Validate requirements
  run: req clean
```

Catch errors before merging.

### Custom Tools

Plain text enables custom tooling:

```python
# Custom analysis script
import glob

for req_file in glob.glob("USR-*.md"):
    # Parse, analyze, generate reports, etc.
```

## Integration Patterns

### Pattern 1: Standalone Requirements

Requirements in dedicated directory:

```
project/
├── src/           ← Source code
├── requirements/  ← Requirements (separate)
│   ├── config.toml
│   └── USR-001.md
└── docs/          ← Documentation (separate)
```

**Best for**: Clear separation of concerns, formal requirements management.

### Pattern 2: Requirements with Documentation

Requirements embedded in documentation:

```
docs/
├── config.toml
├── user-guide.md
├── USR-001.md      ← Mixed with docs
├── architecture.md
└── SYS-001.md      ← Mixed with docs
```

Configuration: `allow_unrecognised = true` (to ignore non-requirement markdown files).

**Best for**: Integrated documentation, technical specifications.

### Pattern 3: Monorepo with Multiple Projects

Each project has its own requirements:

```
monorepo/
├── project-a/
│   └── requirements/
│       └── USR-001.md
├── project-b/
│   └── requirements/
│       └── USR-001.md
└── shared/
    └── requirements/
        └── CORE-USR-001.md
```

**Best for**: Multi-project organizations, shared requirements.

## Integration Benefits

### Single Source of Truth

Requirements are defined once, referenced everywhere:

- User documentation includes requirement text
- Design docs reference requirements
- Test plans trace to requirements
- Code comments link to requirements

Changes propagate automatically (when using includes/references).

### Automated Consistency

CI/CD ensures requirements remain valid:

```yaml
# .github/workflows/requirements.yml
- run: req clean
```

Prevents broken traceability or invalid formatting.

### Developer-Friendly

Plain text means developers can:
- Use their preferred editor
- Search with grep/ripgrep
- Script with Python/Bash
- Review in pull requests

No context switching to specialized tools.

### Audit Trail

Git provides complete history:

```bash
# See all changes to a requirement
git log -p USR-001.md

# See who last modified
git blame USR-001.md

# See what changed in a sprint
git log --since="2 weeks ago" -- requirements/
```

## Next Steps

Explore specific integrations:

- **[Using with MdBook](./integration/mdbook.md)** - Embed requirements in MdBook sites
- **[Using with Sphinx](./integration/sphinx.md)** - Include requirements in Sphinx documentation
- **[Version Control Best Practices](./integration/version-control.md)** - Git workflows for requirements
