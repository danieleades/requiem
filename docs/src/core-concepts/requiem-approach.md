# How Requiem Supports These Principles

Now that you understand requirements management principles, let's explore how Requiem's design choices support these practices.

## Plain Text: The Foundation

Requiem's most fundamental decision is storing requirements as plain text (Markdown with YAML frontmatter).

### Benefits

**Human Readable**: Anyone with a text editor can read and edit requirements. No specialized tools required.

```markdown
---
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
---

Users shall be able to export data in CSV format.
```

This is instantly understandable to developers, managers, and stakeholders alike.

**Version Control Native**: Plain text integrates seamlessly with Git, providing:
- Line-by-line diffs showing exactly what changed
- Complete history with blame and log
- Pull request workflows for review
- Branching and merging for parallel development

**Tool Agnostic**: Requirements aren't locked into proprietary formats. You can:
- Search with grep/ripgrep
- Process with scripts (Python, Bash, etc.)
- View in any text editor or IDE
- Preview as rendered Markdown
- Process with static site generators

**Future Proof**: Plain text files from 1970 are still readable today. Your requirements will outlive any proprietary tool.

### Trade-offs

**No GUI**: Requiem doesn't provide a graphical interface. Users must be comfortable with text editors and the command line.

**No Real-time Collaboration**: Unlike cloud-based tools, Requiem doesn't support simultaneous editing. Use Git workflows instead.

## Dual Identifiers: Stability and Usability

Requiem uses both UUIDs and Human-Readable IDs (HRIDs).

### UUIDs: Stable References

```yaml
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
```

**Purpose**: Permanent, globally unique identifier that never changes.

**Enables**:
- Renumbering requirements without breaking links
- Merging requirement sets from different sources
- Machine processing and indexing

**Example**: You can rename `USR-001` to `AUTH-001` without breaking any parent references, because they use the UUID.

### HRIDs: Human Communication

```yaml
hrid: USR-001
```

**Purpose**: Short, memorable identifier for human use.

**Enables**:
- Easy verbal communication: "Did you implement USR-042?"
- Clear code comments: `// Satisfies SYS-078`
- Intuitive filenames: `USR-001.md`

**Example**: In a meeting, saying "UUID 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a" is impractical. "USR-001" is clear and concise.

### Best of Both Worlds

Parent relationships use UUIDs but store HRIDs for convenience:

```yaml
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a  # For machine processing
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001  # For human readers
```

The `req clean` command corrects HRIDs if requirements are renumbered, maintaining human-readability while UUIDs ensure correctness.

## Content Fingerprinting: Change Detection

Every requirement's content is hashed to create a fingerprint:

```yaml
fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
```

### How It Works

The fingerprint is a SHA256 hash of:
- The requirement text (markdown body)
- Any tags

This means:
- ✓ Changing the requirement text → new fingerprint
- ✓ Adding/removing tags → new fingerprint
- ✗ Changing HRID → same fingerprint (HRID is just a label)
- ✗ Adding parents → same fingerprint (relationships are separate)

### Enables Change Detection

When a parent requirement changes:

1. Parent's fingerprint updates
2. Children still have old fingerprint
3. Children are "suspect" - they might need review

*Note: Automatic detection is planned but not yet implemented. Currently, you detect stale fingerprints manually or via scripts.*

### Future: Review Workflows

Planned features:
- Automatically flag requirements with stale parent fingerprints
- Assign reviews to stakeholders
- Track review status (current, suspect, under review, approved)
- Generate reports of suspect requirements

## Multiple Parents: Modeling Reality

Many requirements management tools force a strict tree hierarchy. Requiem supports **multiple parents** because that's how real systems work.

### Why Multiple Parents Matter

Example: A logging system might satisfy:
- USR-042: "Debugging capability"
- USR-078: "Audit trail for compliance"
- USR-091: "Performance monitoring"

Representing this as a tree forces artificial choices:

```
# Tree (artificial)
USR-042 (Debugging)
  └─ SYS-123 (Logging)  # Also satisfies USR-078 and USR-091, but structure doesn't show this
```

With multiple parents:

```
SYS-123 (Logging)
  ├─ USR-042 (Debugging)
  ├─ USR-078 (Audit trail)
  └─ USR-091 (Performance monitoring)
```

This accurately models the requirement's purpose.

### Implementation

```yaml
parents:
- uuid: <uuid-1>
  hrid: USR-042
- uuid: <uuid-2>
  hrid: USR-078
- uuid: <uuid-3>
  hrid: USR-091
```

### Trade-off: Cycles

Multiple parents enable **cycles** (requirement A depends on B depends on A). Cycles are usually errors.

