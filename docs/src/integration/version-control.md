# Version Control Best Practices

Requiem's plain-text format makes requirements ideal for version control. This chapter covers Git workflows and best practices.

## Why Version Control Matters

Requirements benefit from version control:

- **Complete history**: Track all changes over time
- **Audit trail**: Know who changed what and when
- **Branching**: Develop requirements in parallel
- **Review**: Use pull requests for requirement reviews
- **Rollback**: Revert problematic changes
- **Tagging**: Mark requirement baselines (releases)

## Git Basics for Requirements

### Initial Setup

Create a Git repository for your requirements:

```bash
mkdir my-requirements
cd my-requirements
git init

# Create first requirement
req add USR
git add config.toml USR-001.md
git commit -m "Initial commit: add USR-001"
```

### Adding Requirements

When creating requirements:

```bash
# Create requirement
req add USR

# Stage and commit
git add USR-002.md
git commit -m "Add USR-002: user data export requirement"
```

### Editing Requirements

When modifying requirements:

```bash
# Edit requirement
vim USR-001.md

# Review changes
git diff USR-001.md

# Stage and commit
git add USR-001.md
git commit -m "Update USR-001: clarify email validation format"
```

### Linking Requirements

When linking requirements:

```bash
# Create link
req link SYS-001 USR-001

# Both files change (child gets parent reference)
git diff

# Commit both
git add SYS-001.md
git commit -m "Link SYS-001 to USR-001"
```

## Commit Message Best Practices

### Format

Use clear, descriptive commit messages:

**Bad**:
```bash
git commit -m "update"
git commit -m "fix typo"
git commit -m "changes"
```

**Good**:
```bash
git commit -m "Add USR-042: user data export requirement"
git commit -m "Update USR-001: change email validation to RFC 5322"
git commit -m "Link SYS-001 to USR-001 and USR-002"
```

### Structure

Use conventional commit format:

```
<type>: <HRID>: <description>

[optional body]

[optional footer]
```

**Types**:
- `add`: New requirement
- `update`: Modify existing requirement
- `link`: Create requirement link
- `remove`: Delete requirement
- `refactor`: Reorganize without changing content
- `docs`: Update documentation (non-requirement changes)

**Examples**:

```bash
# Add new requirement
git commit -m "add: USR-042: user data export"

# Update existing
git commit -m "update: USR-001: clarify email validation

Changed from 'valid email' to 'RFC 5322 compliant'
to remove ambiguity."

# Link requirements
git commit -m "link: SYS-001 -> USR-001, USR-002

SYS-001 satisfies both user requirements for authentication."

# Bulk operation
git commit -m "refactor: reorganize requirements into subdirectories

- Move USR-*.md to user/
- Move SYS-*.md to system/
- Update config.toml"
```

## Branching Strategies

### Feature Branches

Develop requirements for new features in branches:

```bash
# Create feature branch
git checkout -b feature/payment-system

# Add requirements
req add USR  # USR-042
req add SYS  # SYS-012
# Edit and link requirements

# Commit changes
git add .
git commit -m "add: payment system requirements (USR-042, SYS-012)"

# Push for review
git push origin feature/payment-system
```

### Requirement Change Branches

For significant requirement changes:

```bash
# Create change branch
git checkout -b change/update-usr-001

# Edit requirement
vim USR-001.md

# Update dependent requirements
req clean

# Commit
git add .
git commit -m "update: USR-001: clarify email validation

Updated email validation to reference RFC 5322.
Corrected parent HRIDs in dependent requirements."

# Create pull request
gh pr create
```

### Release Branches

Stabilize requirements for releases:

```bash
# Create release branch
git checkout -b release/v1.0

# Freeze requirements (no new additions)
# Allow only bug fixes and clarifications

# Tag when stable
git tag v1.0.0
git push origin v1.0.0
```

## Pull Request Workflows

### Creating Pull Requests

When changing requirements:

1. **Create branch**:
```bash
git checkout -b update-auth-requirements
```

2. **Make changes**:
```bash
vim USR-001.md
req clean
```

3. **Commit**:
```bash
git add -A
git commit -m "update: USR-001: strengthen password requirements"
```

4. **Push and create PR**:
```bash
git push origin update-auth-requirements
gh pr create --title "Update authentication requirements" \
             --body "Strengthens password requirements to meet new security policy"
```

### PR Description Template

Use a template for requirement PRs:

```markdown
## Summary
Adds/updates/removes requirements for [feature/change].

## Changed Requirements
- USR-001: [description of change]
- SYS-005: [description of change]

## Impact Analysis
- Affects: SYS-001, SYS-003, TST-001
- Reviewed: ✓ All dependent requirements checked

## Checklist
- [x] `req clean` passes
- [x] Commit messages follow convention
- [x] Dependent requirements reviewed
- [x] Tests updated (if applicable)
- [ ] Approved by: @stakeholder
```

### Reviewing Pull Requests

When reviewing requirement PRs:

1. **Check diffs carefully**:
```bash
# Review line-by-line changes
gh pr diff 123
```

2. **Verify UUIDs unchanged**:
```bash
# Ensure UUIDs haven't been modified
git diff main..HEAD -- '*.md' | grep 'uuid:'
```

3. **Check req clean passes**:
```bash
# Validate requirements
req clean
```

4. **Review dependent requirements**:
```bash
# Find affected requirements
grep -r "uuid: <changed-req-uuid>" *.md
```

5. **Approve and merge**:
```bash
gh pr review 123 --approve
gh pr merge 123
```

## Handling Conflicts

### Merge Conflicts in Requirements

Conflicts occur when two branches modify the same requirement.

**Example conflict**:

```markdown
<<<<<<< HEAD
The system shall validate emails using RFC 5321.
=======
The system shall validate emails using RFC 5322.
>>>>>>> feature/update-email-validation
```

**Resolution**:

1. **Understand both changes**: Read both versions.

2. **Choose or combine**: Decide which is correct or merge both:
```markdown
The system shall validate emails using RFC 5322.
```

3. **Mark resolved**:
```bash
git add USR-001.md
git commit
```

### Frontmatter Conflicts

**UUID conflicts** (should never happen):

```yaml
<<<<<<< HEAD
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
=======
uuid: 00000000-0000-0000-0000-000000000000
>>>>>>> other-branch
```

**Resolution**: Keep the original UUID (HEAD). Changing UUIDs breaks traceability.

**Parent conflicts** (both branches added parents):

```yaml
<<<<<<< HEAD
parents:
- uuid: aaaa...
  hrid: USR-001
=======
parents:
- uuid: bbbb...
  hrid: USR-002
>>>>>>> other-branch
```

**Resolution**: Merge both parents:
```yaml
parents:
- uuid: aaaa...
  hrid: USR-001
- uuid: bbbb...
  hrid: USR-002
```

Then run:
```bash
req clean  # Validate merged result
```

## Tagging and Releases

### Creating Baseline Tags

Tag stable requirement sets:

```bash
# Tag current state
git tag -a v1.0.0 -m "Release 1.0.0 requirements baseline"
git push origin v1.0.0
```

### Naming Conventions

Use semantic versioning:

- `v1.0.0` - Major release
- `v1.1.0` - Minor release (new requirements)
- `v1.0.1` - Patch release (clarifications, typo fixes)

Or use date-based tags:

- `baseline-2025-07-22`
- `release-2025-q3`

### Comparing Baselines

Compare requirement changes between releases:

```bash
# List changed requirements
git diff v1.0.0..v2.0.0 --name-only -- '*.md'

# Show detailed changes
git diff v1.0.0..v2.0.0 -- USR-001.md

# Generate changelog
git log v1.0.0..v2.0.0 --oneline -- '*.md'
```

## Advanced Git Techniques

### Git Blame for Requirements

See who last modified each line:

```bash
git blame USR-001.md
```

Output:
```
4bfeb7d5 (Alice  2025-07-20) The system shall validate
a1b2c3d4 (Bob    2025-07-22) user email addresses according
e5f6g7h8 (Alice  2025-07-23) to RFC 5322.
```

### Git Log for Requirement History

View complete history:

```bash
# All commits affecting USR-001
git log -p USR-001.md

# One-line summary
git log --oneline USR-001.md

# Show who, when, what
git log --format="%h %an %ad %s" --date=short -- USR-001.md
```

### Git Diff for Requirement Changes

Compare versions:

```bash
# Current vs. last commit
git diff HEAD~1 USR-001.md

# Current vs. specific commit
git diff a1b2c3d4 USR-001.md

# Between branches
git diff main..feature/update USR-001.md

# Ignore whitespace
git diff -w USR-001.md
```

### Bisect to Find Breaking Changes

Find when a requirement changed incorrectly:

```bash
git bisect start
git bisect bad HEAD
git bisect good v1.0.0

# Git checks out middle commit
req clean  # Test if requirements are valid
git bisect good  # or 'bad'

# Repeat until found
git bisect reset
```

