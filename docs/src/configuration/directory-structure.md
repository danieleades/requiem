# Directory Structure

Requiem is flexible about how you organize requirements on disk. This chapter explains directory organization options and best practices.

## Directory Structure Modes

Requiem supports two fundamental modes for organizing requirements, controlled by the `subfolders_are_namespaces` configuration option:

### 1. Filename-Based Mode (Default)

**Configuration**: `subfolders_are_namespaces = false` (or omitted)

In this mode, the full HRID is encoded in the filename, and directory structure is purely organizational:

```
my-requirements/
├── USR-001.md              → HRID: USR-001
├── SYS-002.md              → HRID: SYS-002
├── auth-USR-003.md         → HRID: auth-USR-003
└── custom/folder/
    └── system-REQ-004.md   → HRID: system-REQ-004
```

**Use when**: You want maximum flexibility in folder organization without affecting requirement namespaces.

### 2. Path-Based Mode

**Configuration**: `subfolders_are_namespaces = true`

In this mode, subfolders encode the namespace, and the filename contains only KIND-ID:

```
my-requirements/
├── REQ-001.md              → HRID: REQ-001 (no namespace)
├── system/
│   └── auth/
│       ├── REQ-002.md      → HRID: system-auth-REQ-002
│       └── USR/
│           └── 003.md      → HRID: system-auth-USR-003 (KIND from parent)
```

The filename format is automatically inferred:
- **Numeric only** (e.g., `003.md`) → KIND taken from parent folder
- **KIND-ID** (e.g., `REQ-002.md`) → KIND and ID from filename

**Use when**: You want folder structure to directly mirror requirement namespaces.

## Basic Structure

At minimum, you need a directory containing requirement markdown files:

```
my-requirements/
├── USR-001.md
├── USR-002.md
├── SYS-001.md
└── SYS-002.md
```

That's it! No initialization or hidden directories required.

## With Configuration

Add a `config.toml` for project-specific settings:

```
my-requirements/
├── config.toml      ← Configuration
├── USR-001.md
├── USR-002.md
├── SYS-001.md
└── SYS-002.md
```

The configuration file is optional but recommended for projects with specific needs.

## Using Subdirectories

Requiem recursively searches subdirectories, enabling hierarchical organization.

### Organize by Requirement Kind

```
requirements/
├── config.toml
├── user/
│   ├── USR-001.md
│   ├── USR-002.md
│   └── USR-003.md
├── system/
│   ├── SYS-001.md
│   ├── SYS-002.md
│   └── SYS-003.md
└── test/
    ├── TST-001.md
    └── TST-002.md
```

**Advantages**:
- Clear separation of requirement levels
- Easy to navigate
- Natural for large projects

**Note**: In filename-based mode (default), directory names don't affect requirement behavior. `user/USR-001.md` and `system/USR-001.md` would create a filename conflict (both have HRID `USR-001`), so use different kinds. In path-based mode, these would have different HRIDs: `user-USR-001` and `system-USR-001`.

### Organize by Feature

```
requirements/
├── config.toml
├── authentication/
│   ├── USR-001.md   # User login
│   ├── SYS-001.md   # Auth service
│   └── TST-001.md   # Auth tests
├── payment/
│   ├── USR-002.md   # Payment processing
│   ├── SYS-002.md   # Payment gateway
│   └── TST-002.md   # Payment tests
└── reporting/
    ├── USR-003.md   # Report generation
    └── SYS-003.md   # Report engine
```

**Advantages**:
- Groups related requirements
- Mirrors feature structure
- Good for feature-based development

### Organize by Component

```
requirements/
├── config.toml
├── frontend/
│   ├── WEB-USR-001.md
│   ├── WEB-SYS-001.md
│   ├── MOBILE-USR-001.md
│   └── MOBILE-SYS-001.md
└── backend/
    ├── API-SYS-001.md
    ├── DB-SYS-001.md
    └── CACHE-SYS-001.md
```