*Note: Cycle detection is planned but not yet implemented.*

## Namespace Support: Scaling to Large Projects

HRIDs support optional namespaces:

```
USR-001              # Simple
COMPONENT-USR-001    # One namespace level
AUTH-LOGIN-SYS-042   # Multiple namespace levels
```

### Use Cases

**Large projects**: Partition requirements by subsystem

```
AUTH-USR-001    # Authentication user requirements
AUTH-SYS-001
PAYMENT-USR-001 # Payment user requirements
PAYMENT-SYS-001
```

**Product families**: Distinguish product-specific vs. shared requirements

```
CORE-SYS-001    # Shared across all products
MOBILE-USR-001  # Mobile app specific
WEB-USR-001     # Web app specific
```

**Acquisitions/Mergers**: Integrate requirement sets without renumbering

```
LEGACY-SYS-001  # From acquired company
NEW-SYS-001     # Newly created
```

### Implementation

Namespaces are part of the HRID format:

```rust
// Format: {NAMESPACE*}-{KIND}-{ID}
NAMESPACE-NAMESPACE-KIND-042
```

Configure allowed namespaces in `config.toml` (optional).

## Configuration and Flexibility

Requiem provides sane defaults but allows customization:

### config.toml

```toml
_version = "1"

# Restrict to specific requirement kinds
allowed_kinds = ["USR", "SYS", "SWR", "TST"]

# Digits in HRID numbering (e.g., 001 vs 0001)
digits = 3

# Allow markdown files that aren't requirements
allow_unrecognised = false
```

This balances flexibility (customize as needed) with safety (defaults prevent common errors).

## Integration Philosophy: Compose, Don't Replace

Requiem integrates with existing tools rather than replacing them:

### Documentation Tools

Use with **MdBook** or **Sphinx** to embed requirements in user documentation:

```markdown
# User Guide - Data Export

{{#include ../requirements/USR-001.md}}

To export data, click the Export button...
```

Requirements stay synchronized with documentation automatically.

### Version Control

Works naturally with **Git** workflows:
- Feature branches for requirement changes
- Pull requests for review
- Merge commits for approval
- Tags for releases/versions

### Static Analysis

Plain-text enables custom tooling:

```bash
# Find all USR requirements not linked by any SYS requirement
comm -23 \
  <(ls USR-*.md | sort) \
  <(grep -oh "USR-[0-9]*" SYS-*.md | sort -u)
```

### CI/CD Integration

Add requirement validation to CI:

```yaml
# .github/workflows/requirements.yml
- name: Validate requirements
  run: req clean --dry-run  # Check HRIDs are correct
```

## Performance: Built for Scale

Requiem is written in Rust and uses parallelism for operations on large requirement sets.

### Parallel Loading

When loading a directory with thousands of requirements:

```rust
// Uses rayon for parallel iteration
let requirements: Vec<_> = md_paths
    .par_iter()
    .map(|path| load_requirement(path))
    .collect();
```

This means Requiem scales to large projects (1000s of requirements) without becoming sluggish.

### Efficient Indexing

Requirements are indexed by UUID in a HashMap, enabling O(1) lookups:

```rust
let req = tree.requirement(uuid);  // Constant time
```

## Design Trade-offs

Every design involves trade-offs. Requiem prioritizes:

**Over** graphical interfaces: **Plain text and CLI**
- Pro: Version control, scripting, no vendor lock-in
- Con: Steeper learning curve for non-technical users

**Over** centralized databases: **Distributed files**
- Pro: Works offline, natural with Git, simple deployment
- Con: No real-time collaboration, requires file system access

**Over** strict tree hierarchies: **Multiple parents (DAGs)**
- Pro: Models reality accurately
- Con: Enables cycles (requires detection)

**Over** comprehensive built-in features: **Composability**
- Pro: Integrate with existing tools, stay focused
- Con: Some features require external tools or scripts

## Summary: Why Requiem?

Requiem's design supports requirements management principles through:

1. **Plain text** - Readable, versionable, future-proof
2. **Dual identifiers** - Stable UUIDs + usable HRIDs
3. **Fingerprinting** - Detect changes, enable reviews
4. **Multiple parents** - Model complex dependencies accurately
5. **Namespace support** - Scale to large, multi-component projects
6. **Composable** - Works with Git, MdBook, Sphinx, custom tools
7. **Fast** - Parallel processing for large requirement sets

These choices make Requiem a powerful tool for teams that value:
- Version control integration
- Plain-text workflows
- Speed and scalability
- Flexibility and composability

Ready to dive deeper into practical usage? Continue to [Working with Requirements](../working-with-requirements.md).
