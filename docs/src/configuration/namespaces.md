# Namespaces

Namespaces extend HRIDs with prefixes to organize requirements in large, multi-component projects. This chapter explains when and how to use them.

## What Are Namespaces?

A namespace is an optional prefix in an HRID that provides additional context:

```
USR-001              # No namespace
AUTH-USR-001         # One namespace level: AUTH
AUTH-LOGIN-USR-001   # Two namespace levels: AUTH, LOGIN
A-B-C-USR-001        # Three namespace levels: A, B, C
```

### HRID Format with Namespaces

Complete format:
```
{NAMESPACE-}*{KIND}-{ID}
```

Where:
- `NAMESPACE`: Zero or more namespace segments
- `KIND`: Requirement kind (e.g., USR, SYS)
- `ID`: Numeric identifier

Examples:
```
USR-001                    # KIND=USR, ID=001, namespace=[]
COMPONENT-USR-001          # KIND=USR, ID=001, namespace=[COMPONENT]
AUTH-LOGIN-SYS-042         # KIND=SYS, ID=042, namespace=[AUTH, LOGIN]
```

## When to Use Namespaces

### Large Projects

Projects with many requirements benefit from namespacing:

```
# Without namespaces (confusion)
USR-001  # Which component?
USR-002  # Auth or payment?
USR-003  # Mobile or web?

# With namespaces (clarity)
AUTH-USR-001    # Authentication user requirement
PAY-USR-002     # Payment user requirement
MOBILE-USR-003  # Mobile app user requirement
```

### Multi-Component Systems

Systems with distinct components or subsystems:

```
FRONTEND-USR-001
FRONTEND-SYS-001

BACKEND-USR-001
BACKEND-SYS-001

DATABASE-SYS-001
CACHE-SYS-001
```

### Product Families

Related products sharing requirements:

```
CORE-USR-001      # Shared across all products
CORE-SYS-001

MOBILE-USR-001    # Mobile-specific
MOBILE-SYS-001

WEB-USR-001       # Web-specific
WEB-SYS-001
```

### Team Boundaries

Map namespaces to team ownership:

```
TEAM-ALPHA-USR-001    # Team Alpha's requirements
TEAM-BETA-USR-001     # Team Beta's requirements
SHARED-USR-001        # Cross-team requirements
```

### Acquisitions and Mergers

Integrate acquired codebases without renumbering:

```
LEGACY-USR-001    # From acquired company (existing numbering)
NEW-USR-001       # Newly created requirements
```

## When NOT to Use Namespaces

### Small Projects

Projects with < 100 requirements rarely need namespaces:

```
# Simple project (no namespace needed)
USR-001
USR-002
SYS-001
SYS-002
```

Namespaces add complexity without benefit for small projects.

### Single Component

If your project is a single cohesive unit:

```
# Single-component app
USR-001
SYS-001
TST-001
```

No need for namespaces unless planning for future growth.

### Shallow Hierarchy

If requirements naturally form a flat or shallow structure, namespaces may be overkill.

## Namespace Strategies

### Strategy 1: By Component

Organize by system architecture:

```
AUTH-USR-001       # Authentication component
AUTH-SYS-001

API-USR-001        # API gateway
API-SYS-001

DB-SYS-001         # Database layer

CACHE-SYS-001      # Cache layer
```

**Advantages**:
- Mirrors system architecture
- Clear component ownership
- Natural for microservices

**When to use**: Systems with well-defined component boundaries.

### Strategy 2: By Layer

Organize by architectural layer:

```
UI-USR-001         # User interface layer
UI-SYS-001

BL-SYS-001         # Business logic layer

DATA-SYS-001       # Data layer
```

**Advantages**:
- Aligns with layered architectures
- Clear layer separation

**When to use**: Traditional n-tier applications.

### Strategy 3: By Feature

Organize by product feature:

```
LOGIN-USR-001      # Login feature
LOGIN-SYS-001
LOGIN-TST-001

PAYMENT-USR-001    # Payment feature
PAYMENT-SYS-001
PAYMENT-TST-001

REPORTING-USR-001  # Reporting feature
REPORTING-SYS-001
```

**Advantages**:
- Groups related requirements
- Feature-based development
- Easy feature scoping

**When to use**: Feature-driven development, agile teams.

### Strategy 4: Hybrid

Combine multiple levels:

```
MOBILE-AUTH-USR-001    # Mobile app, auth feature, user requirement
MOBILE-PAY-SYS-001     # Mobile app, payment feature, system requirement

WEB-AUTH-USR-001       # Web app, auth feature, user requirement
WEB-PAY-SYS-001        # Web app, payment feature, system requirement
```

**Advantages**:
- Maximum organization
- Clear context at a glance

**When to use**: Very large, multi-product systems.

**Caution**: Deep hierarchies can make HRIDs verbose. Balance clarity with brevity.

## Namespace Depth

### Single Level

```
COMPONENT-USR-001
```

**Typical**: 5-10 namespaces per project

