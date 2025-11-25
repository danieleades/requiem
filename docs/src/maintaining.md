# Maintaining Requirements

Requirements evolve throughout a project's lifecycle. This chapter covers maintenance tasks that keep your requirements accurate and consistent.

## Overview

Requiem provides tools for:

- **[Correcting HRIDs](./maintaining/correcting-hrids.md)** - Fixing parent references after renaming requirements
- **[Fingerprints and Change Detection](./maintaining/fingerprints.md)** - Understanding how Requiem tracks content changes
- **[Review Workflows](./maintaining/review-workflows.md)** - Managing reviews when requirements change (planned feature)

## Common Maintenance Tasks

### Regular Synchronization

Run `req sync` periodically to ensure consistency:

```bash
req sync               # update stored parent HRIDs
req sync --what paths  # move files to canonical locations
```

This command:
- Loads all requirements
- Corrects stored parent HRIDs if requirements were renamed
- Optionally fixes file locations to match the configured path mode
- Exits non-zero when drift exists (use `--check` for CI-friendly checks)

**Frequency**: Run after major reorganizations, renames, or before releases.

### After Renaming Requirements

If you rename requirement files (change HRIDs):

1. Rename the file:
```bash
mv USR-001.md USR-100.md
```

2. Update parent references:
```bash
req sync
```

3. Verify changes:
```bash
git diff
```

The `sync` command updates all parent HRIDs automatically.

### After Editing Content

When requirement text changes:

1. Edit the markdown body
2. Save the file
3. Commit with a descriptive message:
```bash
git add USR-001.md
git commit -m "Update USR-001: clarify email validation requirement"
```

The fingerprint updates automatically when content changes, enabling change detection.

### Managing Requirements at Scale

For projects with hundreds of requirements:

**Use scripts**: Automate repetitive tasks
```bash
# Example: Add a tag to all USR requirements
for file in USR-*.md; do
    # Add tag via sed or Python script
done
```

**Use version control**: Track changes over time
```bash
# See what requirements changed this sprint
git log --since="2 weeks ago" --name-only -- "*.md"
```

**Use traceability**: Understand impact of changes
```bash
# Find all children of a requirement
grep -r "uuid: <parent-uuid>" *.md
```

## Maintenance Best Practices

### 1. Validate Before Committing

Always check for drift before committing changes:

```bash
req review && req sync --check && git add -A && git commit -m "Update requirements"
```

This catches errors before they enter the repository.

### 2. Use Descriptive Commit Messages

Bad:
```bash
git commit -m "Update reqs"
```

Good:
```bash
git commit -m "Add USR-042: user data export requirement

Related to feature request #123. Establishes requirements
for CSV and JSON export formats."
```

### 3. Review Changes Carefully

Before committing requirement changes, review diffs:

```bash
git diff USR-001.md
```

Ensure:
- UUID hasn't changed (breaking traceability)
- Metadata is valid
- Content changes are intentional

### 4. Keep Requirements Current

**Regular reviews**: Schedule periodic requirement reviews

**Update as needed**: Don't let requirements become stale documentation

**Archive obsolete requirements**: Move to `archived/` subdirectory rather than deleting

### 5. Document Maintenance Procedures

Create a `CONTRIBUTING.md` or similar:

```markdown
# Requirements Maintenance

## Before Committing
1. Run `req review` and `req sync --check`
2. Review diffs carefully
3. Don't modify UUIDs manually

## Renaming Requirements
1. Rename file
2. Run `req sync` to update parent references
3. Commit changes

## Adding Tags
Use YAML syntax in frontmatter...
```

## Tools and Automation

### Pre-commit Hooks

Automate validation with Git hooks:

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Validating requirements..."
req review && req sync --check

if [ $? -ne 0 ]; then
    echo "Error: Requirements validation failed"
    exit 1
fi
```

### CI/CD Integration

Add requirement validation to CI:

```yaml
# .github/workflows/requirements.yml
name: Validate Requirements

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Requiem
        run: cargo install requirements-manager
      - name: Validate requirements
        run: req review && req sync --check
        working-directory: ./requirements
```

### Scripts for Common Tasks

**Find requirements without tests**:
```bash
#!/bin/bash
# List USR requirements not linked by any TST requirement
comm -23 \
  <(ls USR-*.md | sed 's/.md//' | sort) \
  <(grep -oh "USR-[0-9]*" TST-*.md | sort -u)
```

**Generate traceability report**:
```bash
#!/bin/bash
# Show parent-child relationships
for file in *.md; do
    hrid=$(basename "$file" .md)
    parents=$(grep "hrid:" "$file" | awk '{print $2}')
    if [ -n "$parents" ]; then
        echo "$hrid -> $parents"
    fi
done
```

## Error Recovery

### Corrupted Frontmatter

If YAML becomes invalid:

1. Check syntax:
```bash
req status  # Fails fast with the parse error
```

2. Fix manually or restore from Git:
```bash
git checkout HEAD -- USR-001.md
```

3. Use online YAML validator if needed

### Duplicate UUIDs

If two requirements have the same UUID (serious error):

1. Requiem will panic with error message
2. Identify files with duplicate UUIDs
3. Manually assign new UUID to one:
```bash
uuidgen  # Generate new UUID
# Edit file, replace UUID
```

4. Verify:
```bash
req status
```

### Missing Parents

If a parent requirement is deleted but children still reference it:

1. `req review` will surface the missing parent as a suspect link (empty current fingerprint)
2. Options:
   - Restore the deleted parent
   - Remove the parent reference from the child
   - Link the child to a different parent

### Merge Conflicts

When merging Git branches with requirement changes:

1. Resolve conflicts in frontmatter carefully:
   - UUIDs should never conflict (unique per requirement)
   - Timestamps: keep newer
   - Parents: merge both sets if both added parents

2. Resolve markdown body conflicts normally

3. After resolving:
```bash
req review && req sync --check  # Validate merged result
```

## Monitoring and Reporting

### Requirement Statistics

Count requirements by kind:
```bash
ls USR-*.md | wc -l  # User requirements
ls SYS-*.md | wc -l  # System requirements
```

### Change Tracking

Requirements changed in last month:
```bash
git log --since="1 month ago" --name-only --pretty=format: -- "*.md" | \
  sort -u | grep -E "^[A-Z]+-[0-9]+\.md$"
```

### Coverage Analysis

Find requirements without children (potential gaps):
```bash
# Find USR requirements not referenced by any SYS requirement
comm -23 \
  <(ls USR-*.md | sed 's/.md//' | sort) \
  <(grep -roh "USR-[0-9]*" SYS-*.md | sort -u)
```

## Next Steps

Dive deeper into specific maintenance topics:

- **[Correcting HRIDs](./maintaining/correcting-hrids.md)** - Using the `req sync` command
- **[Fingerprints](./maintaining/fingerprints.md)** - How change detection works
- **[Review Workflows](./maintaining/review-workflows.md)** - Managing reviews (planned)