**Advantages**:
- Aligns with system architecture
- Clear ownership boundaries
- Natural for microservices

**Note**: Consider using namespaces in HRIDs (e.g., `WEB-USR-001`) to avoid conflicts.

### Deep Hierarchies

Requiem supports arbitrary nesting:

```
requirements/
├── config.toml
└── product/
    ├── core/
    │   ├── auth/
    │   │   ├── USR-001.md
    │   │   └── SYS-001.md
    │   └── data/
    │       ├── USR-002.md
    │       └── SYS-002.md
    └── plugins/
        ├── export/
        │   └── USR-003.md
        └── import/
            └── USR-004.md
```

**Caution**: Deep hierarchies can be harder to navigate. Consider flat or shallow structures unless your project naturally requires depth.

## Flat vs. Hierarchical

### Flat Structure

```
requirements/
├── config.toml
├── USR-001.md
├── USR-002.md
├── ...
├── USR-050.md
├── SYS-001.md
├── SYS-002.md
├── ...
└── SYS-100.md
```

**Advantages**:
- Simple and straightforward
- Easy to understand
- Works well for small/medium projects (< 100 requirements)

**Disadvantages**:
- Directory listing can become unwieldy with many requirements
- Harder to navigate in file browser with 100+ files

**Best for**: Small to medium projects, single-component systems.

### Hierarchical Structure

```
requirements/
├── config.toml
├── user-requirements/
│   └── (50 files)
├── system-requirements/
│   └── (100 files)
└── test-requirements/
    └── (150 files)
```

**Advantages**:
- Scalable to large projects (1000+ requirements)
- Natural organization
- Easier to navigate

**Disadvantages**:
- Slightly more complex
- Must decide on hierarchy scheme

**Best for**: Large projects, multi-component systems.

## Best Practices

### 1. Start Flat, Refactor When Needed

Begin with a flat structure:

```
requirements/
├── config.toml
├── USR-001.md
└── SYS-001.md
```

Introduce subdirectories when you have 50+ requirements or natural groupings emerge.

### 2. Use Consistent Naming

If using subdirectories, name them consistently:

```
requirements/
├── 1-user-requirements/
├── 2-system-requirements/
├── 3-software-requirements/
└── 4-test-requirements/
```

Or:

```
requirements/
├── user/
├── system/
├── software/
└── test/
```

**Avoid**: Mixing naming schemes (`user_reqs/`, `system-requirements/`, `Tests/`).

### 3. Keep Shallow (2-3 Levels Max)

Prefer:
```
requirements/
└── user/
    └── USR-001.md
```

Over:
```
requirements/
└── level1/
    └── level2/
        └── level3/
            └── level4/
                └── USR-001.md
```

Deep hierarchies are hard to navigate and remember.

### 4. Align with Team Structure

Organize to match how your team works:

**Team by feature**:
```
requirements/
├── login-team/
├── payment-team/
└── reporting-team/
```

**Team by layer**:
```
requirements/
├── product-managers/  (USR requirements)
├── architects/        (SYS requirements)
└── developers/        (SWR requirements)
```

### 5. Don't Encode Information in Paths

**Bad**:
```
requirements/
└── high-priority/
    └── USR-001.md  # Priority encoded in directory
```

**Good**:
```
requirements/
└── USR-001.md

# USR-001.md content:
---
tags:
- high-priority
---
The requirement text...
```

Use tags or content for metadata, not directory structure. Directories are for organization only.

## Special Cases

### Mixed Content (Requirements + Documentation)

If requirements live alongside other documentation:

```
docs/
├── config.toml
├── introduction.md      ← Not a requirement
├── architecture.md      ← Not a requirement
├── requirements/
│   ├── USR-001.md      ← Requirement
│   └── SYS-001.md      ← Requirement
└── user-guide.md        ← Not a requirement
```

Set `allow_unrecognised = true` in `config.toml` to allow non-requirement markdown files.

