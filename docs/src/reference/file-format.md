# File Format Specification

Formal specification of the Requiem requirement file format.

## Overview

Requiem requirements are stored as Markdown files with YAML frontmatter. This document provides the formal specification.

### File Extension

`.md` (Markdown)

### Character Encoding

UTF-8

### Filename Format

```
{NAMESPACE-}*{KIND}-{ID}.md
```

Where:
- `NAMESPACE`: Zero or more namespace segments (alphanumeric, hyphen-separated)
- `KIND`: Alphanumeric requirement kind (e.g., `USR`, `SYS`)
- `ID`: Positive integer (zero-padded to configured digits, default 3)

**Examples**:
```
USR-001.md
SYS-042.md
AUTH-USR-001.md
MOBILE-AUTH-LOGIN-SYS-005.md
```

**Validation Rules**:
- Must end with `.md`
- NAMESPACE segments: non-empty, alphanumeric plus hyphen
- KIND: non-empty, alphanumeric
- ID: positive integer (1 or more), may have leading zeros
- Segments separated by single hyphen `-`
- No consecutive hyphens `--`
- No leading or trailing hyphens

## File Structure

Requirements consist of three parts:

1. **YAML Frontmatter**: Metadata enclosed in `---` delimiters
2. **HRID Heading**: First heading with HRID as first token
3. **Markdown Body**: Requirement text in CommonMark format

### General Structure

```
---
<YAML frontmatter>
---
# <HRID> <Title>
<blank line optional>
<Markdown content>
```

### Example

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
tags:
- authentication
- security
parents:
- uuid: 3fc6800c-5acc-457e-baf9-a29b42b663fd
  fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
  hrid: USR-001
---
# SYS-001 Email Validation

The system shall validate user email addresses according to RFC 5322.

Email validation must occur before account creation.
```

## YAML Frontmatter

### Delimiters

- **Opening delimiter**: `---` on first line
- **Closing delimiter**: `---` on its own line
- Both required

### Schema Version 1

Current version: `1`

#### Required Fields

##### `_version`

**Type**: String (quoted)

**Format**: `"1"`

**Purpose**: Schema version for forward/backward compatibility

**Validation**:
- Must be present
- Must be string type (quoted in YAML)
- Currently only `"1"` is valid

**Example**:
```yaml
_version: '1'
```

##### `uuid`

**Type**: UUID (string format)

**Format**: UUID v4 (RFC 4122)

**Purpose**: Globally unique, stable identifier

**Validation**:
- Must be present
- Must be valid UUID format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
- Should be generated with `uuid::Uuid::new_v4()` or equivalent
- Must be unique across all requirements (globally)
- Must never change after creation

**Example**:
```yaml
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
```

##### `created`

**Type**: Timestamp (string format)

**Format**: ISO 8601 with timezone (RFC 3339)

**Purpose**: Requirement creation timestamp

**Validation**:
- Must be present
- Must be valid ISO 8601 timestamp
- Must include timezone (use UTC: `Z` suffix)
- Format: `YYYY-MM-DDTHH:MM:SS.sssssssssZ`

**Example**:
```yaml
created: 2025-07-22T12:19:56.950194157Z
```

**Notes**:
- Nanosecond precision supported
- Timezone must be UTC (`Z` suffix)
- Set once at creation; never modified

#### Optional Fields

##### `tags`

**Type**: Array of strings

**Format**: YAML list

**Purpose**: Categorize and tag requirements

**Validation**:
- Optional (omit if no tags)
- Array elements must be strings
- Empty array allowed but should be omitted
- No duplicate tags within same requirement
- Tags are case-sensitive

**Example**:
```yaml
tags:
- authentication
- security
- high-priority
```

**Omission**:
```yaml
# No tags field = no tags
_version: '1'
uuid: ...
# tags field omitted
```

**Serialization**:
- Include only if non-empty: `skip_serializing_if = "BTreeSet::is_empty"`
- Stored as `BTreeSet` internally (sorted, unique)

##### `parents`

**Type**: Array of parent objects

**Format**: YAML list of objects

**Purpose**: Link to parent (upstream) requirements

**Validation**:
- Optional (omit if no parents, e.g., root requirements)
- Array elements must be parent objects (see Parent Object Schema)
- Empty array allowed but should be omitted
- Duplicate parent UUIDs allowed (though unusual)

**Example**:
```yaml
parents:
- uuid: 3fc6800c-5acc-457e-baf9-a29b42b663fd
  fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
  hrid: USR-001
