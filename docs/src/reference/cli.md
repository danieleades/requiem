# CLI Command Reference

Complete reference for the `req` command-line interface.

## Overview

The `req` command is the main interface to Requiem. It provides commands for creating, linking, and managing requirements.

### Global Synopsis

```
req [OPTIONS] <COMMAND>
```

### Getting Help

```bash
# General help
req --help

# Command-specific help
req add --help
req link --help
req clean --help
```

### Version Information

```bash
req --version
```

## Global Options

Options that apply to all commands.

### `-v, --verbose`

Increase logging verbosity. Can be specified multiple times.

**Levels**:
- No `-v`: WARN level (errors and warnings only)
- `-v`: INFO level (general information)
- `-vv`: DEBUG level (detailed debugging information)
- `-vvv`: TRACE level (very detailed trace information)

**Examples**:
```bash
req -v clean              # INFO level
req -vv add USR           # DEBUG level
req -vvv link SYS-001 USR-001  # TRACE level
```

**Use cases**:
- **Normal use**: No `-v` flag
- **Troubleshooting**: `-v` or `-vv`
- **Development/debugging**: `-vvv`

### `-r, --root <PATH>`

Specify the root directory containing requirements.

**Default**: Current directory (`.`)

**Examples**:
```bash
req --root /path/to/requirements add USR
req -r ./reqs clean
req --root ~/project/requirements link SYS-001 USR-001
```

**Notes**:
- Path can be absolute or relative
- Must be a directory
- Directory should contain `config.toml` (optional) and `.md` requirement files

## Commands

### `req add`

Create a new requirement.

#### Synopsis

```
req add [OPTIONS] <KIND>
```

#### Arguments

**`<KIND>`** (required)

The kind of requirement to create. This becomes the KIND component of the HRID.

Examples: `USR`, `SYS`, `SWR`, `TST`, `AUTH-USR` (with namespace)

#### Options

**`-p, --parent <PARENT>...`**

Specify parent requirement(s) by HRID. Can be specified multiple times or as comma-separated list.

**Examples**:
```bash
# Single parent
req add SYS --parent USR-001

# Multiple parents (multiple flags)
req add SYS --parent USR-001 --parent USR-002

# Multiple parents (comma-separated)
req add SYS --parents USR-001,USR-002

# Short form
req add SYS -p USR-001,USR-002
```

**`-t, --title <TITLE>`**

Set the title of the requirement. The title will be formatted as a markdown heading (`# Title`).

If both `-t` and `-b` are provided, the title appears first, followed by a blank line, then the body.

**Examples**:
```bash
# Create with title only
req add USR -t "User Authentication"

# Create with title and body
req add USR -t "User Authentication" -b "The system shall authenticate users."
```

**`-b, --body <BODY>`**

Set the body content of the requirement. Can be any markdown text.

**Examples**:
```bash
# Create with body only
req add USR -b "The system shall validate user input."

# Create with multi-line body (using quotes)
req add USR -b "The system shall:
- Validate input
- Sanitize data
- Log attempts"
```

#### Behavior

1. Determines next available ID for the given KIND
2. Determines content:
   - If `-t` or `-b` flags are provided, uses that content
   - Otherwise, checks for a template file at `.req/templates/<KIND>.md`
   - If no template file exists, uses empty content
3. Creates requirement file `<KIND>-<ID>.md` with:
   - Automatically generated UUID
   - Current timestamp
   - Content (from flags, template, or empty)
   - Parent links (if specified)
4. Prints the HRID of created requirement

**Template Priority**:
- CLI flags (`-t`, `-b`) always override templates
- Templates are matched by KIND (e.g., `USR`, `AUTH-USR`)
- Template files are stored in `.req/templates/` directory
- See [Templates Guide](../working-with-requirements/templates.md) for template setup

#### Examples

**Create user requirement**:
```bash
req add USR
# Output: Added requirement USR-001
```

