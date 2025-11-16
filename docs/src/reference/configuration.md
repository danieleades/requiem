# Configuration Reference

Complete reference for Requiem's `config.toml` configuration file.

## Overview

The `config.toml` file configures Requiem's behavior. It must be located in the root of your requirements directory.

### File Location

```
requirements/
├── config.toml    ← Configuration file
├── USR-001.md
├── USR-002.md
└── SYS-001.md
```

### File Format

TOML (Tom's Obvious Minimal Language)

### Optional

Configuration file is optional. If absent, Requiem uses defaults.

## Schema Version 1

Current schema version: `1`

### Complete Example

```toml
_version = "1"
allowed_kinds = ["USR", "SYS", "SWR", "TST"]
digits = 3
allow_unrecognised = false
subfolders_are_namespaces = false
```

## Configuration Fields

### `_version` (required)

Schema version for forward/backward compatibility.

**Type**: String (quoted)

**Required**: Yes

**Default**: N/A (must be explicitly specified)

**Valid Values**: `"1"`

**Example**:
```toml
_version = "1"
```

**Purpose**:
- Enables future schema changes
- Allows old Requiem versions to detect incompatible configs
- Allows new Requiem versions to handle old configs

**Validation**:
- Must be present
- Must be quoted string
- Must be `"1"` in current version

**Errors**:

Missing:
```toml
# config.toml
allowed_kinds = ["USR"]
```
```
Error: Failed to parse config file: missing field '_version'
```

Wrong type:
```toml
_version = 1  # Wrong: integer instead of string
```
```
Error: Failed to parse config file: invalid type: integer, expected a string
```

### `allowed_kinds`

Restrict which requirement kinds are permitted.

**Type**: Array of strings

**Required**: No

**Default**: `[]` (empty array = all kinds allowed)

**Valid Values**: Array of non-empty strings

**Example**:
```toml
allowed_kinds = ["USR", "SYS", "SWR", "TST"]
```

**Purpose**:
- Enforce project conventions
- Prevent typos (USR vs UST)
- Document allowed requirement types

**Behavior**:

**Empty array (default)**:
```toml
allowed_kinds = []
# OR omit field entirely
```
All kinds accepted: `USR-001`, `SYS-001`, `CUSTOM-001`, etc.

**Non-empty array**:
```toml
allowed_kinds = ["USR", "SYS"]
```
Only listed kinds accepted:
- `req add USR` ✓ Succeeds
- `req add SYS` ✓ Succeeds
- `req add TST` ✗ Fails (TST not in allowed list)

**With namespaces**:
```toml
allowed_kinds = ["AUTH-USR", "AUTH-SYS", "PAYMENT-USR"]
```
Exact match required:
- `req add AUTH-USR` ✓ Succeeds
- `req add USR` ✗ Fails (USR not in allowed list)

**Examples**:

Aerospace project (DO-178C):
```toml
allowed_kinds = ["URQT", "SRQT", "SWRQT", "HWRQT", "TRQT"]
```

Software project:
```toml
allowed_kinds = ["USR", "SYS", "SWR", "TST", "DOC"]
```

Multi-component with namespaces:
```toml
allowed_kinds = [
    "AUTH-USR", "AUTH-SYS",
    "PAYMENT-USR", "PAYMENT-SYS",
    "REPORTING-USR", "REPORTING-SYS"
]
```

**Validation**:
- Each element must be non-empty string
- Duplicates allowed (but pointless)
- Case-sensitive matching

**Errors**:

Empty string in array:
```toml
allowed_kinds = ["USR", ""]
```
```
Error: Failed to parse config file: empty strings not allowed in allowed_kinds
```

### `digits`

Number of digits in HRID numbering (with zero-padding).

**Type**: Unsigned integer

**Required**: No

**Default**: `3`

**Valid Values**: Any positive integer (typically 3-5)

**Example**:
```toml
digits = 3
```

**Purpose**:
- Control HRID formatting
- Accommodate projects with many requirements per kind

**Behavior**:

**digits = 3** (default):
```
USR-001
USR-002
USR-010
USR-099
USR-100  # Expands beyond 3 digits when needed
USR-1000
```

**digits = 4**:
```
USR-0001
USR-0002
USR-0010
USR-0999
USR-1000
USR-10000  # Expands beyond 4 digits when needed
```

**digits = 2**:
```
USR-01
USR-02
USR-99
USR-100  # Expands beyond 2 digits when needed
```

**Parsing**:
- Requirements can have any number of digits when parsing
- `USR-1`, `USR-01`, `USR-001` all parse as ID 1
- Display/creation uses configured padding

**Recommendations**:
- `digits = 3`: < 1000 requirements per kind (most projects)
- `digits = 4`: 1000-9999 requirements per kind
- `digits = 5`: Very large projects

**Examples**:

Small project:
```toml
digits = 2
```

Medium project (default):
```toml
digits = 3
```

Large project:
```toml
digits = 4
```

**Validation**:
- Must be positive integer
- Zero or negative not allowed

**Errors**:

Zero:
```toml
digits = 0
```
```
Error: Failed to parse config file: digits must be positive
```

### `allow_unrecognised`

Allow markdown files that don't match HRID pattern.

**Type**: Boolean

**Required**: No

**Default**: `false`

**Valid Values**: `true`, `false`

**Example**:
```toml
allow_unrecognised = true
```

**Purpose**:
- Enable mixing requirements with other documentation
- Control strictness of file validation

**Behavior**:

**allow_unrecognised = false** (default, strict):
```
docs/
├── config.toml
├── USR-001.md     ← Loaded (valid HRID)
├── USR-002.md     ← Loaded (valid HRID)
└── README.md      ← ERROR: Not a valid HRID
```

Error during `req clean`:
```
Error: Unrecognised file: README.md
```

**allow_unrecognised = true** (permissive):
```
docs/
├── config.toml
├── USR-001.md     ← Loaded (valid HRID)
├── USR-002.md     ← Loaded (valid HRID)
└── README.md      ← Ignored (not a valid HRID)
```

No error; `README.md` is silently skipped.

**Use Cases**:

**Use `false` (default)**:
- Dedicated requirements directory
- Strict validation desired
- Catch typos in filenames

**Use `true`**:
- Requirements mixed with MdBook/Sphinx content
- Documentation and requirements in same directory
- Legacy projects with existing non-requirement files

**Examples**:

Strict requirements-only:
```toml
_version = "1"
allow_unrecognised = false
```

Mixed documentation:
```toml
_version = "1"
allow_unrecognised = true  # Allow chapter1.md, README.md, etc.
```

**Validation**:
- Must be boolean
- Case-sensitive: `true` or `false` (lowercase)

**Errors**:

Wrong case:
```toml
allow_unrecognised = True  # Wrong: uppercase
```
```
Error: Failed to parse config file: invalid value
```

### `subfolders_are_namespaces`

Control whether subfolder paths contribute to requirement namespaces.

**Type**: Boolean

**Required**: No

**Default**: `false`

**Valid Values**: `true`, `false`

**Example**:
```toml
subfolders_are_namespaces = true
```

**Purpose**:
- Choose between filename-based and path-based directory organization
- Enable cleaner filenames in hierarchical structures
- Align folder structure with namespace hierarchy

**Behavior**:

**subfolders_are_namespaces = false** (default, filename-based):
```
requirements/
├── custom/folder/
│   └── system-auth-REQ-001.md   → HRID: system-auth-REQ-001
└── any/path/
    └── payment-USR-002.md       → HRID: payment-USR-002
```

- **HRID**: Fully encoded in filename
- **Folders**: Purely organizational, don't affect HRID
- **Flexibility**: Move files freely without changing HRIDs

**subfolders_are_namespaces = true** (path-based):
```
requirements/
├── system/
│   └── auth/
│       ├── REQ/
│       │   └── 001.md           → HRID: system-auth-REQ-001 (from heading)
│       └── USR/
│           └── 002.md           → HRID: system-auth-USR-002 (from heading)
└── payment/
    └── USR/
        └── 003.md               → HRID: payment-USR-003 (from heading)
```

- **HRID Source**: Always from the markdown heading (e.g., `# system-auth-USR-002`)
- **Canonical Path**: `namespace/KIND/ID.md` for new files
- **Folder Structure**: Namespace segments → KIND directory → numeric filename
- **Loading**: Files can exist anywhere; HRID read from heading content
- **Saving New Files**: Written to canonical path structure
- **Existing Files**: Stay at their current location (no automatic migration)

**Use Cases**:

**Use `false` (default)**:
- Maximum folder flexibility
- Arbitrary organizational schemes
- Frequent folder reorganization
- Explicit namespaces in every filename

**Use `true`**:
- Hierarchical component structures
- Folder structure mirrors system architecture
- Cleaner, shorter filenames
- Enforced namespace-folder alignment

**Examples**:

Filename-based (flexible organization):
```toml
_version = "1"
subfolders_are_namespaces = false
```

Path-based (structured hierarchy):
```toml
_version = "1"
subfolders_are_namespaces = true
```

**Migration**:

To convert from filename-based to path-based:
1. Set `subfolders_are_namespaces = true`
2. Reorganize files to match namespace structure
3. Rename files to remove namespace prefix

To convert from path-based to filename-based:
1. Move files and encode full HRID in filename
2. Set `subfolders_are_namespaces = false`
3. Optionally flatten directory structure

See [Directory Structure](../configuration/directory-structure.md) for detailed migration guide.

**Validation**:
- Must be boolean
- Case-sensitive: `true` or `false` (lowercase)

**Errors**:

Wrong type:
```toml
subfolders_are_namespaces = "yes"  # Wrong: string instead of boolean
```
```
Error: Failed to parse config file: invalid type: string, expected a bool
```

## Minimal Configuration

Smallest valid configuration:

```toml
_version = "1"
```

All other fields use defaults:
- `allowed_kinds = []` (all kinds allowed)
- `digits = 3`
- `allow_unrecognised = false`
- `subfolders_are_namespaces = false`

## Default Configuration

If `config.toml` is absent, equivalent to:

```toml
_version = "1"
allowed_kinds = []
digits = 3
allow_unrecognised = false
subfolders_are_namespaces = false
```

## Configuration Examples

### Small Project

```toml
_version = "1"
digits = 3
allow_unrecognised = false
```

### Large Project

```toml
_version = "1"
allowed_kinds = ["USR", "SYS", "SWR", "HWR", "TST", "DOC"]
digits = 4  # Expect 1000+ requirements per kind
allow_unrecognised = false
```

### Integrated Documentation

```toml
_version = "1"
allowed_kinds = ["USR", "SYS"]
digits = 3
allow_unrecognised = true  # Allow MdBook/Sphinx files
```

### Migration Project

```toml
_version = "1"
digits = 3
allow_unrecognised = true   # Mixed content during migration
```

### Aerospace (DO-178C)

```toml
_version = "1"
allowed_kinds = ["URQT", "SRQT", "SWRQT", "HWRQT", "TRQT"]
digits = 4
allow_unrecognised = false
```

### Multi-Component System

```toml
_version = "1"
allowed_kinds = [
    "CORE-USR", "CORE-SYS",
    "AUTH-USR", "AUTH-SYS",
    "PAYMENT-USR", "PAYMENT-SYS",
    "REPORTING-USR", "REPORTING-SYS"
]
digits = 3
allow_unrecognised = false
```

## Validation

### Parsing Errors

**Missing _version**:
```toml
allowed_kinds = ["USR"]
```
```
Error: Failed to parse config file: missing field '_version'
```

**Invalid TOML syntax**:
```toml
_version = "1
allowed_kinds = ["USR"]
```
```
Error: Failed to parse config file: expected '"', got newline
```

**Wrong type**:
```toml
_version = 1  # Should be string
```
```
Error: Failed to parse config file: invalid type: integer, expected a string
```

**Unknown field** (future-proofing):
```toml
_version = "1"
unknown_field = "value"
```
Currently allowed (forward compatibility), but may be rejected in strict mode.

### Runtime Validation

Configuration is loaded at:
- `req add`
- `req link`
- `req clean`

Errors reported immediately:
```bash
req clean
# Error: Failed to load config: missing field '_version'
```

## Schema Evolution

### Version 1 (Current)

Fields:
- `_version` (required)
- `allowed_kinds` (optional)
- `digits` (optional)
- `allow_unrecognised` (optional)
- `subfolders_are_namespaces` (optional)

### Future Versions

Planned fields (not yet implemented):

**Version 2** (hypothetical):
```toml
_version = "2"

# Existing fields
allowed_kinds = ["USR", "SYS"]
digits = 3
allow_unrecognised = false

# New fields
[namespaces]
required = true
allowed = ["AUTH", "PAYMENT"]

[review]
auto_flag = true
notify = "team@example.com"

[coverage]
minimum = 95
```

**Compatibility**:
- Old Requiem: Rejects `_version = "2"` (unknown version)
- New Requiem: Reads `_version = "1"` (backward compatible)

## Troubleshooting

### Config Not Recognized

**Problem**: Changes to `config.toml` don't take effect.

**Solution**:
- Ensure file is named exactly `config.toml` (lowercase, no extension)
- Ensure file is in requirements root directory
- Verify TOML syntax with validator

### Parse Errors

**Problem**: `Error: Failed to parse config file`

**Diagnosis**:
1. Check TOML syntax
2. Ensure strings are quoted
3. Ensure arrays use square brackets
4. Check for typos in field names

**Solution**: Validate TOML:
```bash
# Python
python -c "import sys, toml; toml.load(open('config.toml'))"

# Online validator
# Copy config to https://www.toml-lint.com/
```

### Requirements Rejected

**Problem**: `req add` fails with "Kind not in allowed list"

**Diagnosis**: Check `allowed_kinds` in config.

**Solution**: Add kind to `allowed_kinds` or use empty array:
```toml
allowed_kinds = []  # Allow all kinds
```

### Files Ignored Unexpectedly

**Problem**: Valid requirement files not loaded.

**Diagnosis**: Check filename matches HRID pattern.

**Solution**:
- Ensure filename is `{KIND}-{ID}.md`
- Set `allow_unrecognised = false` to get error messages

## Summary

**Configuration file**:
- Location: `config.toml` in requirements root
- Format: TOML
- Optional: Uses defaults if absent

**Required fields**:
- `_version`: Schema version (currently `"1"`)

**Optional fields**:
- `allowed_kinds`: Restrict requirement kinds (default: `[]`, allow all)
- `digits`: HRID digit padding (default: `3`)
- `allow_unrecognised`: Allow non-HRID files (default: `false`)
- `subfolders_are_namespaces`: Use path-based structure (default: `false`)

**Defaults**:
- All kinds allowed
- 3-digit HRID padding
- Strict file validation (non-HRID files rejected)
- Filename-based directory structure

## Next Steps

- See [CLI Command Reference](./cli.md) for commands affected by configuration
- See [File Format Specification](./file-format.md) for requirement format
- Review [Configuration File](../configuration/config-file.md) for detailed explanations
