# Linking Requirements

The `req link` command establishes parent-child relationships between requirements, creating traceability throughout your requirement hierarchy.

## Basic Usage

```bash
req link <CHILD> <PARENT>
```

This adds PARENT to CHILD's parent list.

### Example

```bash
req link SYS-001 USR-001
```

Output:

```
Linked SYS-001 to USR-001
```

Now `SYS-001.md` contains:

```yaml
---
_version: '1'
uuid: 81e63bac-4035-47b5-b273-ac13e47a2ff6
created: 2025-07-22T13:14:40.510075462Z
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
---

...requirement content...
```

## Multiple Parents

You can link a child to multiple parents by running `req link` multiple times:

```bash
req link SYS-001 USR-001
req link SYS-001 USR-002
req link SYS-001 USR-003
```

Now `SYS-001.md` has three parents:

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

## Linking at Creation Time

Instead of linking after creation, you can specify parents when creating:

```bash
# Instead of:
req create SYS
req link SYS-001 USR-001
req link SYS-001 USR-002

# Do:
req create SYS --parent USR-001,USR-002
```

This is more efficient and ensures traceability from the start.

## What Gets Updated

When you run `req link CHILD PARENT`:

1. **CHILD file is modified**: The parent entry is added to the YAML frontmatter
2. **PARENT file is NOT modified**: Parent doesn't know about its children

This is a directed relationship: children point to parents, not vice versa.

### Finding Children

To find all children of a requirement, search files:

```bash
grep -l "hrid: USR-001" *.md
```

This shows all requirements that list USR-001 as a parent.

## Parent Information

Each parent entry contains three fields:

### UUID (Required)

```yaml
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
```

The stable, unique identifier of the parent. This is the canonical reference that enables reliable traceability even if the parent is renumbered.

### Fingerprint (Required)

```yaml
fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
```

A SHA256 hash of the parent's content at the time of linking. This enables change detection:

- Parent content changes → fingerprint changes
- Child still has old fingerprint → signals that parent has changed
- Child should be reviewed to ensure it's still valid

### HRID (Required)

```yaml
hrid: USR-001
```

The human-readable ID of the parent. This is for convenience when reading the YAML. The actual relationship is based on the UUID.

If the parent is renumbered, the HRID can be corrected with `req clean` (covered in [Maintaining Requirements](../maintaining.md)).

## Linking Across Namespaces

You can link requirements with different namespaces:

```bash
req link PAYMENT-SYS-001 CORE-USR-001
```

The namespaces don't need to match. Links are based on UUIDs, which are globally unique across all namespaces.

## Error Conditions

### Child Not Found

```bash
req link SYS-999 USR-001
```

If `SYS-999.md` doesn't exist:

```
Error: Failed to load requirement SYS-999
```

### Parent Not Found

```bash
req link SYS-001 USR-999
```

If `USR-999.md` doesn't exist:

```
Error: Failed to load requirement USR-999
```

### Duplicate Link

Linking a child to the same parent twice:

```bash
req link SYS-001 USR-001
req link SYS-001 USR-001
```

This creates duplicate entries in the parent list. **Avoid this** by checking existing links before adding new ones.

*Note: Duplicate detection is not currently implemented but may be added in future versions.*

## Typical Linking Patterns

### One-to-Many (Parent → Children)

One user requirement satisfied by multiple system requirements:

```
USR-001
  ├─ SYS-001
  ├─ SYS-002
  └─ SYS-003
```

```bash
req create USR
req create SYS --parent USR-001
req create SYS --parent USR-001
req create SYS --parent USR-001
```

### Many-to-One (Parents → Child)

One system requirement satisfying multiple user requirements:

```
USR-001 ─┐
USR-002 ─┼─ SYS-001
USR-003 ─┘
```

```bash
req create USR
req create USR
req create USR
req create SYS --parent USR-001,USR-002,USR-003
```

### Many-to-Many

Complex dependencies:

```
USR-001 ─┬─ SYS-001
         └─ SYS-002
USR-002 ─┬─ SYS-002
         └─ SYS-003
```

```bash
req create SYS --parent USR-001
req create SYS --parent USR-001,USR-002
req create SYS --parent USR-002
```

## Best Practices

### Link During Creation When Possible

```bash
# Preferred
req create SYS --parent USR-001

# Works but requires two commands
req create SYS
req link SYS-001 USR-001
```

### Establish Traceability Early

Link requirements as you create them, not as an afterthought. This ensures your hierarchy is always traceable.

### Document Linking Rationale

In the requirement body, explain why each parent link exists:

```markdown
---
parents:
- hrid: USR-001
  ...
- hrid: USR-003
  ...
---

The system shall provide OAuth authentication.

Satisfies USR-001 (secure login) and USR-003 (third-party integration).
```

### Validate Links in Code Review

When reviewing requirement changes, check that:
- New requirements link to appropriate parents
- Links are not duplicated
- Parent requirements actually exist

## Unlinking (Not Yet Supported)

Currently, there's no `req unlink` command. To remove a parent link, manually edit the child file:

```yaml
# Before
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
- uuid: 7a8f9e2b-1c3d-4e5f-6a7b-8c9d0e1f2a3b
  fingerprint: a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
  hrid: USR-002

# After (removed USR-002)
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
```

Be careful to preserve valid YAML syntax.

## Visualizing Links

Since links aren't bidirectional in the files, you may want to generate visual representations:

```bash
# Simple script to list parent-child relationships
for file in *.md; do
  hrid=$(basename "$file" .md)
  parents=$(grep "hrid:" "$file" | awk '{print $2}')
  for parent in $parents; do
    echo "$parent -> $hrid"
  done
done
```

Output:

```
USR-001 -> SYS-001
USR-001 -> SYS-002
USR-002 -> SYS-002
```

You can pipe this to graph visualization tools like Graphviz.

## Next Steps

Learn how to work with complex requirement relationships in [Managing Relationships](./relationships.md).