**Create system requirement with parent**:
```bash
req add SYS --parent USR-001
# Output: Added requirement SYS-001
```

**Create requirement with multiple parents**:
```bash
req add SWR --parents SYS-001,SYS-002
# Output: Added requirement SWR-001
```

**Create namespaced requirement**:
```bash
req add AUTH-USR
# Output: Added requirement AUTH-USR-001
```

**Create with title and body**:
```bash
req add USR -t "User Authentication" -b "The system shall authenticate users."
# Output: Added requirement USR-001
# Content: # User Authentication\n\nThe system shall authenticate users.
```

**Create with template** (assuming `.req/templates/USR.md` exists):
```bash
req add USR
# Output: Added requirement USR-001
# Content populated from .req/templates/USR.md
```

**Override template with CLI flags**:
```bash
req add USR -t "Custom Title"
# Output: Added requirement USR-001
# Content from -t flag, template ignored
```

#### Error Cases

**Empty KIND**:
```bash
req add ""
# Error: Kind cannot be empty
```

**Invalid KIND** (if `allowed_kinds` configured):
```bash
req add INVALID
# Error: Kind 'INVALID' is not in the allowed list
```

**Parent not found**:
```bash
req add SYS --parent USR-999
# Error: Parent requirement USR-999 not found
```

### `req link`

Create a parent-child link between two existing requirements.

#### Synopsis

```
req link <CHILD> <PARENT>
```

#### Arguments

**`<CHILD>`** (required)

HRID of the child requirement (the requirement gaining a parent).

**`<PARENT>`** (required)

HRID of the parent requirement (the upstream requirement).

#### Behavior

1. Loads both child and parent requirements
2. Adds parent reference to child's frontmatter:
   - Parent's UUID
   - Parent's current HRID
   - Parent's current fingerprint
3. Saves updated child requirement
4. Prints confirmation message

#### Examples

**Link system to user requirement**:
```bash
req link SYS-001 USR-001
# Output: Linked SYS-001 to USR-001
```

**Link with namespaces**:
```bash
req link AUTH-SYS-001 AUTH-USR-001
# Output: Linked AUTH-SYS-001 to AUTH-USR-001
```

**Create second parent** (multiple parents supported):
```bash
req link SYS-001 USR-001  # First parent
req link SYS-001 USR-002  # Second parent
# SYS-001 now has two parents
```

#### Error Cases

**Child not found**:
```bash
req link SYS-999 USR-001
# Error: Child requirement SYS-999 not found
```

**Parent not found**:
```bash
req link SYS-001 USR-999
# Error: Parent requirement USR-999 not found
```

**Invalid HRID format**:
```bash
req link INVALID USR-001
# Error: Invalid HRID format: INVALID
```

### `req suspect`

List all suspect links in the requirement graph.

#### Synopsis

```
req suspect
```

#### Arguments

None.

#### Options

None.

#### Behavior

1. Loads all requirements from the requirements directory
2. Compares stored parent fingerprints with current parent fingerprints
3. Lists all requirements with mismatched fingerprints (suspect links)
4. For each suspect link, displays:
   - Child HRID → Parent HRID
   - Stored fingerprint (first 16 characters)
   - Current fingerprint (first 16 characters)
5. Exits with code 1 if suspect links found (useful for CI/CD)
6. Exits with code 0 if no suspect links found

#### Examples

**Check for suspect links**:
```bash
req suspect
```

Example output when suspect links exist:
```
Found 3 suspect link(s):

  SYS-001 → USR-001
    Stored fingerprint:  e533784ff58c16cb
    Current fingerprint: c4020419ead000e9

  SYS-002 → USR-001
    Stored fingerprint:  e533784ff58c16cb
    Current fingerprint: c4020419ead000e9

  SYS-005 → USR-004
    Stored fingerprint:  407c6e3413d5b3fa
    Current fingerprint: c28afe188a974322
```

Example output when no suspect links:
```
No suspect links found.
```

