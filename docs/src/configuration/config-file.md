# Configuration File

Requiem uses a TOML configuration file named `config.toml` in the root of your requirements directory to customize behavior.

## Location

The configuration file must be named `config.toml` and placed in the root directory where your requirements are stored:

```
my-requirements/
├── config.toml       ← Configuration file
├── USR-001.md
├── USR-002.md
└── SYS-001.md
```

If no configuration file exists, Requiem uses sensible defaults and continues without error.

## File Format

The configuration file is written in TOML (Tom's Obvious Minimal Language), a simple, human-readable format.

### Minimal Example

The simplest valid configuration:

```toml
_version = "1"
```

This accepts all defaults. The `_version` field is required for future format compatibility.

### Complete Example

A configuration using all available options:

```toml
_version = "1"

# Restrict requirement kinds
allowed_kinds = ["USR", "SYS", "SWR", "TST"]

# Number of digits in HRID numbering
digits = 3

# Allow non-requirement markdown files
allow_unrecognised = false
```

## Configuration Options

### `_version` (required)

```toml
_version = "1"
```

**Type**: String (quoted)

**Default**: N/A (required field)

**Purpose**: Specifies the configuration format version. This enables future format changes while maintaining backward compatibility.

**Current version**: `"1"` (as a string, not a number)

**Example**:
```toml
_version = "1"  # Correct
```

**Invalid**:
```toml
_version = 1    # Wrong: must be a quoted string
```

### `allowed_kinds` (optional)

```toml
allowed_kinds = ["USR", "SYS", "SWR", "TST"]
```

**Type**: Array of strings

**Default**: `[]` (empty array = all kinds allowed)

**Purpose**: Restricts which requirement kinds (the KIND component of HRIDs) are permitted in your project.

**When to use**:
- Enforce project standards (e.g., only USR, SYS, TST requirements allowed)
- Prevent typos (USR-001 vs. UST-001)
- Document your requirement taxonomy

**Behavior**:
- **Empty array** (default): Any kind is accepted
- **Non-empty array**: Only listed kinds are valid

**Example - Aerospace project**:
```toml
allowed_kinds = ["URQT", "SRQT", "SWRQT", "HWRQT", "TEST"]
# User Requirements (URQT), System Requirements (SRQT), etc.
```

**Example - Software project**:
```toml
allowed_kinds = ["USR", "SYS", "SWR", "TST", "DOC"]
```

**Enforcement**:
When `allowed_kinds` is non-empty, attempting to create a requirement with a disallowed kind will fail:

```bash
$ req add INVALID
Error: Kind 'INVALID' is not in the allowed list
```

### `digits` (optional)

```toml
digits = 3
```

**Type**: Unsigned integer

**Default**: `3`

**Purpose**: Specifies the minimum number of digits used in HRID numbering. IDs are zero-padded to this width.

**Valid values**: Any positive integer, though 3 or 4 are most common.

**Behavior**:

With `digits = 3`:
```
USR-001
USR-002
...
USR-099
USR-100  # Exceeds 3 digits when needed
```

With `digits = 4`:
```
USR-0001
USR-0002
...
USR-9999
USR-10000  # Exceeds 4 digits when needed
```

**Choosing a value**:
- `digits = 3`: Projects with < 1000 requirements per kind
- `digits = 4`: Projects with 1000+ requirements per kind
- `digits = 5`: Very large projects

**Note**: This setting affects display format only. Parsing accepts any number of digits:
- `USR-1`, `USR-01`, `USR-001` all parse as ID 1
- Display format uses the configured padding

**Example - Large project**:
```toml
digits = 4
# Requirements display as USR-0001, USR-0002, etc.
```

### `allow_unrecognised` (optional)

```toml
allow_unrecognised = false
```

**Type**: Boolean

**Default**: `false`

**Purpose**: Controls whether markdown files that don't match the HRID pattern are allowed in the requirements directory.

**Behavior**:

**`false` (default - strict mode)**:
- Only files matching the HRID pattern (e.g., `USR-001.md`) are allowed
- Any other `.md` file causes an error during loading
- Ensures clean, requirements-only directory

**`true` (permissive mode)**:
- Files with non-HRID names are silently ignored
- Useful when requirements live alongside other documentation

**When to use `true`**:

**Integration with documentation tools**:
```
docs/
├── config.toml          ← allow_unrecognised = true
├── introduction.md      ← Ignored (not an HRID)
├── architecture.md      ← Ignored
├── USR-001.md          ← Loaded as requirement
└── USR-002.md          ← Loaded as requirement
```

**Mixed content repositories**:
```
project-docs/
├── README.md           ← Ignored
├── CHANGELOG.md        ← Ignored
├── requirements/
│   ├── USR-001.md      ← Loaded
│   └── SYS-001.md      ← Loaded
```

**When to use `false` (default)**:
- Dedicated requirements directory
- Strict separation between requirements and other docs
- Catch typos (e.g., `US-001.md` instead of `USR-001.md`)

**Error example with `allow_unrecognised = false`**:
```bash
$ req clean
Error: Unrecognised file: README.md
```

## Configuration Strategy

### Start Simple

Begin with minimal configuration:

```toml
_version = "1"
```

Add constraints as your project matures.

### Recommended Settings

**Small project (< 100 requirements)**:
```toml
_version = "1"
digits = 3
allow_unrecognised = false
```

**Large project (> 1000 requirements)**:
```toml
_version = "1"
allowed_kinds = ["USR", "SYS", "SWR", "HWR", "TST", "DOC"]
digits = 4
allow_unrecognised = false
```

**Integrated documentation project**:
```toml
_version = "1"
allowed_kinds = ["USR", "SYS"]
digits = 3
allow_unrecognised = true  # Mixed with other docs
```

## Validation

Validate your configuration by running:

```bash
req clean
```

This loads all requirements and reports configuration-related errors.

**Successful validation**:
```bash
$ req clean
# No output = success
```

**Configuration error**:
```bash
$ req clean
Error: Failed to parse config file: missing field '_version'
```

## Configuration Examples

### Regulated Industry (Aerospace)

```toml
_version = "1"

# DO-178C levels: User, System, Software, Hardware, Test
allowed_kinds = ["URQT", "SRQT", "SWRQT", "HWRQT", "TRQT"]

# Large project
digits = 4

# Strict validation
allow_unrecognised = false
```

### Agile Software Project

```toml
_version = "1"

# User stories, system reqs, tests
allowed_kinds = ["USR", "SYS", "TST"]

# Small/medium project
digits = 3

# Strict validation
allow_unrecognised = false
```

### Multi-Component System

```toml
_version = "1"

# Component-prefixed kinds
allowed_kinds = [
    "USR",           # Cross-cutting user requirements
    "AUTH-SYS",      # Authentication subsystem
    "PAY-SYS",       # Payment subsystem
    "REPORT-SYS",    # Reporting subsystem
]

digits = 3
allow_unrecognised = false
```

## Troubleshooting

### Config file not recognized

**Problem**: Changes to `config.toml` don't take effect.

**Solution**: Ensure the file is named exactly `config.toml` (lowercase, no extra extensions) and is in the requirements root directory.

### Parse errors

**Problem**: `Error: Failed to parse config file`

**Solution**: Validate TOML syntax:
- Strings must be quoted: `_version = "1"`, not `_version = 1`
- Arrays use square brackets: `allowed_kinds = ["USR", "SYS"]`
- Check for typos in field names

Use a TOML validator or linter if needed.

### Unexpected behavior

**Problem**: Requirements are rejected unexpectedly.

**Solution**:
1. Check `allowed_kinds` - ensure the kinds you're using are listed
2. Check `allow_unrecognised` - set to `true` if mixing requirements with other docs
3. Check requirement files for YAML formatting errors or missing required fields

## Future Configuration Options

Planned configuration options (not yet implemented):

- **`namespace_separator`**: Customize the separator in namespaced HRIDs
- **`require_namespaces`**: Enforce namespace usage
- **`max_parents`**: Limit number of parent requirements
- **`tag_validation`**: Restrict allowed tag values
- **`review_policies`**: Configure review workflow behavior

## Next Steps

- Learn about [Directory Structure](./directory-structure.md) for organizing requirements
- Understand [Namespaces](./namespaces.md) for large projects