**Best for**: Medium projects with distinct components

### Two Levels

```
PRODUCT-COMPONENT-USR-001
```

**Typical**: 3-5 top-level namespaces, 3-7 sub-namespaces each

**Best for**: Large projects, product families

### Three+ Levels

```
ORG-PRODUCT-COMPONENT-USR-001
```

**Typical**: Very large enterprises

**Best for**: Massive systems, multi-organization projects

**Caution**: HRIDs become long and unwieldy. Consider if truly necessary.

## Namespace Naming Conventions

### Use Descriptive Names

```
# Good
AUTH-USR-001      # Clear: authentication
PAYMENT-SYS-001   # Clear: payment processing

# Less clear
A-USR-001         # What is "A"?
MOD1-SYS-001      # What is "MOD1"?
```

### Keep Short

```
# Good
AUTH-USR-001      # 4 characters
PAY-USR-001       # 3 characters

# Too long
AUTHENTICATION-USR-001    # 14 characters
PAYMENT-PROCESSING-USR-001  # 18 characters
```

Aim for 3-8 characters per namespace segment.

### Use Consistent Casing

```
# Good (all uppercase)
AUTH-USR-001
PAYMENT-SYS-001

# Inconsistent (avoid)
auth-USR-001
Payment-SYS-001
```

Convention: Use uppercase to match KIND and ID conventions.

### Avoid Special Characters

```
# Good
AUTH-USR-001
PAYMENT-SYS-001

# Bad (don't use)
AUTH_USR-001      # Underscore confuses parsing
AUTH.USR.001      # Periods not supported
AUTH/USR/001      # Slashes not supported
```

Only use hyphens as separators.

## Implementation

### Creating Namespaced Requirements

Use the namespace in the KIND argument:

```bash
# Single namespace
req add AUTH-USR

# Output: Added requirement AUTH-USR-001
```

```bash
# Two namespace levels
req add MOBILE-AUTH-SYS

# Output: Added requirement MOBILE-AUTH-SYS-001
```

### Linking Namespaced Requirements

Linking works identically:

```bash
req link MOBILE-AUTH-SYS-001 AUTH-USR-001
```

Namespaces don't affect link functionality.

### File Naming

Filenames match HRIDs exactly:

```
requirements/
├── AUTH-USR-001.md
├── AUTH-SYS-001.md
├── PAYMENT-USR-001.md
└── MOBILE-AUTH-USR-001.md
```

## Namespace Organization with Directories

Combine namespaces with directory structure:

### Option 1: Match Namespace to Directory

```
requirements/
├── auth/
│   ├── AUTH-USR-001.md
│   └── AUTH-SYS-001.md
└── payment/
    ├── PAYMENT-USR-001.md
    └── PAYMENT-SYS-001.md
```

**Advantages**:
- Reinforces namespace structure
- Easy to navigate

**Disadvantage**:
- Redundancy (namespace in filename and path)

### Option 2: Namespace Without Matching Directory

```
requirements/
├── AUTH-USR-001.md
├── AUTH-SYS-001.md
├── PAYMENT-USR-001.md
└── PAYMENT-SYS-001.md
```

**Advantages**:
- Simpler structure
- No redundancy

**Disadvantage**:
- Large directories if many components

### Option 3: Hybrid

```
requirements/
├── mobile/
│   ├── MOBILE-AUTH-USR-001.md
│   └── MOBILE-PAY-USR-001.md
└── web/
    ├── WEB-AUTH-USR-001.md
    └── WEB-PAY-USR-001.md
```

**Advantages**:
- Top level organized by directory
- Sub-level by namespace

## Configuration

### Restricting Namespaces

Currently, Requiem doesn't restrict namespaces via configuration. Any valid namespace is accepted.

**Workaround**: Use `allowed_kinds` to restrict full namespaced kinds:

```toml
_version = "1"
allowed_kinds = [
    "AUTH-USR",
    "AUTH-SYS",
    "PAYMENT-USR",
    "PAYMENT-SYS",
]
```

This allows:
- `AUTH-USR-001` ✓
- `PAYMENT-SYS-001` ✓

This blocks:
- `INVALID-USR-001` ✗
- `USR-001` ✗ (no namespace)

**Limitation**: Doesn't enforce namespace structure for multi-level namespaces.

### Future Configuration

Planned configuration options (not yet implemented):

```toml
# Planned (not implemented)
[namespaces]
required = true          # Require all HRIDs to have namespaces
allowed = ["AUTH", "PAYMENT", "REPORTING"]
separator = "-"          # Customize separator (default: "-")
max_depth = 2            # Limit namespace depth
```

## Migration to Namespaces

### Adding Namespaces to Existing Project

**Challenge**: Existing requirements lack namespaces:
```
USR-001.md
USR-002.md
SYS-001.md
```

**Goal**: Add namespaces:
```
AUTH-USR-001.md
PAYMENT-USR-002.md
AUTH-SYS-001.md
```