**In CI/CD pipeline**:
```bash
req suspect && echo "All links current" || echo "Review needed"
```

#### Use Cases

- **Pre-commit validation**: Check if parent requirements changed without updating children
- **CI/CD integration**: Fail builds when requirements need review
- **Regular audits**: Identify requirements needing review after upstream changes
- **Change impact**: See which requirements are affected by parent changes

#### Exit Codes

- **0**: No suspect links found (all fingerprints current)
- **1**: Suspect links found (some requirements need review)

This exit code behavior makes the command useful in automation:

```bash
#!/bin/bash
req suspect
if [ $? -ne 0 ]; then
    echo "ERROR: Requirements need review before release"
    exit 1
fi
```

### `req accept`

Accept suspect links after review, updating fingerprints to current values.

#### Synopsis

```
req accept <CHILD> <PARENT>
req accept --all [--dry-run] [--force]
```

#### Arguments

**`<CHILD>`** (required unless `--all`)

HRID of the child requirement containing the suspect link.

**`<PARENT>`** (required unless `--all`)

HRID of the parent requirement referenced by the suspect link.

#### Options

**`--all`**

Accept all suspect links in the requirements directory. Updates all fingerprints to match current parent values.

**`--dry-run`** (requires `--all`)

Preview changes without applying them. Shows which links would be accepted but doesn't modify any files.

**`--force`** (requires `--all`)

Skip confirmation prompt and accept all suspect links immediately. Useful for automation and CI/CD workflows.

#### Behavior

**Single link mode** (`req accept <CHILD> <PARENT>`):
1. Loads the child and parent requirements
2. Verifies the link exists
3. Updates the stored fingerprint to match the parent's current fingerprint
4. Saves the updated child requirement
5. Prints confirmation or reports if link was already up to date

**Bulk mode** (`req accept --all`):
1. Finds all suspect links
2. Shows what will be updated
3. Requests confirmation (unless `--force`)
4. Updates all fingerprints to current values (unless `--dry-run`)
5. Saves all affected requirements
6. Prints summary of updated links

#### Examples

**Accept a single suspect link**:
```bash
req accept SYS-001 USR-001
# Output: Accepted suspect link: SYS-001 → USR-001
```

**Accept all suspect links**:
```bash
req accept --all
# Output:
# Found 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
#
# Accept all 3 suspect link(s)? [y/N] y
# Accepted 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
```

**Dry-run mode (preview only)**:
```bash
req accept --all --dry-run
# Output:
# Found 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
#
# Dry-run mode: no changes made.
```

**Force mode (skip confirmation)**:
```bash
req accept --all --force
# Output:
# Found 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
#
# Accepted 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
```

**Accept after no review needed**:
```bash
req accept SYS-001 USR-001
# Output: Link SYS-001 → USR-001 is already up to date (not suspect).
```

#### Workflow

Typical workflow for handling suspect links:

```bash
# 1. Check for suspect links
req suspect

# 2. Review parent changes
vim USR-001.md  # Review what changed

# 3. Review child requirement
vim SYS-001.md  # Update if needed

# 4. Accept the link (marks as reviewed)
req accept SYS-001 USR-001

# 5. Verify no more suspect links
req suspect
```

#### Use Cases

- **After review**: Mark requirements as reviewed after verifying consistency with parent changes
- **Bulk acceptance**: Update all fingerprints after reviewing multiple changes
- **Post-merge**: Accept fingerprints after merging upstream requirement changes
- **Release preparation**: Clear all suspect links before release

#### Error Cases

**Link not found**:
```bash
req accept SYS-001 USR-999
# Error: Parent requirement USR-999 not found
```

**Child not found**:
```bash
req accept SYS-999 USR-001
# Error: Child requirement SYS-999 not found
```

**No link exists**:
```bash
req accept SYS-001 USR-001
# Error: link from SYS-001 to USR-001 not found
```

