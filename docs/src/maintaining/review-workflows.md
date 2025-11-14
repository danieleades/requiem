# Review Workflows

> **Note**: Basic suspect link detection is **implemented**. Advanced review workflow features (state tracking, assignments, notifications) are **planned for future releases**.

Review workflows enable teams to track which requirements need review after upstream changes are detected via fingerprint mismatches.

## Available Now: Suspect Link Detection

Requiem can now automatically detect and manage suspect links:

### Commands

**`req suspect`** - List all requirements with fingerprint mismatches

```bash
req suspect
```

Example output:
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

Exit codes:
- `0`: No suspect links (all current)
- `2`: Suspect links found (useful for CI/CD)

**`req accept <CHILD> <PARENT>`** - Accept a suspect link after review

```bash
req accept SYS-001 USR-001
# Output: Accepted suspect link: SYS-001 → USR-001
```

**`req accept --all`** - Accept all suspect links

```bash
req accept --all
# Output:
# Accepted 3 suspect link(s):
#   SYS-001 → USR-001
#   SYS-002 → USR-001
#   SYS-005 → USR-004
```

### Basic Workflow

```bash
# 1. Check for suspect links
req suspect

# 2. Review parent changes
vim USR-001.md

# 3. Review and update affected children
vim SYS-001.md
vim SYS-002.md

# 4. Accept links after review
req accept SYS-001 USR-001
req accept SYS-002 USR-001

# Or accept all at once
req accept --all

# 5. Verify clean
req suspect
# Output: No suspect links found.
```

### CI/CD Integration

Use in continuous integration:

```bash
#!/bin/bash
# Fail build if suspect links exist
req suspect
if [ $? -ne 0 ]; then
    echo "ERROR: Requirements need review"
    exit 1
fi
```

Or in GitHub Actions:

```yaml
- name: Check for suspect links
  run: |
    req suspect || (echo "Requirements need review" && exit 1)
```

## Planned Advanced Functionality

### Automatic Review Triggers

When a parent requirement changes:

1. Requiem detects fingerprint mismatch in child requirements
2. Child requirements are flagged as "needs review"
3. Team members are notified (configurable)
4. Reviews are tracked until completed

### Review States

Requirements will have review states:

- **Current**: No parent changes; requirement is up to date
- **Suspect**: Parent changed (fingerprint mismatch); needs review
- **Under Review**: Review in progress
- **Reviewed**: Review completed; requirement updated as needed
- **Approved**: Reviewed and approved; baseline updated

### Commands (Planned)

```bash
# Check for requirements needing review
req suspect

# Mark requirement as under review
req review start SYS-001

# Complete review and update fingerprint
req review complete SYS-001

# Show review status
req status

# Generate review report
req report review
```

## Motivation

### The Problem

**Scenario**: A user requirement changes. Which system requirements are affected?

**Current state**: Manual tracking required.

**Example**:

1. Edit USR-001: Change email validation rules
2. SYS-001, SYS-002, and SYS-005 reference USR-001
3. **Challenge**: Remember to review all three system requirements
4. **Risk**: Forget to review one; inconsistency results

### The Solution

Automatic review tracking:

1. Edit USR-001
2. Requiem automatically flags SYS-001, SYS-002, SYS-005 as "needs review"
3. Team dashboard shows requirements pending review
4. Reviews are tracked and reported
5. Nothing falls through the cracks

## How It Works (Current Implementation)

### Step 1: Make Changes

Edit a requirement:

```bash
vim USR-001.md  # Make your changes
```

The fingerprint of USR-001 automatically changes when content or tags change.

### Step 2: Detect Suspect Links

Find affected children:

```bash
req suspect
```

Output:
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

### Step 3: Conduct Review

Review each affected requirement:

```bash
# Review parent changes
vim USR-001.md

# Review child and update if needed
vim SYS-001.md
```

### Step 4: Accept Links

After review, accept the fingerprint change:

```bash
# Accept individual link
req accept SYS-001 USR-001

# Or accept all at once
req accept --all
```

Output:
```
Accepted 3 suspect link(s):
  SYS-001 → USR-001
  SYS-002 → USR-001
  SYS-005 → USR-004
```

### Step 5: Verify Clean

Confirm no suspect links remain:

```bash
req suspect
# Output: No suspect links found.
```

## How Advanced Features Will Work (Planned)

