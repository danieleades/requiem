# Human-Readable IDs (HRIDs)

Human-Readable IDs (HRIDs) are short, memorable identifiers like `USR-001` or `COMPONENT-SYS-042`. They make requirements easy to reference in conversations, code comments, and documentation.

## HRID Format

The general format is:

```
{NAMESPACE*}-{KIND}-{ID}
```

Where:
- `NAMESPACE*` = Zero or more namespace segments (optional)
- `KIND` = Requirement type/category
- `ID` = Numeric identifier

### Components

#### Namespace (Optional)

Zero or more segments separated by hyphens:

```
USR-001                          # No namespace
COMPONENT-USR-001                # One namespace level
AUTH-LOGIN-SYS-042               # Two namespace levels
PRODUCT-FEATURE-MODULE-SWR-123   # Three namespace levels
```

**Purpose**: Organize requirements in large projects or multi-component systems.

**Rules**:
- Each segment must be non-empty
- Segments are case-sensitive (by convention, use UPPERCASE)
- No special characters (letters, numbers, underscores OK)

#### Kind (Required)

The requirement category or type:

```
USR    # User requirements
SYS    # System requirements
SWR    # Software requirements
HWR    # Hardware requirements
TST    # Test requirements
```

**Purpose**: Distinguish requirement levels in the hierarchy.

**Rules**:
- Must be non-empty
- Case-sensitive (by convention, use UPPERCASE)
- No special characters

**Common conventions**:
- `USR` - User requirements (what users need)
- `SYS` - System requirements (technical specifications)
- `SWR` - Software requirements (software-specific details)
- `HWR` - Hardware requirements (hardware-specific details)
- `TST` - Test requirements (test cases)

You're free to use any KIND values that make sense for your project.

#### ID (Required)

A positive integer:

```
USR-1      # Valid
USR-001    # Valid (zero-padded)
USR-42     # Valid
USR-1000   # Valid
```

**Purpose**: Unique number within the KIND (or NAMESPACE-KIND combination).

**Rules**:
- Must be a positive integer (1, 2, 3, ...)
- Zero is not allowed
- Negative numbers are not allowed

**Display format**: IDs are zero-padded to a configurable width (default 3 digits):

```
USR-001   # 3 digits (default)
USR-042
USR-123
USR-1000  # Exceeds padding, shows all digits
```

Configure padding in `config.toml`:

```toml
digits = 3   # USR-001
# or
digits = 4   # USR-0001
```

## Parsing Examples

### Valid HRIDs

```
USR-001                      # Simple: KIND-ID
SYS-042                      # Different kind
USR-1                        # No leading zeros
USR-1000                     # Large ID
COMPONENT-USR-001            # One namespace
AUTH-LOGIN-SYS-042           # Two namespaces
A-B-C-D-E-KIND-123           # Five namespaces
```

### Invalid HRIDs

```
USR001                       # Missing hyphen
-USR-001                     # Leading hyphen
USR-001-                     # Trailing hyphen
USR--001                     # Double hyphen
USR-                         # Missing ID
-001                         # Missing KIND
USR-abc                      # Non-numeric ID
USR-0                        # Zero ID not allowed
USR--001                     # Empty segment
```

## Namespaces in Practice

### When to Use Namespaces

**Large projects**: Organize by subsystem

```
AUTH-USR-001     # Authentication subsystem user requirements
AUTH-SYS-001
PAYMENT-USR-001  # Payment subsystem user requirements
PAYMENT-SYS-001
```

**Product families**: Distinguish shared vs. product-specific

```
CORE-SYS-001    # Shared by all products
MOBILE-USR-001  # Mobile app specific
WEB-USR-001     # Web app specific
```

**Organizational structure**: Match company divisions

```
FRONTEND-DASHBOARD-USR-001
FRONTEND-REPORTS-USR-001
BACKEND-API-SYS-001
BACKEND-DATABASE-SYS-001
```

### When Not to Use Namespaces

**Small projects**: Added complexity without benefit