### Integration with MdBook

MdBook projects:

```
docs/
├── book.toml           ← MdBook config
├── src/
│   ├── SUMMARY.md      ← MdBook table of contents
│   ├── chapter1.md     ← Documentation
│   ├── USR-001.md      ← Requirement (can be in SUMMARY.md)
│   └── USR-002.md      ← Requirement
└── config.toml         ← Requiem config (allow_unrecognised = true)
```

See [Integration > Using with MdBook](../integration/mdbook.md) for details.

### Integration with Sphinx

Sphinx projects:

```
docs/
├── conf.py             ← Sphinx config
├── index.md
├── requirements/
│   ├── config.toml     ← Requiem config
│   ├── USR-001.md      ← Requirement
│   └── SYS-001.md      ← Requirement
└── other-content.md
```

See [Integration > Using with Sphinx](../integration/sphinx.md) for details.

### Monorepo Structure

Large monorepos with multiple products:

```
monorepo/
├── products/
│   ├── web-app/
│   │   └── requirements/
│   │       ├── config.toml
│   │       └── WEB-USR-001.md
│   └── mobile-app/
│       └── requirements/
│           ├── config.toml
│           └── MOBILE-USR-001.md
└── shared/
    └── requirements/
        ├── config.toml
        └── CORE-SYS-001.md
```

Each product/component has its own independent requirements directory with separate `config.toml`.

## File System Considerations

### Case Sensitivity

**Linux/Mac**: Filenames are case-sensitive
- `USR-001.md` and `usr-001.md` are different files
- Requiem only recognizes `USR-001.md` (uppercase HRID)

**Windows**: Filenames are case-insensitive
- `USR-001.md` and `usr-001.md` refer to the same file
- Use consistent casing to avoid issues

**Recommendation**: Always use uppercase HRIDs to avoid cross-platform issues.

### Symbolic Links

Requiem follows symbolic links when scanning for requirements:

```
requirements/
├── current -> v2.0/     ← Symlink
├── v1.0/
│   └── USR-001.md
└── v2.0/
    └── USR-001.md
```

**Caution**: Ensure symlinks don't create cycles or duplicate requirements.

### Hidden Files and Directories

Files and directories starting with `.` are typically ignored by Requiem (following standard Unix convention):

```
requirements/
├── .git/               ← Ignored
├── .DS_Store           ← Ignored (Mac)
├── config.toml
└── USR-001.md
```

## Performance Considerations

### Large Directories

Requiem uses parallel loading for performance:

```
requirements/
├── USR-001.md
├── USR-002.md
├── ...
└── USR-5000.md  # 5000 files loads quickly due to parallelism
```

**Performance**: Requiem can handle thousands of requirements efficiently. Directory structure has minimal performance impact.

### Network File Systems

If requirements are on a network drive:
- Initial loading may be slower
- Consider using subdirectories to localize access patterns
- Parallel loading helps mitigate network latency

## Choosing Between Modes

### When to Use Filename-Based Mode (Default)

**Advantages**:
- Maximum flexibility - organize folders however you want
- Easy to move files between folders without changing HRIDs
- Works well with arbitrary organizational schemes (by feature, by team, etc.)
- Explicit namespaces visible in every filename

**Best for**:
- Projects where folder structure is organizational, not semantic
- Teams that reorganize folders frequently
- Mixed organizational schemes
- When you want full control over namespaces

**Example**:
```
requirements/
├── team-a/
│   └── auth-USR-001.md      → HRID: auth-USR-001
└── team-b/
    └── auth-SYS-002.md      → HRID: auth-SYS-002
```

### When to Use Path-Based Mode

**Advantages**:
- Cleaner filenames (shorter, less redundant)
- Folder structure enforces namespace consistency
- Natural for hierarchical component structures
- Automatic namespace creation from folders