- uuid: 7a8f9e2b-1c3d-4e5f-6a7b-8c9d0e1f2a3b
  fingerprint: a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
  hrid: USR-003
```

**Omission**:
```yaml
# No parents field = no parents (root requirement)
_version: '1'
uuid: ...
# parents field omitted
```

**Serialization**:
- Include only if non-empty: `skip_serializing_if = "Vec::is_empty"`

#### Parent Object Schema

Each parent object has three fields:

##### `uuid`

**Type**: UUID (string format)

**Format**: UUID v4

**Purpose**: Stable reference to parent requirement

**Validation**:
- Must be present in parent object
- Must be valid UUID format
- Should match `uuid` field of an existing requirement

**Example**:
```yaml
uuid: 3fc6800c-5acc-457e-baf9-a29b42b663fd
```

##### `fingerprint`

**Type**: String (hex-encoded SHA256 hash)

**Format**: 64-character hexadecimal string

**Purpose**: Content hash of parent for change detection

**Validation**:
- Must be present in parent object
- Must be 64-character hexadecimal string
- Generated by hashing parent's content and tags with SHA256

**Example**:
```yaml
fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
```

**Generation**:
```
1. Collect parent's content (markdown body) and tags
2. Serialize with Borsh encoding
3. Hash with SHA256
4. Encode as hexadecimal string (64 characters)
```

##### `hrid`

**Type**: String (HRID format)

**Format**: `{NAMESPACE-}*{KIND}-{ID}`

**Purpose**: Human-readable reference to parent (convenience field)

**Validation**:
- Must be present in parent object
- Must be valid HRID format
- Should match parent's filename (without `.md` extension)

**Example**:
```yaml
hrid: USR-001
```

**Notes**:
- Convenience field for human readability
- May become outdated if parent is renamed
- Corrected by `req clean` command
- UUID is authoritative; HRID is informational

### Schema Evolution

Future schema versions (e.g., `_version: "2"`) will be backward-compatible:
- New optional fields may be added
- Required fields will not be removed
- Field semantics will not change incompatibly

Parsers should:
- Ignore unknown fields
- Provide defaults for missing optional fields
- Reject unknown `_version` values (fail-safe)

## HRID Heading

### Format

The first heading in the markdown must contain the HRID as the first token:

```markdown
# <HRID> <Title Text>
```

### Requirements

- Must be a level-1 heading (`#`)
- HRID must be the first token (word)
- Followed by space and title text
- HRID must match the filename (without `.md`)

### Examples

```markdown
# USR-001 Plain Text Storage
# SYS-042 Email Validation System
# AUTH-LOGIN-SYS-001 Password Hashing
```

### Rationale

The HRID is stored in the heading (not frontmatter) for compatibility with Sphinx and MdBook, which use the first heading as the page title.

## Markdown Body

### Format

CommonMark (Markdown specification)

### Location

Everything after the first heading is the markdown body.

### Content

Free-form Markdown:
- Headings (`##`, `###`, etc. - first `#` is reserved for HRID)
- Paragraphs
- Lists (ordered, unordered)
- Code blocks (fenced, indented)
- Emphasis (bold, italic)
- Links
- Images
- Blockquotes
- Tables
- Any CommonMark-compliant content

### Whitespace

- Leading/trailing whitespace: preserved
- Empty lines between frontmatter and body: ignored
- Empty body: valid (empty string)

### Example

```markdown
---
_version: '1'
uuid: ...
created: ...
---
# USR-001 Email Validation

The system shall validate user email addresses.

## Rationale

Email validation ensures...

## Acceptance Criteria

- Valid email format
- Rejects invalid emails
- Provides clear error messages
```

## Parsing Rules

### Frontmatter Parsing