```
# Overkill for a small project
TODO-APP-CORE-STORAGE-SYS-001

# Better
SYS-001
```

**Flat hierarchy**: If you don't need organizational structure

```
USR-001, USR-002, SYS-001, SYS-002
```

Use namespaces only when they add clarity, not as a default.

## HRID Assignment

### Auto-Incrementing

`req add` automatically assigns the next available ID:

```bash
req add USR    # Creates USR-001
req add USR    # Creates USR-002
req add SYS    # Creates SYS-001
req add USR    # Creates USR-003
```

The next ID is determined by examining existing requirements and incrementing the highest ID found.

### With Namespaces

Specify the namespace in the KIND argument:

```bash
req add AUTH-USR        # Creates AUTH-USR-001
req add AUTH-SYS        # Creates AUTH-SYS-001
req add PAYMENT-USR     # Creates PAYMENT-USR-001
req add AUTH-USR        # Creates AUTH-USR-002
```

Each NAMESPACE-KIND combination has its own sequence.

### Gaps in Numbering

If you delete `USR-002`, creating a gap:

```
USR-001.md  # Exists
# USR-002.md deleted
USR-003.md  # Exists
```

The next `req add USR` creates `USR-004`, not `USR-002`. Requiem always uses the next number after the highest existing ID.

**Rationale**: Reusing deleted IDs could confuse people referring to old documentation or Git history that mentioned USR-002.

## HRID Best Practices

### Use Consistent Naming

Pick a convention and stick to it:

```
# Good (consistent)
USR-001, USR-002, SYS-001, SYS-002

# Bad (inconsistent)
USR-001, User-002, SYS-001, system-002
```

### Keep KINDs Short

```
# Good
USR, SYS, SWR, TST

# Less good (too verbose)
USER_REQUIREMENTS, SYSTEM_REQUIREMENTS
```

Short KINDs are easier to type and read.

### Use UPPERCASE

```
# Good
USR-001

# Works but unconventional
usr-001
Usr-001
```

UPPERCASE is the standard convention in requirements engineering.

### Document Your KIND Meanings

Create a glossary:

```markdown
# Requirements Glossary

- **USR**: User requirements (stakeholder needs)
- **SYS**: System requirements (technical specifications)
- **SWR**: Software requirements (implementation details)
- **TST**: Test requirements (test cases)
```

## HRID Stability

### HRIDs Can Change

Unlike UUIDs, HRIDs are **not guaranteed stable**. They might change if:

- Requirements are renumbered
- Namespaces are reorganized
- Projects are merged

### Correcting HRIDs

If you manually rename a requirement file, run:

```bash
req clean
```

This updates parent `hrid` fields to reflect the new names, while UUIDs keep links valid.

Example:

```bash
# Manually rename
mv USR-001.md AUTH-USR-001.md

# Update parent references
req clean

# Children that referenced USR-001 now show AUTH-USR-001
```

### Referencing Requirements

In documentation and code, always reference by HRID for readability:

```rust
// Satisfies USR-042: Email validation
fn validate_email(email: &str) -> Result<(), ValidationError> {
    // ...
}
```

But remember that HRIDs might change. For machine processing, use UUIDs.

## Namespace Configuration

Configure allowed namespaces and kinds in `config.toml`:

```toml
_version = "1"

# Optional: Restrict allowed kinds
allowed_kinds = ["USR", "SYS", "SWR", "TST"]

# Optional: Set ID padding
digits = 3
```

**Note**: Namespace validation is not yet implemented. You can use any namespace structure. Future versions may allow restricting namespaces.

## Filename Matching

Requirement filenames **must** match the HRID:

```
USR-001.md              # Matches USR-001
COMPONENT-SYS-042.md    # Matches COMPONENT-SYS-042
```

Mismatches cause errors:

```
USR-001.md containing HRID USR-002 → Error!
```

The HRID in the filename is authoritative. Requiem derives the HRID from the filename, not from any field in the YAML.

## Next Steps

Now that you understand HRIDs, learn how to [create requirements](./creating.md) with `req add`.