**Steps**:

1. **Plan namespace scheme**: Decide which requirements belong to which namespace

2. **Rename files**:
```bash
mv USR-001.md AUTH-USR-001.md
mv USR-002.md PAYMENT-USR-002.md
mv SYS-001.md AUTH-SYS-001.md
```

3. **Update HRIDs in frontmatter**: Edit each file to update the HRID references

4. **Fix parent links**: Run `req clean` to correct parent HRIDs
```bash
req clean
```

5. **Verify**:
```bash
req clean  # Should succeed with no errors
```

6. **Commit**:
```bash
git add -A
git commit -m "Add namespaces to requirements"
```

**Note**: UUIDs remain unchanged, so relationships are preserved.

## Examples

### Example 1: E-commerce Platform

```
# Customer-facing requirements
CUSTOMER-USR-001.md  # Customer account management
CUSTOMER-USR-002.md  # Product browsing

# Cart and checkout
CART-USR-001.md      # Shopping cart
CHECKOUT-USR-001.md  # Checkout process

# Payment processing
PAYMENT-SYS-001.md   # Payment gateway integration
PAYMENT-SYS-002.md   # PCI compliance

# Order management
ORDER-SYS-001.md     # Order processing
ORDER-SYS-002.md     # Order fulfillment

# Inventory
INVENTORY-SYS-001.md # Stock management
```

### Example 2: Aerospace System

```
# Aircraft levels (DO-178C)
AIRCRAFT-URQT-001.md   # Aircraft-level user requirement

# System level
AVIONICS-SRQT-001.md   # Avionics system requirement
ENGINES-SRQT-001.md    # Engine system requirement

# Software level
AVIONICS-FCS-SWRQT-001.md   # Flight control software
AVIONICS-NAV-SWRQT-001.md   # Navigation software

# Hardware level
AVIONICS-FCS-HWRQT-001.md   # Flight control hardware
```

### Example 3: Mobile and Web Apps

```
# Shared requirements
CORE-USR-001.md         # Cross-platform user requirement
CORE-SYS-001.md         # Backend API

# Mobile-specific
MOBILE-IOS-USR-001.md   # iOS-specific requirement
MOBILE-ANDROID-USR-001.md  # Android-specific

# Web-specific
WEB-USR-001.md          # Web app requirement
WEB-ADMIN-USR-001.md    # Admin panel requirement
```

## Best Practices

### 1. Design Namespace Scheme Early

Define your namespace strategy before creating many requirements:

```toml
# Document in config.toml comments
_version = "1"

# Namespace scheme:
# - AUTH: Authentication and authorization
# - PAYMENT: Payment processing
# - REPORTING: Report generation
# - ADMIN: Administration features

allowed_kinds = ["AUTH-USR", "AUTH-SYS", "PAYMENT-USR", "PAYMENT-SYS"]
```

### 2. Keep Namespaces Shallow

Prefer 1-2 levels over 3+:

```
# Good
AUTH-USR-001

# Still good
MOBILE-AUTH-USR-001

# Getting long
COMPANY-PRODUCT-MOBILE-AUTH-USR-001
```

### 3. Document Namespace Meanings

Create a README or documentation page:

```markdown
# Namespace Guide

- **AUTH**: Authentication and authorization
- **PAYMENT**: Payment processing
- **REPORT**: Report generation
- **ADMIN**: Administration and configuration
```

### 4. Consistent Abbreviations

Use consistent abbreviations across namespaces:

```
# Good (consistent)
AUTH-USR-001
AUTH-SYS-001
PAYMENT-USR-001
PAYMENT-SYS-001

# Inconsistent (avoid)
AUTH-USR-001
AUTHENTICATION-SYS-001
PAY-USR-001
PAYMENT_PROCESSING-SYS-001
```

### 5. Consider Future Growth

Choose namespaces that won't become obsolete:

```
# Flexible
MOBILE-USR-001       # Can add MOBILE-IOS, MOBILE-ANDROID later

# Inflexible
IPHONE-USR-001       # What about iPads? Android?
```

## Summary

**Key Takeaways**:

1. **Optional**: Namespaces are optional; use when beneficial
2. **Hierarchical**: Support multiple levels (1-3 typical)
3. **Flexible**: Choose strategy that fits your project
4. **Scalable**: Essential for large, multi-component systems
5. **Format**: `{NAMESPACE-}*{KIND}-{ID}`

**When to use**:
- Large projects (> 100 requirements)
- Multi-component systems
- Product families
- Team boundaries
- Acquisitions

**When NOT to use**:
- Small projects (< 100 requirements)
- Single-component systems
- When simplicity is priority

**Best practices**:
- Design namespace scheme early
- Keep shallow (1-2 levels)
- Use descriptive, short names
- Document namespace meanings
- Consider future growth

## Next Steps

- Review [Directory Structure](./directory-structure.md) for organizing namespaced requirements
- Learn about [Creating Requirements](../working-with-requirements/creating.md) with namespaces