## Ignoring Files

### .gitignore for Requirements Projects

Exclude generated or temporary files:

**.gitignore**:
```
# Requiem temp files
*.tmp

# Editor files
*.swp
*.swo
*~
.vscode/
.idea/

# OS files
.DS_Store
Thumbs.db

# Build outputs (if using MdBook/Sphinx)
book/
_build/
_generated/

# Python
__pycache__/
*.pyc
.venv/
```

**Don't ignore**:
- `config.toml` (Requiem configuration)
- `*.md` (requirements)

## CI/CD Integration

### GitHub Actions

Validate requirements automatically:

**.github/workflows/requirements.yml**:
```yaml
name: Requirements Validation

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Requiem
        run: cargo install requirements-manager

      - name: Validate requirements
        run: req clean
        working-directory: ./requirements

      - name: Check for uncommitted changes
        run: |
          if [ -n "$(git status --porcelain)" ]; then
            echo "Error: req clean modified files. Run req clean locally."
            git diff
            exit 1
          fi
```

### Pre-commit Hook

Validate before every commit:

**.git/hooks/pre-commit**:
```bash
#!/bin/bash

echo "Validating requirements..."
req clean

if [ $? -ne 0 ]; then
    echo "Error: Requirements validation failed"
    exit 1
fi

# Stage any changes made by req clean
git add -u

exit 0
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Collaboration Best Practices

### 1. Clear Ownership

Define requirement ownership:

```markdown
# CODEOWNERS
# Assign reviewers for requirement changes

requirements/USR-*.md @product-team
requirements/SYS-*.md @architecture-team
requirements/TST-*.md @qa-team
```

### 2. Require Reviews

Enforce PR reviews:

**GitHub branch protection**:
- Require pull request reviews before merging
- Require status checks (req clean) to pass
- Require up-to-date branches

### 3. Communication

Use commit messages and PR descriptions to communicate:

- **Why** the change was made
- **What** requirements are affected
- **Who** should review

### 4. Regular Syncs

Prevent divergence:

```bash
# Update from main frequently
git checkout feature/my-branch
git pull origin main
git merge main

# Resolve conflicts if any
req clean
```

### 5. Atomic Commits

One logical change per commit:

**Bad**:
```bash
# Single commit with unrelated changes
git commit -m "Add USR-042, update USR-001, fix typo in SYS-003"
```

**Good**:
```bash
# Separate commits
git commit -m "add: USR-042: user data export"
git commit -m "update: USR-001: clarify email validation"
git commit -m "fix: SYS-003: correct typo in authentication flow"
```

## Troubleshooting

### Large Diffs

**Problem**: Git diffs for requirement files are hard to read.

**Solution**: Use word-level diffs:
```bash
git diff --word-diff USR-001.md
```

Or color-words:
```bash
git diff --color-words USR-001.md
```

### Accidental UUID Changes

**Problem**: Someone accidentally changed a UUID.

**Detection**:
```bash
# Find UUID changes
git log -p --all -S'uuid:' -- USR-001.md
```

**Recovery**:
```bash
# Restore correct UUID from history
git show a1b2c3d4:USR-001.md | grep 'uuid:'
# Manually fix or revert
```

### Lost Requirements

**Problem**: Requirement file was deleted.

**Recovery**:
```bash
# Find when it was deleted
git log --all --full-history -- USR-099.md

# Restore from last good commit
git checkout a1b2c3d4 -- USR-099.md
```

## Summary

**Key Practices**:

1. **Commit frequently** with clear messages
2. **Use branches** for features and changes
3. **Create PRs** for reviews
4. **Validate** with `req clean` before committing
5. **Tag releases** for baselines
6. **Review carefully** to catch UUID changes
7. **Resolve conflicts** thoughtfully (never change UUIDs)
8. **Use CI/CD** for automated validation

**Benefits**:
- Complete audit trail
- Easy collaboration
- Reversible changes
- Formal review process
- Baseline management

**Common Pitfalls**:
- Changing UUIDs (breaks traceability)
- Poor commit messages (lost context)
- Ignoring conflicts (inconsistent requirements)
- Skipping validation (invalid requirements reach main)

## Next Steps

- Set up [CI/CD validation](#cicd-integration) for your requirements
- Create [commit message templates](#commit-message-best-practices)
- Configure [branch protection](#2-require-reviews)
- Review [Advanced Topics](../advanced.md) for more techniques
