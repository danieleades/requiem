# Traceability

Traceability is the cornerstone of requirements management. It's the ability to track relationships between requirements and other artifacts throughout the development lifecycle.

## Why Traceability Matters

Traceability enables:

1. **Impact Analysis** - If requirement X changes, what else is affected?
2. **Coverage Analysis** - Are all requirements implemented? Tested?
3. **Compliance** - Prove that every requirement has been satisfied
4. **Root Cause Analysis** - Trace bugs back to requirements
5. **Change Management** - Understand dependencies before making changes

## Types of Traceability

### Vertical Traceability

Tracks relationships **between levels** of requirements:

```
Stakeholder Needs
       ↓
User Requirements (USR)
       ↓
System Requirements (SYS)
       ↓
Software Requirements (SWR)
```

Example:
- Stakeholder: "We need to reduce data entry errors"
- USR-042: "Users shall receive validation feedback"
- SYS-078: "Form inputs shall validate on blur events"
- SWR-123: "Use the validator.js library for email validation"

### Horizontal Traceability

Tracks relationships **within the same level**:

```
USR-001 ← USR-005
USR-001 ← USR-012
```

Example: USR-005 and USR-012 might both depend on USR-001's authentication requirement.

### Forward Traceability

From requirements **downstream** to:
- Design documents
- Source code
- Test cases
- User documentation

Example: USR-001 → SYS-042 → test_authentication.rs

### Backward Traceability

From implementation artifacts **upstream** to requirements:

```
test_login.py → SYS-042 → USR-001
```

This answers: "Why does this test exist?" or "Which requirement does this code satisfy?"

## Traceability in Requiem

### Parent-Child Links

Requiem implements vertical traceability through explicit parent-child relationships:

```markdown
---
_version: '1'
uuid: ccdbddbe-d5d2-4656-b4fe-85e61c02cf63
created: 2025-07-22T13:15:27.996136510Z
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
---

The system shall validate user credentials using bcrypt hashing.
```

This SYS requirement explicitly traces to its parent USR-001.

### Stable Identifiers (UUIDs)

Each requirement has a UUID that never changes, even if the requirement is renumbered or renamed. This enables reliable traceability over time.

```yaml
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a  # Stable forever
hrid: USR-001                                # Might change
```

### Human-Readable IDs (HRIDs)

While UUIDs are stable, HRIDs like `USR-001` make traceability human-friendly:

- Easy to reference in conversations: "Did you implement USR-042?"
- Clear in code comments: `// Satisfies SYS-078`
- Readable in reports: "Coverage: 45 of 52 SYS requirements implemented"

### Multiple Parents

Unlike many tools, Requiem supports multiple parent requirements:

```yaml
parents:
- uuid: <uuid-1>
  hrid: USR-001
- uuid: <uuid-2>
  hrid: USR-003
- uuid: <uuid-3>
  hrid: USR-007
```

This reflects reality: a single implementation often satisfies multiple needs.

Example: A logging system might satisfy requirements for debugging, auditing, and compliance - three different parent needs.

## Traceability Beyond Requiem

Requiem manages requirement-to-requirement traceability. For complete lifecycle traceability, you need:

### Requirement → Code

**Manual approach:** Add requirement IDs in comments:

```rust
// Satisfies: SYS-042, SYS-043
fn validate_email(email: &str) -> Result<(), ValidationError> {
    // ...
}
```

**Automated approach:** Use code analysis tools to extract these tags and build traceability matrices.

### Requirement → Tests

**Manual approach:** Reference requirements in test names or docstrings:

```rust
#[test]
fn test_usr_042_email_validation() {
    // Verifies USR-042: Email validation feedback
}
```

**Automated approach:** Tools can parse test names and generate coverage reports.

### Requirement → Documentation

When using Requiem with MdBook or Sphinx, you can embed requirements directly in user documentation:

```markdown
## Plain Text Storage

{{#include ../requirements/USR-001.md}}

Requirements are stored as simple markdown files.
```

This ensures documentation stays synchronized with requirements.

## Traceability Challenges

### Maintaining Links

Manual traceability is prone to drift:
- Code changes but requirement IDs in comments aren't updated
- Tests are renamed and traceability is lost

**Mitigation:** Automate checks in CI. Use tools to validate that referenced requirement IDs exist.

### Granularity

How fine-grained should traceability be?

- Too coarse: "This module satisfies USR-001 through USR-050" (not helpful)
- Too fine: "This line satisfies requirement X clause 2.3.1.4" (maintenance nightmare)

**Balance:** Trace at the function/class level for code, at the test case level for tests.

### Many-to-Many Relationships

Real systems have complex relationships:
- One requirement satisfied by multiple components
- One component satisfying multiple requirements

Requiem's multiple-parent support helps, but complete traceability requires additional tooling.

## Traceability Reports

*Note: Automated report generation is planned but not yet implemented in Requiem.*

Common traceability reports include:

### Coverage Matrix

| Requirement | Designed | Implemented | Tested | Status |
|-------------|----------|-------------|---------|---------|
| USR-001 | ✓ | ✓ | ✓ | Complete |
| USR-002 | ✓ | ✓ | ✗ | Missing Tests |
| USR-003 | ✓ | ✗ | ✗ | Not Implemented |

### Dependency Graph

Visual representation of requirement relationships, showing the complete hierarchy from stakeholder needs down to implementation.

### Impact Report

Given a changed requirement, list all downstream requirements, design documents, code, and tests that might be affected.

## Best Practices

1. **Link early** - Establish traceability when creating requirements, not as an afterthought
2. **Use consistent formats** - Standardize how you reference requirements in code and tests
3. **Automate verification** - Add CI checks that validate traceability links
4. **Review regularly** - Periodically audit traceability to catch drift
5. **Keep it simple** - Traceability is valuable only if maintained; avoid overly complex schemes

## Next Steps

Understanding traceability enables effective change management. Continue to [Change Management](./change-management.md) to learn how Requiem helps you manage evolving requirements.
