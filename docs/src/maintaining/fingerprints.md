# Fingerprints and Change Detection

Requiem uses content fingerprints to detect when requirements change. This chapter explains how fingerprints work and enable change management.

## What is a Fingerprint?

A fingerprint is a cryptographic hash (SHA256) of a requirement's semantic content. It's stored in parent references:

```yaml
# SYS-001.md
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
```

The fingerprint represents the state of `USR-001` when `SYS-001` was last reviewed or updated.

## How Fingerprints Work

### Creation

When you link a child to a parent:

```bash
req link SYS-001 USR-001
```

Requiem:
1. Computes the fingerprint of `USR-001`'s content
2. Stores it in `SYS-001`'s parent reference

### Updates

Fingerprints update in two scenarios:

**1. When creating the link**:
```bash
req link SYS-001 USR-001
# Fingerprint captured at link time
```

**2. When the parent's content changes**:
- The parent's fingerprint is recomputed automatically
- Child references keep the OLD fingerprint
- This creates a mismatch, indicating the child may need review

### What's Included in the Fingerprint

Fingerprints hash:
- ✓ **Markdown body**: The requirement text
- ✓ **Tags**: Any tags in the frontmatter

Fingerprints **do NOT** include:
- ✗ HRID (just a label)
- ✗ UUID (stable identifier, not content)
- ✗ Created timestamp (metadata, not content)
- ✗ Parent relationships (separate from content)

### Example

**Requirement USR-001**:
```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
tags:
- authentication
- security
---

The system shall validate user email addresses according to RFC 5322.
```

**Fingerprint computation**:
```
Content: "The system shall validate user email addresses according to RFC 5322."
Tags: ["authentication", "security"]
→ Encode with Borsh
→ Hash with SHA256
→ Fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
```

## Use Cases

### Change Detection

**Problem**: A parent requirement changes. Which child requirements are affected?

**Solution**: Compare fingerprints.

**Example**:

1. Initial state:
```yaml
# SYS-001.md links to USR-001
parents:
- uuid: 4bfeb7d5-...
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
```

2. Someone edits `USR-001`:
```markdown
# USR-001.md
The system shall validate user email addresses according to RFC 5322.
Email validation must occur before account creation.  # ← New sentence
```

3. USR-001's fingerprint changes:
```
New fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
```

4. SYS-001 still has the old fingerprint:
```yaml
# SYS-001.md
parents:
- fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda  # Old!
```

5. **Mismatch detected**: SYS-001 needs review because its parent changed.

### Impact Analysis

**Question**: If I change this requirement, what else is affected?

**Answer**: Use the `req suspect` command after making changes to find affected children.

**Workflow**:

```bash
# 1. Edit a parent requirement
vim USR-001.md  # Make your changes

# 2. Check for suspect links
req suspect
# Output shows all affected children:
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   ... (fingerprint mismatches)

# 3. Review each affected child
vim SYS-001.md
vim SYS-002.md

# 4. Accept fingerprints after review
req accept SYS-001 USR-001
req accept SYS-002 USR-001

# Or accept all at once
req accept --all
```

**Manual process** (if needed):

```bash
# 1. Get UUID of the requirement you're changing
grep "uuid:" USR-001.md

# 2. Find all requirements that reference this UUID
grep -r "uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a" *.md

# 3. Those requirements may need review after your change
```

### Review Tracking

**Goal**: Track which requirements need review after upstream changes.

**Current state**: Basic suspect link detection implemented via `req suspect` and `req accept` commands.

**Available now**:
- Detect fingerprint mismatches with `req suspect`
- Accept individual links with `req accept <child> <parent>`
- Accept all suspect links with `req accept --all`
- CI/CD integration via exit codes

**Future**: Advanced review workflows with status tracking and assignments (planned feature).

## Computing Fingerprints

### Viewing a Requirement's Fingerprint

No built-in command yet, but you can compute it manually:

**Option 1: Link to a temporary requirement**

```bash
# Create temp requirement
req add TEMP
# Link to target
req link TEMP-001 USR-001
# View fingerprint in TEMP-001.md
grep "fingerprint:" TEMP-001.md
# Clean up
rm TEMP-001.md
```

**Option 2: Use Rust/Python script**

Not currently exposed via CLI. Requires custom scripting.

### Comparing Fingerprints

**Problem**: Is my child requirement's parent reference current?

**Solution**: Use the `req suspect` command:

```bash
req suspect
```

This automatically:
1. Loads all requirements
2. Compares stored parent fingerprints with current parent fingerprints
3. Lists all mismatched fingerprints (suspect links)