**No suspect links with --all**:
```bash
req accept --all
# Output: No suspect links to accept.
```

**User aborts confirmation**:
```bash
req accept --all
# Found 3 suspect link(s):
#   ...
# Accept all 3 suspect link(s)? [y/N] n
# Aborted.
```

#### Exit Codes

- **0**: Success - links accepted or already up to date
- **Non-zero**: Error occurred (child/parent not found, link doesn't exist, I/O error)

Note: Unlike `req suspect`, `req accept` does not use special exit codes to indicate the presence of suspect links.

### `req clean`

Correct parent HRIDs in all requirements.

#### Synopsis

```
req clean
```

#### Arguments

None.

#### Options

None.

#### Behavior

1. Loads all requirements from the requirements directory
2. For each requirement:
   - Checks parent HRIDs against actual parent filenames
   - If HRID is outdated, updates to current HRID
   - Saves requirement if changed
3. Silent on success (no output if no corrections made)
4. With `-v`, logs each correction

#### Examples

**Validate/correct all requirements**:
```bash
req clean
# (no output = success, all HRIDs correct or corrected)
```

**Verbose output**:
```bash
req -v clean
# Output:
# INFO Corrected parent HRID in SYS-001: USR-001 → USR-100
# INFO Corrected parent HRID in SYS-002: USR-001 → USR-100
```

**With custom root**:
```bash
req --root /path/to/requirements clean
```

#### Use Cases

- After renaming requirement files
- After reorganizing requirements
- Before committing changes (validation)
- Regular maintenance

#### Error Cases

**Parent UUID not found**:
```bash
req clean
# Error: Parent requirement <UUID> not found!
```

This indicates a requirement references a parent that doesn't exist. Either restore the parent or manually remove the reference.

**Self-referential parent**:
```bash
req clean
# Error: Requirement <UUID> is its own parent!
```

This indicates a requirement lists itself as a parent. Manually remove the self-reference.

## Common Workflows

### Creating a Requirement Hierarchy

```bash
# Create user requirements
req add USR  # USR-001
req add USR  # USR-002

# Create system requirements linked to user requirements
req add SYS --parent USR-001  # SYS-001
req add SYS --parent USR-002  # SYS-002

# Create software requirement satisfying multiple system requirements
req add SWR --parents SYS-001,SYS-002  # SWR-001

# Create test linked to software requirement
req add TST --parent SWR-001  # TST-001
```

### Renaming Requirements

```bash
# Rename requirement file
mv USR-001.md USR-100.md

# Update parent references
req clean

# Verify
git diff  # See updated parent HRIDs
```

### Linking Existing Requirements

```bash
# Requirements already exist
# USR-001.md
# SYS-001.md

# Create link
req link SYS-001 USR-001

# Add second parent
req link SYS-001 USR-002
```

### Managing Requirement Changes

```bash
# Edit a parent requirement
vim USR-001.md  # Make changes

# Check for suspect links
req suspect
# Output: SYS-001 → USR-001 (fingerprint mismatch)

# Review affected child
vim SYS-001.md  # Review and update if needed

# Accept the change
req accept SYS-001 USR-001

# Verify all links current
req suspect
# Output: No suspect links found.
```

### Bulk Review Workflow

```bash
# After updating multiple parent requirements
req suspect
# Shows all suspect links

# Review and update children as needed
vim SYS-001.md
vim SYS-002.md
# ... review all affected requirements

# Accept all at once
req accept --all

# Commit changes
git add -A
git commit -m "Update requirements after USR changes"
```

## Exit Codes

Requiem uses standard exit codes:

- **0**: Success
- **Non-zero**: Error

Examples:
```bash
req add USR && echo "Success"  # Success
req add INVALID || echo "Failed"  # Failed (if KIND not allowed)
```

Use in scripts:
```bash
#!/bin/bash
req clean
if [ $? -eq 0 ]; then
    echo "Requirements validated"
    git commit -am "Update requirements"
else
    echo "Validation failed"
    exit 1
fi
```

## Environment Variables

### `RUST_LOG`

Control logging level (alternative to `-v` flags).

**Values**:
- `error`: Error messages only
- `warn`: Warnings and errors (default)
- `info`: Informational messages
- `debug`: Debug messages
- `trace`: Verbose trace messages

**Examples**:
```bash
RUST_LOG=info req clean
RUST_LOG=debug req add USR
RUST_LOG=trace req link SYS-001 USR-001
```

**Module-specific logging**:
```bash
RUST_LOG=requiem=debug req clean
RUST_LOG=requiem::storage=trace req add USR
```

## Output Formats

### Standard Output

Success messages go to stdout:
```bash
req add USR
# Output: Added requirement USR-001
```

### Standard Error

Errors and logs go to stderr:
```bash
req add INVALID 2> errors.log
```

### JSON Output (Planned)

Machine-readable output:
```bash
req add USR --format json
# {"success": true, "hrid": "USR-001", "uuid": "..."}
```

## Shell Completion

Generate shell completion scripts:

**Bash**:
```bash
req --generate-completion bash > /etc/bash_completion.d/req
```

**Zsh**:
```bash
req --generate-completion zsh > /usr/local/share/zsh/site-functions/_req
```

**Fish**:
```bash
req --generate-completion fish > ~/.config/fish/completions/req.fish
```

**PowerShell**:
```powershell
req --generate-completion powershell > req.ps1
```

(Note: Completion generation not yet implemented in current version)

## Configuration File

While not a CLI option, the `config.toml` file affects CLI behavior:

```toml
_version = "1"
allowed_kinds = ["USR", "SYS", "TST"]  # Restricts req add
digits = 3                              # Affects HRID formatting
allow_unrecognised = true               # Affects req clean behavior
```

See [Configuration Reference](./configuration.md) for details.

## Performance Considerations

### Parallel Loading

Requiem loads requirements in parallel for performance:

- 100 requirements: < 1 second
- 1000 requirements: ~2-3 seconds
- 10000 requirements: ~15-20 seconds

### Large Directories

For very large requirement sets (1000+):
- Use subdirectories for organization
- `req clean` scales well due to parallelism
- Consider namespaces to partition large sets

## Troubleshooting

### Command Not Found

**Error**: `req: command not found`

**Solution**:
```bash
# Install Requiem
cargo install requirements-manager

# Verify installation
which req
req --version
```

### Permission Denied

**Error**: `Permission denied` when creating/modifying files

**Solution**: Ensure write permissions to requirements directory:
```bash
chmod u+w *.md
chmod u+w .
```

### Invalid Configuration

**Error**: `Failed to parse config file`

**Solution**: Check `config.toml` syntax:
```bash
# Validate TOML
cat config.toml | python -c "import sys, toml; toml.load(sys.stdin)"
```

### Unexpected Behavior

Enable verbose logging:
```bash
req -vv <command>
```

Check logs for detailed error messages.

## Summary

**Core commands**:
- `req add <KIND>` - Create requirement
- `req link <CHILD> <PARENT>` - Link requirements
- `req suspect` - List suspect links (fingerprint mismatches)
- `req accept <CHILD> <PARENT>` - Accept suspect link after review
- `req accept --all` - Accept all suspect links
- `req clean` - Correct parent HRIDs

**Global options**:
- `-v, --verbose` - Increase logging
- `-r, --root <PATH>` - Specify requirements directory

**Exit codes**:
- `0` - Success
- `1` - Suspect links found (req suspect only)
- Non-zero - Error (other commands)

**Getting help**:
- `req --help` - General help
- `req <command> --help` - Command-specific help

## Next Steps

- See [File Format Specification](./file-format.md) for requirement file structure
- See [Configuration Reference](./configuration.md) for `config.toml` options
- Review [Working with Requirements](../working-with-requirements.md) for practical usage
