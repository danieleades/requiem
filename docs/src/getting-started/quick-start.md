# Quick Start Tutorial

This 5-minute tutorial introduces Requiem's basic workflow. You'll create a simple set of requirements and link them together.

## Creating a Requirements Directory

First, create a directory for your requirements and initialize it:

```bash
mkdir my-requirements
cd my-requirements
req init                 # creates .req/config.toml and templates folder
req kind add USR SYS     # optional: restrict allowed kinds
```

Requiem works with any directory—requirements are markdown files with YAML frontmatter.

## Adding Your First Requirement

Create a user requirement using `create`:

```bash
req create USR --title "Plain Text Requirements"
```

This creates a file named `USR-001.md` with automatically generated metadata. Output:

```
Added requirement USR-001
```

The file contains:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---
# USR-001 Plain Text Requirements


```

The YAML frontmatter includes:
- `_version`: Format version for future compatibility
- `uuid`: A globally unique, stable identifier
- `created`: Timestamp of creation

The heading includes:
- `# USR-001`: The HRID as the first token in the first heading
- Followed by the title text (currently empty)

The body (currently empty) is where you'll write the requirement text.

## Editing the Requirement

Open `USR-001.md` in your text editor and add content:

```markdown
---
_version: '1'
uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
created: 2025-07-22T12:19:56.950194157Z
---
# USR-001 Plain Text Requirements

The system shall support plain-text requirements that can be edited with any text editor.
```

Notice the HRID (`USR-001`) appears as the first token in the first heading, followed by the title. Save the file. That's it! You've created your first requirement.

## Adding More Requirements

Create a few more user requirements:

```bash
req create USR --title "Version Control Integration"        # USR-002
req create USR --title "Requirement Traceability"           # USR-003
```

Edit these files to add meaningful content. For example:

**USR-002.md:**
```markdown
---
_version: '1'
uuid: ...
created: ...
---
# USR-002 Version Control Integration

The system shall integrate with version control systems like Git.
```

**USR-003.md:**
```markdown
---
_version: '1'
uuid: ...
created: ...
---
# USR-003 Requirement Traceability

The system shall support requirement traceability and linkage.
```

## Creating System Requirements

Now let's create system-level requirements that satisfy the user requirements:

```bash
req create SYS --title "Markdown File Format"  # Creates SYS-001
```

## Linking Requirements

Link `SYS-001` to its parent `USR-001`:

```bash
req link SYS-001 USR-001
```

Output:

```
Linked SYS-001 to USR-001
```

Now if you open `SYS-001.md`, you'll see the parent relationship in the frontmatter and the HRID in the heading:

```markdown
---
_version: '1'
uuid: 81e63bac-4035-47b5-b273-ac13e47a2ff6
created: 2025-07-22T13:14:40.510075462Z
parents:
- uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a
  fingerprint: e533784ff58c16cbf08e436cb06f09e0076880fd707baaf55aa0f45dc4a6ccda
  hrid: USR-001
---
# SYS-001 Markdown File Format

Each requirement shall be stored as a markdown file with YAML frontmatter.
```

The `parents` section contains:
- `uuid`: The stable identifier of the parent
- `hrid`: The human-readable ID (for your convenience)
- `fingerprint`: A hash of the parent's content (for change detection)

## Creating Requirements with Multiple Parents

You can link a requirement to multiple parents when creating it:

```bash
req create SYS --parent USR-001 --parent USR-002
```

This creates `SYS-002` already linked to both `USR-001` and `USR-002`.

## Viewing Requirements

Requirements are just markdown files, so you can view them with any tool:

```bash
ls *.md
cat USR-001.md
```

Or use your favorite text editor, IDE, or markdown viewer.

## Summary

You've learned the three core commands:

1. **`req create <KIND>`** - Create a new requirement
2. **`req link <CHILD> <PARENT>`** - Link two requirements
3. **`req create <KIND> --parent <PARENT>`** - Create with parents

These commands form the foundation of requirements management with Requiem.

## Next Steps

Continue to [Your First Requirements Project](./first-project.md) to build a complete requirement hierarchy and learn more advanced techniques.