**Example output**:
```
Found 2 suspect link(s):

  SYS-001 → USR-001
    Stored fingerprint:  e533784ff58c16cb
    Current fingerprint: c4020419ead000e9

  SYS-002 → USR-001
    Stored fingerprint:  e533784ff58c16cb
    Current fingerprint: c4020419ead000e9
```

If no suspect links exist:
```
No suspect links found.
```

**Exit codes**:
- `0`: All fingerprints current
- `1`: Suspect links found (useful for CI/CD)

**Manual process** (if needed):

1. Find parent UUID and stored fingerprint in child:
```yaml
# SYS-001.md
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
```

2. Compute current fingerprint of parent (see above)

3. Compare:
   - Match: Child is current
   - Mismatch: Child needs review

## What Changes Affect Fingerprints

### Changes that Update Fingerprints

**Editing requirement text**:
```markdown
# Before
The system shall validate emails.

# After
The system shall validate emails according to RFC 5322.
```
→ Fingerprint changes

**Adding/removing tags**:
```yaml
# Before
tags:
- authentication

# After
tags:
- authentication
- security
```
→ Fingerprint changes

**Modifying tags**:
```yaml
# Before
tags:
- high-priority

# After
tags:
- medium-priority
```
→ Fingerprint changes

**Whitespace changes in content**:
```markdown
# Before
The system shall validate emails.

# After
The system shall validate emails.

```
→ Fingerprint changes (trailing whitespace added)

### Changes that DON'T Affect Fingerprints

**Renaming the requirement**:
```bash
mv USR-001.md USR-100.md
```
→ Fingerprint unchanged (HRID is not part of content)

