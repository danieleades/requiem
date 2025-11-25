# Creating Requirements

The `req create` command creates new requirements with automatically generated metadata.

## Basic Usage

```bash
req create <KIND> [--title <TITLE>] [--body <BODY>]
```

KIND supports namespaces via dash-separated tokens; the last token is the kind (e.g., `AUTH-USR`).

### Examples

```bash
# Create USR-001 with a title
req create USR --title "Plain Text Requirements"

# Create SYS-001 with a parent
req create SYS --parent USR-001 --title "Markdown File Format"
```

Output:

```
Added requirement USR-001
Added requirement SYS-001
```

## Command Options

### Parent Requirements

Create a requirement already linked to parents:

```bash
req create <KIND> --parent <PARENT> [--parent <PARENT>...]
```

**Alias**: `-p` for `--parent` (comma-separated values are accepted)

#### Examples

```bash
# Create SYS-001 linked to USR-001
req create SYS --parent USR-001

# Create SYS-002 linked to both USR-001 and USR-002
req create SYS -p USR-001,USR-002
```

### Specify Root Directory

By default, `req` operates in the current directory. Use `--root` to specify a different directory:

```bash
req --root /path/to/requirements create USR --title "Scoped Root"
```

This is useful when running Requiem from outside the requirements directory:

```bash
# Run from project root
req --root ./docs/src/requirements create USR --title "Doc Requirement"
```

### Verbosity

Control log output:

```bash
req -v create USR      # INFO level
req -vv create USR     # DEBUG level
req -vvv create USR    # TRACE level
```

More v's = more verbose. Useful for troubleshooting.

## Generated Content

When you run `req create USR`, a file `USR-001.md` is created with:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---
# USR-001


```

**Generated fields**:
- `_version`: Always `'1'` (current format version)
- `uuid`: A random UUIDv4
- `created`: Current timestamp in ISO 8601 format (UTC)
- `# USR-001`: First heading with HRID (title text is initially empty)

**Empty body**: You need to add the requirement title and text yourself.

## Adding Content

After creating a requirement, edit it to add content:

```bash
req create USR --title "Title Placeholder"
```

# Edit the file
vim USR-001.md
# or
code USR-001.md
```

Add your requirement title and text:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---
# USR-001 CSV Export

Users shall be able to export data in CSV format.

## Rationale

CSV is a universally supported format for data interchange.
```

Note that the HRID (`USR-001`) is the first token in the heading, followed by the title. Save the file. Your requirement is now complete.

## Creating with Parents

When you specify parents, they're included in the frontmatter:

```bash
req create SYS --parent USR-001
```

Creates `SYS-001.md`:

```markdown
---
_version: '1'
uuid: 81e63bac-4035-47b5-b273-ac13e47a2ff6
created: 2025-07-22T13:14:40.510075462Z
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
---
# SYS-001


```

**Generated parent fields**:
- `uuid`: Copied from USR-001's UUID
- `fingerprint`: Computed from USR-001's current content
- `hrid`: USR-001 (for human readability in frontmatter)

**Generated heading**:
- `# SYS-001`: First heading with HRID (you'll add the title text when editing)

### Multiple Parents

```bash
req create SYS --parent USR-001,USR-002,USR-003
```

Creates `SYS-001.md` with three parent entries:

```yaml
parents:
- uuid: <USR-001 UUID>
  fingerprint: <USR-001 fingerprint>
  hrid: USR-001
- uuid: <USR-002 UUID>
  fingerprint: <USR-002 fingerprint>
  hrid: USR-002
- uuid: <USR-003 UUID>
  fingerprint: <USR-003 fingerprint>
  hrid: USR-003
```

## Using Namespaces

Create requirements with namespaces by including them in the KIND:

```bash
# Create AUTH-USR-001
req create AUTH-USR

# Create AUTH-LOGIN-SYS-001
req create AUTH-LOGIN-SYS

# Create PAYMENT-API-SWR-001
req create PAYMENT-API-SWR
```

The filename matches the full HRID:

```
AUTH-USR-001.md
AUTH-LOGIN-SYS-001.md
PAYMENT-API-SWR-001.md
```

## Workflow Recommendations

### Create High-Level First

Start with user requirements:

```bash
req create USR
req create USR
req create USR
```

Edit them to add content. Then create system requirements that satisfy them:

```bash
req create SYS --parent USR-001
req create SYS --parent USR-001
req create SYS --parent USR-002
```

This top-down approach ensures traceability from the start.

### Create in Batches

If you know you need five user requirements:

```bash
for i in {1..5}; do req create USR; done
```

This creates `USR-001` through `USR-005`. Then edit each file to add content.

### Use Version Control

After creating requirements, commit them:

```bash
req create USR
req create USR
git add USR-001.md USR-002.md
git commit -m "Add initial user requirements (placeholders)"

# Edit the files, then commit again
git add USR-001.md USR-002.md
git commit -m "Add content to USR-001 and USR-002"
```

This creates a clear history of requirement evolution.

## Error Conditions

### Parent Not Found

```bash
req create SYS --parent USR-999
```

If `USR-999.md` doesn't exist, you'll get an error:

```
Error: Failed to load requirement USR-999
```

**Solution**: Ensure parent requirements exist before linking to them.

### Invalid HRID Format

```bash
req create usr-001  # Trying to specify ID manually (not supported)
```

The KIND should not include the ID. Use:

```bash
req create USR  # Correct - ID is auto-assigned
```

### Malformed Parent List

```bash
req create SYS --parent USR-001, USR-002  # Space after comma
```

Don't include spaces in the parent list:

```bash
req create SYS --parent USR-001,USR-002  # Correct
```

## Advanced: Scripting Requirement Creation

For bulk creation with structured content, use shell scripts:

```bash
#!/bin/bash

# Create and populate multiple user requirements
requirements=(
  "Users shall authenticate via username and password"
  "Users shall reset forgotten passwords via email"
  "Users shall enable two-factor authentication"
)

for i in "${!requirements[@]}"; do
  req create USR
  hrid="USR-$(printf '%03d' $((i+1)))"

  # Append requirement text to the file
  echo "${requirements[$i]}" >> "$hrid.md"
done
```

This creates USR-001, USR-002, and USR-003 with predefined content.

## Next Steps

Once you've created requirements, you'll often need to link them together. Learn how in [Linking Requirements](./linking.md).