1. First line must be `---`
2. Read lines until next `---` on its own line
3. Parse collected lines as YAML
4. Validate against schema
5. Remaining lines are markdown body

### Error Handling

**Missing opening delimiter**:
```
Error: Expected frontmatter starting with '---'
```

**Missing closing delimiter**:
```
Error: Unexpected EOF while parsing frontmatter
```

**Invalid YAML**:
```
Error: Failed to parse YAML: <syntax error details>
```

**Missing required field**:
```
Error: Missing required field '<field_name>'
```

**Invalid UUID format**:
```
Error: Invalid UUID format: '<value>'
```

**Invalid timestamp format**:
```
Error: Invalid timestamp format: '<value>'
```

**Unknown _version**:
```
Error: Unknown schema version: '<value>'
```

### Strict vs. Permissive

Requiem parsing is strict by default:
- All required fields must be present
- All fields must be valid
- Unknown fields in schema version 1 cause errors (currently)

Controlled by `allow_invalid` config option:
- `allow_invalid = false` (default): Strict parsing, fail on errors
- `allow_invalid = true`: Skip invalid requirements with warnings

## Serialization Rules

### Field Order

Fields serialized in this order:
1. `_version`
2. `uuid`
3. `created`
4. `tags` (if present)
5. `parents` (if present)

### Omission Rules

- `tags`: Omitted if empty
- `parents`: Omitted if empty

### Formatting

- YAML indentation: 2 spaces
- String quoting: Single quotes for strings containing special characters
- Array formatting: One element per line with `-` prefix
- Object formatting: Indented key-value pairs

### Example Output

```yaml
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
tags:
- authentication
- security
parents:
- uuid: 3fc6800c-5acc-457e-baf9-a29b42b663fd
  fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
  hrid: USR-001
---

Requirement text here.
```

## Validation

### Syntactic Validation

- Valid YAML frontmatter
- Valid Markdown body (CommonMark)
- Correct delimiters

### Semantic Validation

- Required fields present
- Field types correct
- UUID format valid
- Timestamp format valid
- HRID format valid (in parent references)
- Fingerprint format valid (64-char hex)

### Referential Integrity

- Parent UUIDs reference existing requirements
- No self-references (requirement is not its own parent)
- No duplicate UUIDs across all requirements

### Configuration-Based Validation

- HRID KIND in `allowed_kinds` (if configured)
- File follows naming convention (if `allow_unrecognised = false`)

## Canonical Example

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
tags:
- authentication
- security
- high-priority
parents:
- uuid: 3fc6800c-5acc-457e-baf9-a29b42b663fd
  fingerprint: c4020419ead000e9b5f9cfd4ebf6192e73f905c27e6897548d8f6e12fd7f1356
  hrid: USR-001
- uuid: 7a8f9e2b-1c3d-4e5f-6a7b-8c9d0e1f2a3b
  fingerprint: a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
  hrid: USR-003
---
# SYS-001 Email Validation

The system shall validate user email addresses according to RFC 5322.

## Rationale

Email validation ensures that user accounts can be contacted and that
authentication tokens can be delivered reliably.

## Acceptance Criteria

- Email addresses must match RFC 5322 format
- Invalid email addresses must be rejected with clear error message
- Validation must occur before account creation

## References

- RFC 5322: Internet Message Format
- OWASP Email Validation Guidelines
```

## Summary

**File format**:
- Markdown (`.md`) files with UTF-8 encoding
- YAML frontmatter enclosed in `---` delimiters
- CommonMark markdown body

**Required frontmatter fields**:
- `_version`: Schema version (currently `"1"`)
- `uuid`: Unique identifier (UUID v4)
- `created`: Creation timestamp (ISO 8601 with UTC)

**Optional frontmatter fields**:
- `tags`: Array of tags
- `parents`: Array of parent objects (uuid, fingerprint, hrid)

**Validation**:
- Strict by default
- Controlled by `allow_invalid` config option
- Includes syntactic, semantic, and referential checks

## Next Steps

- See [CLI Command Reference](./cli.md) for working with requirements
- See [Configuration Reference](./configuration.md) for config options
- Review [Working with Requirements](../working-with-requirements/file-format.md) for practical examples
