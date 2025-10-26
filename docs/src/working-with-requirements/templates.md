# Templates

Templates provide default content when creating new requirements, helping maintain consistency across your requirements documentation.

## Overview

When you create a new requirement using `req add <KIND>`, the tool looks for a template file in the `.req/templates/` directory. If found, the template content is used as the initial content for the new requirement.

## Template Location

Templates are stored as markdown files in the `.req/templates/` directory:

```
your-requirements/
├── .req/
│   └── templates/
│       ├── USR.md          # Template for USR requirements
│       ├── SYS.md          # Template for SYS requirements
│       └── AUTH-USR.md     # Template for AUTH-USR requirements
├── config.toml
├── USR-001.md
└── SYS-001.md
```

## Template Matching

When creating a requirement, templates are matched in order of specificity:

1. **Full prefix match**: If creating `AUTH-USR-001`, looks for `.req/templates/AUTH-USR.md`
2. **KIND-only match**: If not found, looks for `.req/templates/USR.md`
3. **No template**: If neither exists, creates empty content

This allows you to:
- Define general templates for kinds (e.g., `USR.md` for all user requirements)
- Override with namespace-specific templates (e.g., `AUTH-USR.md` for auth user requirements)

## Creating Templates

### Simple Text Template

The simplest template is just plain text:

```markdown
[Describe the requirement here]
```

Save this as `.req/templates/USR.md`.

### Structured Template

For more structure, use markdown headers and sections:

```markdown
# Requirement Title

## Description

[Detailed description]

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2

## Notes

[Additional notes]
```

### Namespace-Specific Template

Create specialized templates for namespaced requirements:

```markdown
# Authentication Requirement

## Security Considerations

[Describe security implications]

## Implementation

[Authentication mechanism details]

## Test Strategy

[How to verify this auth requirement]
```

Save this as `.req/templates/AUTH-USR.md`.

## Using Templates

### Automatic Template Application

When you create a requirement without content flags, the template is automatically used:

```bash
req add USR
# Creates USR-001.md with content from .req/templates/USR.md
```

### Overriding Templates

Templates are ignored when you provide content via CLI flags:

```bash
# Template ignored - uses title and body from flags
req add USR -t "Custom Title" -b "Custom content"

# Template ignored - uses title from flag
req add USR -t "Custom Title"

# Template ignored - uses body from flag
req add USR -b "Custom content"
```

## Examples

### Software Project Templates

**`.req/templates/USR.md`**:
```markdown
[Describe the user-facing requirement]

**User Value**: [Why does the user need this?]

**Acceptance Criteria**:
- [ ] Criterion 1
```

**`.req/templates/SYS.md`**:
```markdown
[Describe the system-level implementation requirement]

**Technical Notes**: [Implementation details]

**Testing**: [How to verify]
```

### Aerospace Project Templates

**`.req/templates/URQT.md`** (User Requirements):
```markdown
## User Requirement

**Description**: [User requirement description]

**Rationale**: [Justification]

**Verification Method**: [Test, Analysis, Inspection, or Demonstration]
```

**`.req/templates/SRQT.md`** (Software Requirements):
```markdown
## Software Requirement

**Specification**: [Detailed technical specification]

**Derived From**: [Parent URQT reference]

**Verification Method**: [Test method]

**Criticality**: [DAL level]
```

## Best Practices

### Keep Templates Simple

Templates should provide structure without being overly prescriptive:

```markdown
✓ Good: [Describe the requirement]

✗ Too prescriptive: The system SHALL [verb] [object] [condition]
```

### Use Placeholders

Use square brackets for clear placeholders:

```markdown
[Describe the requirement here]

**Rationale**: [Why is this needed?]
```

### Team Conventions

Document your team's template conventions:

```markdown
## Description
[One paragraph summary]

## Details
[Detailed specification with sub-sections as needed]

## Testing
[How to verify this requirement]
```

### Version Control

Commit templates to version control alongside your requirements:

```bash
git add .req/templates/
git commit -m "Add requirement templates"
```

## Updating Templates

Templates only affect *new* requirements. To update existing requirements:

1. Update the template file
2. Manually edit existing requirements to match (if desired)
3. Or use the template as a guide for new requirements only

## Common Patterns

### Minimal Templates

For projects that prefer freeform text:

```markdown
[Requirement description]
```

### Structured Templates

For projects requiring specific sections:

```markdown
## Specification
[Technical details]

## Rationale
[Why this is needed]

## Verification
[How to test]
```

### Compliance Templates

For regulated industries:

```markdown
## Requirement
[Requirement text]

## Compliance
**Standard**: [e.g., DO-178C]
**Criticality**: [e.g., DAL A]

## Verification
**Method**: [Test, Analysis, Inspection, Demonstration]
**Criteria**: [Pass/fail criteria]
```

## Troubleshooting

### Template Not Being Used

**Problem**: Created requirement is empty even though template exists.

**Check**:
1. Template file is in `.req/templates/` directory
2. Template filename matches KIND exactly (case-sensitive)
3. Template file has `.md` extension
4. You didn't use `-t` or `-b` flags (which override templates)

### Wrong Template Being Used

**Problem**: Expected namespace-specific template but got general template.

**Reason**: Namespace-specific template file doesn't exist.

**Solution**: Create `.req/templates/<NAMESPACE>-<KIND>.md`

Example: For `AUTH-USR-001`, create `.req/templates/AUTH-USR.md`

## Real-World Example

Want to see templates in action? Check out the templates used by the Requiem project itself:

- [USR Template](../requirements/.req/templates/USR.md) - User requirements template with Statement, Rationale, and Acceptance Criteria sections
- [SYS Template](../requirements/.req/templates/SYS.md) - System requirements template with Statement, Implementation Notes, and Verification sections

These templates are used to create all requirements in the [Example Project](../requirements.md), demonstrating professional structure and best practices.