**Best for**:
- Projects with stable, hierarchical component structures
- When folder structure mirrors system architecture
- Microservices or multi-component systems
- When you want enforced namespace-folder alignment

**Example**:
```
requirements/
├── auth/
│   ├── USR-001.md           → HRID: auth-USR-001
│   └── SYS-002.md           → HRID: auth-SYS-002
└── payment/
    ├── USR-001.md           → HRID: payment-USR-001
    └── SYS-002.md           → HRID: payment-SYS-002
```

### Comparison Table

| Aspect | Filename-Based | Path-Based |
|--------|----------------|------------|
| **Filename contains** | Full HRID | KIND-ID only |
| **Namespace from** | Filename | Folder path |
| **Folder flexibility** | High | Low (tied to namespace) |
| **File move impact** | None | Changes HRID |
| **Filename length** | Longer | Shorter |
| **Default mode** | Yes | No |

## Migration Between Modes

### Converting from Filename-Based to Path-Based

1. **Update configuration**:
```toml
# config.toml
_version = "1"
subfolders_are_namespaces = true
```

2. **Reorganize files** to match namespace structure:

```bash
# Before (filename-based):
# requirements/
# ├── auth-USR-001.md
# └── payment-USR-002.md

# Create namespace folders
mkdir -p auth payment

# Move and rename files
mv auth-USR-001.md auth/USR-001.md
mv payment-USR-002.md payment/USR-002.md

# After (path-based):
# requirements/
# ├── auth/
# │   └── USR-001.md
# └── payment/
#     └── USR-002.md
```

3. **Verify**:
```bash
req clean
```

### Converting from Path-Based to Filename-Based

1. **Flatten files** and encode namespace in filename:

```bash
# Before (path-based):
# requirements/
# ├── auth/
# │   └── USR-001.md        (HRID: auth-USR-001)
# └── payment/
#     └── USR-002.md        (HRID: payment-USR-002)

# Move and rename with full HRID
mv auth/USR-001.md auth-USR-001.md
mv payment/USR-002.md payment-USR-002.md

# Remove now-empty folders
rmdir auth payment

# After (filename-based):
# requirements/
# ├── auth-USR-001.md
# └── payment-USR-002.md
```

2. **Update configuration**:
```toml
# config.toml
_version = "1"
subfolders_are_namespaces = false
```

3. **Verify**:
```bash
req clean
```

**Note**: When converting between modes, the HRID (and thus UUID) remains the same, preserving all links and history.

## Migration and Refactoring

### Reorganizing Files

To reorganize directory structure in filename-based mode:

1. Move requirement files:
```bash
mkdir system
mv SYS-*.md system/
```

2. Verify:
```bash
req clean
```

3. Commit:
```bash
git add -A
git commit -m "Organize system requirements into subdirectory"
```

**Safe because**: Requirements are identified by HRID (filename), not path. Moving files doesn't break links.

### Splitting Directories

To split a large flat directory:

```bash
# Before
requirements/
├── USR-001.md (100 files)

# After
requirements/
├── user/
│   └── USR-001.md (100 files)
└── system/
    └── SYS-001.md (100 files)
```

Steps:
```bash
mkdir user system test
mv USR-*.md user/
mv SYS-*.md system/
mv TST-*.md test/
req clean  # Verify
```

## Summary

**Key Takeaways**:

1. **Flexible**: Flat or hierarchical, your choice
2. **Recursive**: Subdirectories are automatically scanned
3. **Simple**: Directory names don't affect requirement behavior
4. **Scalable**: Supports small projects (10s) to large (1000s)
5. **Reorganizable**: Safe to move files (links use UUIDs, not paths)

**Recommendations**:
- Start flat, add structure as needed
- Keep shallow (2-3 levels max)
- Organize by kind, feature, or component
- Use consistent naming conventions
- Align with team structure

## Next Steps

- Understand [Namespaces](./namespaces.md) for large multi-component projects
- Learn about [Version Control Best Practices](../integration/version-control.md)
