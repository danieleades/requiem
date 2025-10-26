# Correcting HRIDs

The `req clean` command corrects outdated parent HRIDs in requirement files. This chapter explains when and how to use it.

## The Problem

Requirements reference their parents using HRIDs for human readability:

```yaml
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001  # Human-readable reference
```

If you rename the parent file (`USR-001.md` → `USR-100.md`), the HRID in child requirements becomes outdated:

```yaml
# Child still says:
parents:
- hrid: USR-001  # Wrong! File is now USR-100.md
```

The UUID remains correct (traceability is preserved), but the HRID shown to humans is misleading.

## The Solution: `req clean`

The `req clean` command:

1. Loads all requirements
2. For each requirement, checks parent HRIDs against actual parent files
3. Corrects any mismatches
4. Saves updated requirements

### Basic Usage

```bash
req clean
```

Run in your requirements directory. No output means success (all HRIDs were correct or have been fixed).

### With Custom Root

```bash
req --root /path/to/requirements clean
```

Specify a different requirements directory.

### Verbose Output

```bash
req -v clean
```

Shows what's being corrected:

```
INFO Corrected parent HRID in SYS-001: USR-001 → USR-100
INFO Corrected parent HRID in SYS-002: USR-001 → USR-100
```

## When to Run `req clean`

### After Renaming Requirements

**Scenario**: You renamed a requirement file.

**Steps**:
1. Rename the file:
```bash
mv USR-001.md USR-100.md
```

2. Correct parent references:
```bash
req clean
```

3. Verify changes:
```bash
git diff
```

You'll see parent HRIDs updated in child requirements.

### After Reorganization

**Scenario**: Major restructuring with many renamed files.

**Steps**:
1. Perform renames:
```bash
mv USR-001.md AUTH-USR-001.md
mv USR-002.md PAYMENT-USR-002.md
# ... many more
```

2. Fix all references at once:
```bash
req clean
```

3. Verify:
```bash
git status    # See all modified files
git diff      # Review changes
```

### Before Committing

**Best practice**: Run `req clean` before every commit involving requirement changes.

```bash
req clean && git add -A && git commit -m "Reorganize requirements"
```

This ensures the repository always has correct HRIDs.

### Regular Maintenance

**Frequency**: Run periodically (e.g., before releases) to catch any drift.

```bash
req clean
```

If requirements are managed carefully, this should show no changes.

## How It Works

### Step 1: Load All Requirements

Requiem scans the requirements directory recursively, loading all `.md` files with valid HRID names.

### Step 2: Build Index

Creates a UUID-to-requirement mapping:

```
UUID 4bfeb7d5-... → USR-100 (actual current HRID)
UUID 3fc6800c-... → SYS-001
...
```

### Step 3: Check Each Parent Reference

For each requirement, examines parent references:

```yaml
# Child requirement SYS-001.md
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  hrid: USR-001  # Check if this matches actual HRID for this UUID
```

### Step 4: Correct Mismatches

If the HRID doesn't match:
- Look up the UUID in the index
- Find the actual current HRID
- Update the parent reference
- Save the requirement file

### Step 5: Report Results

With verbose logging (`-v`), reports each correction. Otherwise, silent success.

## Examples

### Example 1: Simple Rename

**Before**:
```
requirements/
├── USR-001.md
└── SYS-001.md
```

`SYS-001.md` links to `USR-001`:
```yaml
# SYS-001.md
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  hrid: USR-001
```

**Rename parent**:
```bash
mv USR-001.md USR-100.md
```

**Run clean**:
```bash
req clean
```

**After**:

`SYS-001.md` now shows correct HRID:
```yaml
# SYS-001.md
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  hrid: USR-100  # Corrected!
```

### Example 2: Multiple Children

**Scenario**: One parent with multiple children.

**Before**:
```
requirements/
├── USR-001.md
├── SYS-001.md  (parent: USR-001)
├── SYS-002.md  (parent: USR-001)
└── SYS-003.md  (parent: USR-001)
```

**Rename parent**:
```bash
mv USR-001.md USR-050.md
```

**Run clean**:
```bash
req -v clean
```

**Output**:
```
INFO Corrected parent HRID in SYS-001: USR-001 → USR-050
INFO Corrected parent HRID in SYS-002: USR-001 → USR-050
INFO Corrected parent HRID in SYS-003: USR-001 → USR-050
```

All three children are updated in one command.

### Example 3: Adding Namespaces

**Scenario**: Migrating to namespaced HRIDs.

**Before**:
```
requirements/
├── USR-001.md
├── USR-002.md
├── SYS-001.md  (parent: USR-001)
└── SYS-002.md  (parent: USR-002)
```

**Rename with namespaces**:
```bash
mv USR-001.md AUTH-USR-001.md
mv USR-002.md PAYMENT-USR-002.md
mv SYS-001.md AUTH-SYS-001.md
mv SYS-002.md PAYMENT-SYS-002.md
```

**Run clean**:
```bash
req clean
```

**Result**: All parent references updated to namespaced HRIDs.

## Edge Cases

### Requirement Not Found

**Scenario**: Child references a parent UUID that doesn't exist.

