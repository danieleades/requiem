# Creating Requirements

The `req add` command creates new requirements with automatically generated metadata.

## Basic Usage

```bash
req add <KIND>
```

This creates a requirement with the next available ID for the specified KIND.

### Examples

```bash
# Create USR-001
req add USR

# Create USR-002
req add USR

# Create SYS-001
req add SYS
```

Output:

```
Added requirement USR-001
Added requirement USR-002
Added requirement SYS-001
```

## Command Options

### Specify Parent Requirements

Create a requirement already linked to parents:

```bash
req add <KIND> --parents <PARENT1>,<PARENT2>,...
```

**Alias**: `-p` for `--parents`

#### Examples

```bash
# Create SYS-001 linked to USR-001
req add SYS --parents USR-001

# Create SYS-002 linked to both USR-001 and USR-002
req add SYS --parents USR-001,USR-002

# Using short form
req add SYS -p USR-001,USR-002
```

### Specify Root Directory

By default, `req` operates in the current directory. Use `--root` to specify a different directory:

```bash
req --root /path/to/requirements add USR
```

**Alias**: `-r` for `--root`

This is useful when running Requiem from outside the requirements directory:

```bash
# Run from project root
req --root ./docs/requirements add USR

# Using short form
req -r ./docs/requirements add USR
```

### Verbosity

Control log output:

```bash
req -v add USR      # INFO level
req -vv add USR     # DEBUG level
req -vvv add USR    # TRACE level
```

More v's = more verbose. Useful for troubleshooting.

## Generated Content

When you run `req add USR`, a file `USR-001.md` is created with:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---


```

**Generated fields**:
- `_version`: Always `'1'` (current format version)
- `uuid`: A random UUIDv4
- `created`: Current timestamp in ISO 8601 format (UTC)

**Empty body**: You need to add the requirement text yourself.

## Adding Content

After creating a requirement, edit it to add content:

```bash
req add USR
# Output: Added requirement USR-001

# Edit the file
vim USR-001.md
# or
code USR-001.md
```

Add your requirement text:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---

Users shall be able to export data in CSV format.

Rationale: CSV is a universally supported format for data interchange.
```

Save the file. Your requirement is now complete.

## Creating with Parents

When you specify parents, they're included in the frontmatter:

```bash
req add SYS --parents USR-001
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


```

**Generated parent fields**:
- `uuid`: Copied from USR-001's UUID
- `fingerprint`: Computed from USR-001's current content
- `hrid`: USR-001 (for human readability)

### Multiple Parents

```bash
req add SYS --parents USR-001,USR-002,USR-003
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
req add AUTH-USR

# Create AUTH-LOGIN-SYS-001
req add AUTH-LOGIN-SYS

# Create PAYMENT-API-SWR-001
req add PAYMENT-API-SWR
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
req add USR
req add USR
req add USR
```

Edit them to add content. Then create system requirements that satisfy them:

```bash
req add SYS --parents USR-001
req add SYS --parents USR-001
req add SYS --parents USR-002
```

This top-down approach ensures traceability from the start.

### Create in Batches

If you know you need five user requirements:

```bash
for i in {1..5}; do req add USR; done
```

This creates `USR-001` through `USR-005`. Then edit each file to add content.

### Use Version Control

After creating requirements, commit them:

```bash
req add USR
req add USR
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
req add SYS --parents USR-999
```

If `USR-999.md` doesn't exist, you'll get an error:

```
Error: Failed to load requirement USR-999
```

**Solution**: Ensure parent requirements exist before linking to them.

### Invalid HRID Format

```bash
req add usr-001  # Trying to specify ID manually (not supported)
```

The KIND should not include the ID. Use:

```bash
req add USR  # Correct - ID is auto-assigned
```

### Malformed Parent List

```bash
req add SYS --parents USR-001, USR-002  # Space after comma
```

Don't include spaces in the parent list:

```bash
req add SYS --parents USR-001,USR-002  # Correct
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
  req add USR
  hrid="USR-$(printf '%03d' $((i+1)))"

  # Append requirement text to the file
  echo "${requirements[$i]}" >> "$hrid.md"
done
```

This creates USR-001, USR-002, and USR-003 with predefined content.

## Next Steps

Once you've created requirements, you'll often need to link them together. Learn how in [Linking Requirements](./linking.md).