**Changing UUID** (don't do this!):
```yaml
# Before
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a

# After
uuid: 00000000-0000-0000-0000-000000000000
```
→ Fingerprint unchanged (but breaks traceability!)

**Changing created timestamp**:
```yaml
# Before
created: 2025-07-22T12:19:56Z

# After
created: 2025-08-01T10:00:00Z
```
→ Fingerprint unchanged

**Adding/removing parents**:
```yaml
# Before
parents:
- uuid: ...

# After
parents:
- uuid: ...
- uuid: ...  # Added parent
```
→ Fingerprint unchanged

## Fingerprint Algorithms

### Current Algorithm

**Encoding**: Borsh (Binary Object Representation Serializer for Hashing)
- Deterministic serialization
- Consistent across platforms
- Efficient for hashing

**Hashing**: SHA256
- Cryptographically secure
- 256-bit output (64 hex characters)
- Collision resistant

**Process**:
```
1. Collect content and tags
2. Serialize with Borsh: content + tags → binary
3. Hash with SHA256: binary → 256-bit hash
4. Encode as hex: hash → 64-character string
```

### Why These Choices?

**Borsh**:
- Stable encoding (no ambiguity)
- Handles strings and collections consistently
- Designed for hashing use cases

**SHA256**:
- Industry standard
- Strong collision resistance
- Fast to compute

**Benefits**:
- Consistent fingerprints across systems
- Detects even small changes
- Impossible to forge (cryptographically secure)

## Practical Examples

### Example 1: Detecting Stale References

**Scenario**: USR-001 was updated 2 months ago. Are any child requirements out of date?

**Process**:

```bash
# Check for suspect links
req suspect
```

**Output**:
```
Found 1 suspect link(s):

  SYS-001 → USR-001
    Stored fingerprint:  e533784ff58c16cb
    Current fingerprint: c4020419ead000e9
```

**Result**: SYS-001 has a stale fingerprint and needs review.

**Next steps**:
```bash
# Review changes
vim USR-001.md  # See what changed
vim SYS-001.md  # Update if needed

# Accept the link
req accept SYS-001 USR-001

# Verify clean
req suspect
# Output: No suspect links found.
```

### Example 2: Tag Changes

**Scenario**: Add a tag to a requirement.

**Before**:
```yaml
# USR-001.md
tags:
- authentication
```

Fingerprint: `e533784f...`

**After**:
```yaml
# USR-001.md
tags:
- authentication
- security
```

Fingerprint: `c4020419...` (changed!)

**Impact**: Any child requirements now have stale fingerprints.

### Example 3: Whitespace-Only Changes

**Scenario**: Reformatting requirement text.

**Before**:
```markdown
The system shall validate emails.
```

**After**:
```markdown
The system shall validate emails.

```
(Added blank line)

**Result**: Fingerprint changes!

**Caution**: Even whitespace affects fingerprints. Be mindful of formatting changes.

## Limitations and Future Work

### Implemented Features

**✓ Automatic suspect link detection**

Use `req suspect` to find all stale fingerprints:
```bash
req suspect
# Lists all requirements with fingerprint mismatches
```

**✓ Accepting suspect links**

Update fingerprints after review:
```bash
# Accept individual link
req accept SYS-001 USR-001

# Accept all suspect links
req accept --all
```

**✓ CI/CD integration**

Exit codes enable automation:
```bash
req suspect
# Exit 0 if clean, exit 1 if suspect links found
```

### Current Limitations

**1. No review state tracking**

No tracking of review status (current, under review, approved).

**Planned**: Review state management with status tracking and assignments.

**2. No reporting**

Can't generate comprehensive reports of requirement status.

**Planned**: `req report` command for traceability and review status.

**3. No impact visualization**

Can't see full dependency tree affected by a change.

**Planned**: `req impact USR-001` to show affected descendants.

**4. No fingerprint diff**

Can't see what content changed between fingerprints.

**Planned**: `req diff USR-001` to show fingerprint changes and content diff.

**5. Whitespace sensitivity**

Formatting changes trigger fingerprint updates.

**Trade-off**: Precision vs. false positives. Current design prioritizes detecting all changes.

### Planned Features

**Review state management**:
```bash
req review start SYS-001
# Mark SYS-001 as "under review"

req review complete SYS-001
# Mark as "reviewed" and update fingerprints

req status
# Output:
# USR-001: current
# SYS-001: reviewed (parents unchanged since review)
# SYS-002: needs review (parent changed)
```

**Impact analysis**:
```bash
req impact USR-001
# Output:
# Direct children:
#   SYS-001, SYS-002
# Indirect descendants:
#   SWR-001 (via SYS-001)
#   TST-001 (via SWR-001)
# Total affected: 4 requirements
```

**Reporting**:
```bash
req report review
# Generate review status report with metrics
```

## Best Practices

### 1. Link Immediately After Creation

When creating a requirement with parents:

```bash
# Create requirement
req add SYS --parents USR-001

# Fingerprint is captured immediately
```

This ensures the fingerprint represents the baseline.

### 2. Check for Suspect Links Regularly

Periodically check for fingerprint mismatches:

```bash
# Check for suspect links
req suspect

# Before committing changes
req suspect || (echo "Review needed" && exit 1)

# In CI/CD pipeline
req suspect
```

### 3. Accept Links After Review

After reviewing parent changes and updating children:

```bash
# Accept individual link
req accept SYS-001 USR-001

# Or accept all after bulk review
req accept --all
```

This updates fingerprints to acknowledge the review.

### 4. Document Review Process

Include fingerprint checking in your review process:

```markdown
## Requirement Review Checklist

1. Check for suspect links: `req suspect`
2. For each suspect link:
   - Review parent changes
   - Review child requirement text
   - Update child if needed
   - Accept link: `req accept CHILD PARENT`
3. Verify all clean: `req suspect`
4. Commit changes
```

### 5. Avoid Trivial Changes

Minimize whitespace-only or formatting changes to reduce fingerprint churn:

- Use consistent formatting from the start
- Configure editor to preserve formatting
- Avoid unnecessary reformatting

## Troubleshooting

### Unexpected Fingerprint Changes

**Issue**: Fingerprint changed but content looks the same.

**Causes**:
1. Whitespace changes (trailing spaces, blank lines)
2. Tag modifications
3. Character encoding differences

**Diagnosis**:
```bash
# Show all changes including whitespace
git diff --ws-error-highlight=all USR-001.md
```

### Fingerprint Not Updating

**Issue**: Changed requirement but fingerprint seems unchanged.

**Explanation**: Fingerprints are stored in parent references, not in the requirement itself.

**Check**: Look at a child requirement's parent reference to see the fingerprint.

### Manual Fingerprint Edit

**Issue**: Accidentally edited a fingerprint in frontmatter.

**Impact**: Child will show incorrect fingerprint for parent.

**Fix**: Re-link the requirement:
```bash
req link SYS-001 USR-001
# This recalculates and stores the correct fingerprint
```

## Summary

**Key Concepts**:

- **Fingerprint**: SHA256 hash of requirement content and tags
- **Purpose**: Detect when parent requirements change
- **Storage**: Stored in child's parent reference
- **Automatic**: Computed when linking requirements
- **Immutable**: Old fingerprints preserved in children, enabling change detection

**What's Included**: Markdown body + tags

**What's Excluded**: HRID, UUID, timestamps, parent relationships

**Use Cases**: Change detection, impact analysis, review tracking

**Commands Available**:

- `req suspect` - List all suspect links (fingerprint mismatches)
- `req accept <CHILD> <PARENT>` - Accept suspect link after review
- `req accept --all` - Accept all suspect links

**Current State**: Basic suspect link detection implemented

**Future**: Advanced review workflows with state tracking and assignments

## Next Steps

- Learn about [Review Workflows](./review-workflows.md) (planned feature)
- Understand [Correcting HRIDs](./correcting-hrids.md) for maintaining references