### Review State Tracking

Track review progress with states:

```bash
req review start SYS-001
# - Marks requirement as "under review"
# - Records reviewer and timestamp

req review complete SYS-001
# - Marks requirement as "reviewed"
# - Updates parent fingerprints
# - Clears review flag
```

### Status Reports

Generate review reports:

```bash
req report review
```

Output:
```
Review Status Report (2025-07-22)

Pending Reviews:
  SYS-002: parent USR-001 changed (flagged 3 days ago)
  SYS-005: parent USR-001 changed (flagged 3 days ago)

Recently Reviewed:
  SYS-001: reviewed by alice@example.com (2025-07-22)

All Current:
  SYS-003, SYS-004, ...
```

## Use Cases

### Use Case 1: Change Impact Analysis

**Scenario**: Proposing a change to a critical requirement.

**Question**: What's the impact?

**Workflow**:

1. Identify requirement to change (USR-001)
2. Check impact:
```bash
req impact USR-001
```
3. Output shows all descendants:
```
Requirements that depend on USR-001:
  Direct children:
    SYS-001, SYS-002, SYS-005
  Indirect descendants:
    SWR-001, SWR-003 (via SYS-001)
    TST-001, TST-002 (via SYS-001, SYS-002)

Total affected: 7 requirements
```
4. Decide if change is worth the review burden

### Use Case 2: Release Readiness

**Scenario**: Preparing for a release.

**Question**: Are all requirements reviewed and current?

**Workflow**:

```bash
req status
```

Sample output:
```
Requirement Counts
==================
Kind       | Count
-----------+-----
SYS        | 118
TST        | 24
USR        | 6
-----------+-----
Total      | 148

Suspect links: 3
```

If the command exits with code `2`, use the suspect link total as your release checklist—clear
them before shipping:
```bash
req suspect
# Review each flagged requirement
req review complete ...
```

### Use Case 3: Compliance Audits

**Scenario**: Demonstrating requirements traceability for audit.

**Goal**: Show that all requirements were reviewed after changes.

**Workflow**:

```bash
req report audit --since 2025-01-01
```

Output:
```
Audit Report: Jan 1 - Jul 22, 2025

Requirements Changed: 23
  - USR-001, USR-005, SYS-003, ...

Reviews Conducted: 47
  - All downstream requirements reviewed
  - Average review time: 2.3 days

Compliance: ✓ PASS
  - All changed requirements reviewed
  - All affected descendants reviewed
```

## Configuration (Planned)

### Review Policies

```toml
# config.toml (planned)
[review]
# Automatically flag children when parent changes
auto_flag = true

# Require review completion before allowing further changes
block_on_pending_review = false

# Notification settings
notify_on_flag = true
notification_email = "team@example.com"

# Review SLA (days)
review_sla = 7
```

### Review Rules

```toml
# config.toml (planned)
[review.rules]
# Require review for specific requirement kinds
require_review_for = ["USR", "SYS"]

# Skip review for certain kinds (e.g., documentation)
skip_review_for = ["DOC"]

# Require multiple approvers for critical requirements
require_approvals = 2  # For requirements tagged "critical"
```

## Integration

### Git Integration

Reviews tracked alongside code changes:

```bash
# Edit requirement
vim USR-001.md

# Flag affected requirements
req suspect

# Create PR with review tracking
git checkout -b update-usr-001
git add USR-001.md
git commit -m "Update USR-001: clarify email validation"

# PR description includes:
# - Changed requirement
# - Affected requirements
# - Review checklist
```

### CI/CD Integration

```yaml
# .github/workflows/requirements.yml (planned)
- name: Check review status
  run: |
    req suspect
    if [ $? -ne 0 ]; then
      echo "Requirements need review"
      exit 1
    fi
```

### Issue Tracker Integration

Automatically create issues for flagged requirements:

```bash
req suspect --format json | ./scripts/create-issues
# Use the JSON output to open tracking issues for each requirement needing review
```

## Current Workflow Examples

### Example 1: Daily Review Check

Check for suspect links before starting work:

```bash
#!/bin/bash
# daily-check.sh

echo "Checking for suspect links..."
req suspect

if [ $? -eq 0 ]; then
    echo "✓ All requirements current"
else
    echo "⚠ Review needed - see above"
fi
```

### Example 2: Pre-Commit Hook

Prevent commits with unreviewed changes:

