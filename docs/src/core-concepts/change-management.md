# Change Management

Requirements change. Users discover new needs, technology evolves, regulations update, and understanding deepens. Effective requirements management doesn't fight change - it embraces and manages it.

## Why Requirements Change

Common drivers of change:

- **Evolving understanding** - Stakeholders clarify needs as they see prototypes
- **External factors** - New regulations, competitor features, market shifts
- **Technical discoveries** - Implementation reveals previously unknown constraints
- **Scope refinement** - Priorities shift as the project progresses
- **Error correction** - Requirements contained mistakes or ambiguities

## Change Management Goals

Effective change management aims to:

1. **Track changes** - Know what changed, when, and why
2. **Analyze impact** - Understand ripple effects before committing
3. **Notify stakeholders** - Alert affected parties
4. **Trigger reviews** - Ensure dependent artifacts are updated
5. **Maintain history** - Preserve the evolution for audit and learning

## Requiem's Change Management Features

### Content Fingerprinting

Every requirement has a **fingerprint** - a SHA256 hash of its content:

```yaml
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
```

When a parent requirement changes, its fingerprint changes. This allows child requirements to detect that their parent has been modified.

### How Fingerprinting Works

The fingerprint is computed from:
- The requirement's text content (markdown body)
- Any tags on the requirement

It does **not** include:
- UUID (never changes)
- HRID (might be renumbered)
- Creation timestamp
- Parent relationships

This means fingerprints change **only when meaningful content changes**.

### Detecting Changes

When a parent requirement is edited, its fingerprint becomes stale in child requirements:

```yaml
# Parent USR-001 was edited, fingerprint is now abc123...
# But child still references old fingerprint:
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda  # Old!
  hrid: USR-001
```

*Note: Automated detection of stale fingerprints and triggering reviews is planned but not yet implemented.*

### Version Control Integration

Requiem's plain-text format provides powerful change management through Git:

#### Viewing Changes

```bash
git diff USR-001.md
```

Shows exactly what changed:

```diff
-Users shall be able to create tasks with a title and description.
+Users shall be able to create tasks with a title, description, and due date.
```

#### Change History

```bash
git log --follow USR-001.md
```

Shows complete history of a requirement, including:
- Who changed it
- When it changed
- Why (from commit messages)

#### Blame/Annotate

```bash
git blame USR-001.md
```

Shows who last modified each line, useful for finding the source of specific clauses.

## Change Workflow

A typical requirement change workflow:

### 1. Propose Change

Create a branch:

```bash
git checkout -b update-authentication-requirements
```

Edit the requirement:

```markdown
# USR-001.md
-Users shall authenticate using username and password.
+Users shall authenticate using username and password, or via OAuth providers (Google, GitHub).
```

### 2. Analyze Impact

Identify affected requirements:

```bash
# Find requirements that reference USR-001
grep -r "USR-001" *.md
```

Review child requirements to see if they need updates:
- SYS-042: "System shall hash passwords with bcrypt" - Still valid
- SYS-043: "System shall rate-limit login attempts" - Needs OAuth rate limiting too!

### 3. Update Dependent Requirements

Update child requirements to reflect the change:

```markdown
# SYS-043.md
-The system shall rate-limit password login attempts to 5 per minute.
+The system shall rate-limit authentication attempts to 5 per minute, including both password and OAuth flows.
```

### 4. Review and Approve

Create a pull request:

```bash
git add USR-001.md SYS-043.md
git commit -m "Add OAuth authentication to USR-001 and update rate limiting in SYS-043"
git push origin update-authentication-requirements
```

The PR shows:
- Exact changes (diff)
- Affected requirements
- Commit message explaining rationale

Stakeholders review and approve.

### 5. Merge and Notify

After approval, merge the PR. Git history preserves:
- What changed
- When it changed
- Who approved it
- Why it changed (commit message)

## Change Impact Analysis

Understanding the impact of a change is crucial. Requiem helps through:

### Parent-Child Relationships

If requirement X changes, all child requirements **may** be affected. Review each to determine if updates are needed.

### Multiple Parents

When a requirement has multiple parents and one changes, evaluate:
- Does the change conflict with other parents?
- Do child requirements still satisfy all parents?

Example:
```
USR-001: "Fast performance"
USR-002: "Strong encryption"
   └─ SYS-042: "Use AES-256 encryption"
```

If USR-001 changes to require sub-millisecond response times, SYS-042's encryption choice might need reconsideration (encryption adds latency).

## Review Triggers

*Note: Automated review triggering is planned but not yet implemented.*

In the future, Requiem will support:

### Automatic Review Flags

When a requirement changes:
1. Its fingerprint updates
2. Child requirements detect stale fingerprints
3. Those requirements are flagged for review
4. Reviews can be assigned to stakeholders
5. Requirements are approved or updated
6. Fingerprints are refreshed

### Review States

Possible states:
- **Current** - Fingerprint matches parent
- **Suspect** - Parent changed, review needed
- **Under Review** - Assigned for review
- **Approved** - Reviewed and still valid despite parent change

## Best Practices

### 1. Write Meaningful Commit Messages

```bash
# Bad
git commit -m "Update requirements"

# Good
git commit -m "Add due date support to USR-001 per stakeholder feedback from 2025-10-15 meeting"
```

### 2. Link Changes to Issues

Reference issue trackers in commits:

```bash
git commit -m "Update authentication requirements (resolves #42)"
```

### 3. Review Dependent Requirements

When changing a requirement, always:
- Search for references to its HRID
- Review child requirements
- Check related documentation
- Update tests

### 4. Use Branches for Changes

Never modify requirements directly on main:
- Create a branch
- Make changes
- Get review
- Merge

This creates an audit trail and enables discussion before commitment.

### 5. Batch Related Changes

If changing USR-001 requires updating SYS-042 and SYS-043, do it in one commit:

```bash
git add USR-001.md SYS-042.md SYS-043.md
git commit -m "Extend authentication to support OAuth (USR-001, SYS-042, SYS-043)"
```

This preserves the logical relationship between changes.

## Change Metrics

Useful metrics for requirement stability:

- **Churn rate** - How often requirements change
- **Ripple effect** - Average number of requirements affected by a change
- **Review latency** - Time from change to review completion
- **Approval rate** - Percentage of proposed changes accepted

High churn in high-level requirements (USR) may indicate poor initial understanding. High churn in low-level requirements (SWR) may be normal as implementation details evolve.

## Handling Breaking Changes

Some changes invalidate child requirements:

**Example:** USR-001 requires "single-user application," but later changes to "multi-user application."

SYS requirements assuming single-user (no authentication, shared global state) are now invalid.

**Process:**
1. Mark affected requirements as "obsolete" or delete them
2. Create new requirements for multi-user scenario
3. Update traceability links
4. Document the change rationale

## Next Steps

Now that you understand how requirements change, see how Requiem's design supports these practices: [How Requiem Supports These Principles](./requiem-approach.md)