**Example**:
```yaml
# SYS-001.md
parents:
- uuid: 00000000-0000-0000-0000-000000000000  # No requirement with this UUID
  hrid: USR-999
```

**Behavior**: `req clean` panics with error message:

```
Error: Parent requirement 00000000-0000-0000-0000-000000000000 not found!
```

**Resolution**:
- Restore the missing parent requirement, or
- Manually remove the invalid parent reference from the child

### Self-Referential Parent

**Scenario**: Requirement lists itself as a parent (should never happen).

**Example**:
```yaml
# SYS-001.md
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a  # Same as own UUID!
  hrid: SYS-001
```

**Behavior**: `req clean` panics with error message:

```
Error: Requirement 4bfeb7d5-... is its own parent!
```

**Resolution**: Manually remove the self-reference from the frontmatter.

### Circular Dependencies

**Scenario**: Requirement A depends on B, B depends on C, C depends on A.

**Current behavior**: `req clean` doesn't detect cycles (cycle detection is planned but not implemented).

**Impact**: HRIDs will be corrected, but the circular dependency remains undetected.

**Workaround**: Manually audit requirement relationships or use external tools.

## Integration with Workflows

### Pre-commit Hook

Automatically correct HRIDs before every commit:

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Correcting requirement HRIDs..."
req clean

if [ $? -ne 0 ]; then
    echo "Error: Failed to correct HRIDs"
    exit 1
fi

# Stage any changes made by req clean
git add -u
```

**Benefit**: Never commit incorrect HRIDs.

**Caution**: Automatically stages changes. Review carefully.

### CI Pipeline

Validate HRIDs in CI:

```yaml
# .github/workflows/requirements.yml
- name: Validate HRIDs
  run: |
    req clean
    if [ -n "$(git status --porcelain)" ]; then
      echo "Error: HRIDs are out of sync"
      git diff
      exit 1
    fi
```

**Benefit**: Catches incorrect HRIDs before merging.

### Manual Review Workflow

For critical projects, manually review HRID corrections:

```bash
# Run clean
req clean

# Review changes
git diff

# If acceptable, commit
git add -A
git commit -m "Correct parent HRIDs after reorganization"
```

## Performance

`req clean` loads all requirements in parallel, making it fast even for large projects:

- **100 requirements**: < 1 second
- **1000 requirements**: ~2-3 seconds
- **10000 requirements**: ~15-20 seconds

**Scales well** due to Rust's performance and parallel processing.

## Limitations

### No Dry-Run Mode

Currently, `req clean` modifies files immediately. There's no preview mode.

**Workaround**: Use Git to preview changes:

```bash
req clean         # Make changes
git diff          # Preview
git checkout -- . # Undo if needed (before committing)
```

### No Selective Correction

Can't correct only specific requirements; it's all-or-nothing.

**Workaround**: Use Git to selectively stage changes:

```bash
req clean
git add SYS-001.md SYS-002.md  # Stage only specific files
```

### Requires All Parents Present

If a parent requirement is missing, `req clean` fails. Can't correct partial sets.

**Workaround**: Ensure all requirements are present, or manually fix references.

## Best Practices

### 1. Run Before Committing

```bash
req clean && git add -A && git commit
```

Make it a habit.

### 2. Review Changes

Always review what `req clean` changed:

```bash
req clean
git diff  # See what was corrected
```

### 3. Use Verbose Mode for Learning

When first using `req clean`, run with `-v` to understand what it's doing:

```bash
req -v clean
```

### 4. Combine with Validation

Use `req clean` as a validation step:

```bash
req clean
if [ $? -eq 0 ]; then
    echo "Requirements are consistent"
else
    echo "Errors found"
fi
```

### 5. Document in Team Processes

Include in your team's documentation:

```markdown
## Renaming Requirements

1. Rename the file
2. Run `req clean`
3. Review changes with `git diff`
4. Commit
```

## Troubleshooting

### Command Not Found

**Error**: `req: command not found`

**Solution**: Install Requiem:
```bash
cargo install requirements-manager
```

### Permission Denied

**Error**: `Error: Permission denied`

**Solution**: Ensure you have write permissions to requirement files.

### Configuration Parse Error

**Error**: `Error: Failed to parse config file`

**Solution**: Check `config.toml` syntax. Remove or fix if invalid.

### Unexpected Changes

**Issue**: `req clean` makes unexpected modifications.

**Diagnosis**:
1. Run with verbose: `req -v clean`
2. Examine which HRIDs are being corrected
3. Check if requirement files were renamed

**Resolution**: Review changes with `git diff`. Revert if incorrect.

## Summary

**Key points**:

- **Purpose**: Correct outdated parent HRIDs after renaming requirements
- **Usage**: `req clean` in requirements directory
- **When**: After renaming files, before committing, regular maintenance
- **How**: Loads all requirements, checks parent HRIDs, corrects mismatches
- **Safe**: Uses UUIDs for correctness; HRIDs are display-only

**Best practice**: Run `req clean` before every commit involving requirements.

**Limitation**: No dry-run or selective correction (yet).

## Next Steps

- Learn about [Fingerprints](./fingerprints.md) for change detection
- Understand [Review Workflows](./review-workflows.md) (planned feature)