```bash
#!/bin/bash
# .git/hooks/pre-commit

req suspect
if [ $? -ne 0 ]; then
    echo "ERROR: Suspect links found. Run 'req suspect' to see them."
    echo "Review and accept links before committing."
    exit 1
fi
```

### Example 3: Pull Request Workflow

```bash
# 1. After making changes
git add -A
git commit -m "Update USR-001 validation rules"

# 2. Check for suspect links
req suspect

# 3. Review and update affected requirements
vim SYS-001.md
vim SYS-002.md

# 4. Accept all after review
req accept --all

# 5. Commit accepted fingerprints
git add -A
git commit -m "Accept fingerprints after USR-001 changes"

# 6. Push
git push
```

### Example 4: Bulk Review Session

Review all pending changes at once:

```bash
# List all suspect links
req suspect > review-list.txt

# Review each one
vim SYS-001.md
vim SYS-002.md
# ... review all

# Accept all at once
req accept --all

# Verify clean
req suspect
```

## Supplementary Tracking (Optional)

While `req suspect` and `req accept` handle basic detection and acceptance, you may want additional tracking:

### Git Commit Messages

Document reviews in commit messages:

```bash
git commit -m "Review SYS-001 after USR-001 change

USR-001 updated email validation rules.
Reviewed SYS-001 and verified consistency.
No changes needed to SYS-001.

Accepted fingerprint: c4020419ead000e9"
```

### Issue Tracking

Create issues for complex reviews:

```markdown
## Review affected requirements after USR-001 change

- [x] SYS-001: Reviewed, no changes needed
- [x] SYS-002: Updated validation logic
- [ ] SYS-005: Waiting for clarification
```

### Tags in Frontmatter

Track review priority with tags:

```yaml
# SYS-001.md
tags:
- high-priority
- security-critical
```

## Future Enhancements

Beyond basic review workflows:

### Advanced Features

**Review assignments**:
```bash
req review assign SYS-001 --to alice@example.com
```

**Review templates**:
```bash
req review start SYS-001 --template checklist.md
# Provides structured review checklist
```

**Review history**:
```bash
req history SYS-001
# Shows all reviews and changes over time
```

**Bulk operations**:
```bash
req review complete SYS-001 SYS-002 SYS-003
# Complete multiple reviews at once
```

### Integrations

**Slack/Teams notifications**:
```
@alice Your review is needed: SYS-001 (parent changed)
```

**Dashboard UI**:
Web dashboard showing review status, pending items, team metrics.

**Approval workflows**:
Multi-step approval for critical requirements.

## Implementation Status

**Current status**: **Partially implemented**

**Available now**:
- ✅ `req suspect` - Detect fingerprint mismatches
- ✅ `req accept <CHILD> <PARENT>` - Accept individual suspect links
- ✅ `req accept --all` - Accept all suspect links
- ✅ CI/CD integration via exit codes

**Planned for future releases**:
- ⏳ Review state tracking (under review, reviewed, approved)
- ⏳ Review assignments and notifications
- ⏳ Status reports and dashboards
- ⏳ Review history and audit logs
- ⏳ Multi-approver workflows

See [GitHub repository](https://github.com/danieleades/requirements-manager) for updates.

## Contributing

Interested in helping implement advanced review workflow features? See the project repository for contribution guidelines.

## Summary

**Implemented features**:

- Automatic suspect link detection (`req suspect`)
- Accepting suspect links after review (`req accept`)
- CI/CD integration with proper exit codes
- Batch operations (`req accept --all`)

**Use these commands now**:
```bash
req suspect              # List suspect links
req accept SYS-001 USR-001  # Accept single link
req accept --all         # Accept all suspect links
```

**Planned advanced features**:

- Review state tracking (suspect → under review → reviewed → approved)
- Commands: `req review start/complete`, `req status`, `req report`
- Review assignments and team workflows
- Notifications and integrations
- Comprehensive audit reports

**Current best practices**:

- Use `req suspect` regularly to check for changes
- Review requirements manually before accepting
- Use `req accept` to update fingerprints after review
- Integrate into CI/CD pipelines for automated checks
- Document reviews in git commit messages

## Next Steps

- Use [Fingerprints](./fingerprints.md) for manual change detection
- Implement [workarounds](#workarounds-until-implemented) for your workflow
- Watch GitHub repository for implementation updates
