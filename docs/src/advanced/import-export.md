# Import and Export

> **Note**: Import and export features are **planned but not yet implemented**. This chapter describes how these features will work when available.

Import and export enable interoperability with other requirements management tools and formats.

## Overview

Planned import/export formats:

- **ReqIF**: Requirements Interchange Format (OMG standard)
- **JSON**: Machine-readable format for custom tools
- **CSV**: Spreadsheet format for analysis
- **Doorstop YAML**: Migrate from Doorstop projects
- **HTML**: Human-readable export for documentation
- **PDF**: Formal document generation

## Why Import/Export Matters

**Migration**: Move requirements from other tools to Requiem

**Integration**: Exchange requirements with other systems

**Analysis**: Export to spreadsheets for custom analysis

**Documentation**: Generate formal requirement documents

**Backup**: Archive requirements in multiple formats

**Compliance**: Provide requirements in auditor-requested formats

## Export Functionality

### Export to JSON

```bash
req export --format json --output requirements.json
```

**Output format**:
```json
{
  "version": "1",
  "exported_at": "2025-07-22T12:00:00Z",
  "requirements": [
    {
      "hrid": "USR-001",
      "uuid": "4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a",
      "created": "2025-07-22T10:00:00Z",
      "content": "The system shall validate...",
      "tags": ["authentication", "security"],
      "parents": [
        {
          "uuid": "3fc6800c-5acc-457e-baf9-a29b42b663fd",
          "hrid": "USR-002",
          "fingerprint": "e533784ff58c16cbf08e436cb06f09e0..."
        }
      ]
    }
  ]
}
```

### Export to CSV

```bash
req export --format csv --output requirements.csv
```

**Output format**:
```csv
HRID,UUID,Created,Content,Tags,Parents
USR-001,4bfeb7d5-...,2025-07-22T10:00:00Z,"The system shall...","authentication,security","USR-002"
USR-002,3fc6800c-...,2025-07-22T09:00:00Z,"Users shall be able to...","",""
SYS-001,81e63bac-...,2025-07-22T11:00:00Z,"The authentication service...","","USR-001,USR-002"
```

Open in Excel, Google Sheets, or other spreadsheet tools.

### Export to ReqIF

```bash
req export --format reqif --output requirements.reqif
```

**ReqIF**: OMG standard for requirements exchange, compatible with:
- IBM DOORS
- Siemens Polarion
- PTC Integrity
- Jama Connect
- Many other tools

**Use case**: Send requirements to partners/customers using commercial tools.

### Export to HTML

```bash
req export --format html --output requirements.html
```

**Features**:
- Styled requirement documents
- Clickable traceability links
- Filterable by kind, tag
- Printable
- Standalone (no external dependencies)

### Export to PDF

```bash
req export --format pdf --output requirements.pdf
```

**Use case**: Formal requirement documents for:
- Reviews
- Approvals
- Audits
- Archival

## Import Functionality

### Import from JSON

```bash
req import --format json --input requirements.json
```

Creates `.md` files from JSON requirement data.

**Validation**:
- Checks for UUID conflicts
- Validates HRID format
- Ensures parent references are valid

**Conflict resolution**:
```bash
req import --format json --input requirements.json --on-conflict skip
req import --format json --input requirements.json --on-conflict overwrite
req import --format json --input requirements.json --on-conflict rename
```

### Import from CSV

```bash
req import --format csv --input requirements.csv
```

**CSV format requirements**:
- Must have headers: `HRID`, `UUID`, `Content`
- Optional: `Created`, `Tags`, `Parents`
- `Parents` column: comma-separated HRIDs or UUIDs

**Example CSV**:
```csv
HRID,UUID,Content,Tags,Parents
USR-001,4bfeb7d5-...,The system shall validate emails,authentication,
SYS-001,81e63bac-...,Email validation service,authentication,USR-001
```

### Import from Doorstop

Migrate from Doorstop projects:

```bash
req import --format doorstop --input /path/to/doorstop/project
```

**What's imported**:
- Doorstop documents → Requiem requirement kinds
- Doorstop items → Requiem requirements
- Links → Parent relationships
- Attributes → Tags or content

**Mapping**:
```
Doorstop                  Requiem
--------                  -------
Document PREFIX           Requirement KIND (e.g., USR)
Item UID                  UUID (generated or preserved)
Item text                 Requirement content
Item links                Parent relationships
Item attributes           Tags
```

### Import from ReqIF

```bash
req import --format reqif --input requirements.reqif
```

**Use case**: Import from commercial tools (DOORS, Polarion, etc.)

**Challenges**:
- ReqIF is complex; not all features map to Requiem
- May require manual cleanup after import

## Selective Export/Import

### Export Specific Requirements

By kind:
```bash
req export --kind USR --format json --output usr-requirements.json
```

By tag:
```bash
req export --tag security --format csv --output security-reqs.csv
```

By namespace:
```bash
req export --namespace AUTH --format html --output auth-reqs.html
```

### Import with Filtering

Skip certain requirements:
```bash
req import --format json --input reqs.json --exclude-kind DOC
```

Rename on import:
```bash
req import --format json --input reqs.json --rename-kind USR=USER
```

## Transformation During Import/Export

### HRID Remapping

Change HRIDs during import:

```bash
req import --format json --input reqs.json --remap-hrids
```

Generates new HRIDs while preserving UUIDs (maintains traceability).

**Use case**: Merging requirement sets with conflicting HRIDs.

### Namespace Addition

Add namespace during import:

```bash
req import --format json --input reqs.json --add-namespace LEGACY
```

Imports USR-001 as LEGACY-USR-001.

**Use case**: Integrating acquired projects.

### Tag Transformation

Add tags during import:
```bash
req import --format json --input reqs.json --add-tag imported --add-tag legacy
```

## Round-Trip Compatibility

**Goal**: Export and re-import without data loss.

**Guaranteed for**:
- JSON format (lossless)
- ReqIF format (best effort)

**Limitations**:
- CSV format (lossy: no complex structures)
- HTML/PDF (read-only export, no import)

**Validation**:
```bash
# Export
req export --format json --output export.json

# Import to clean directory
mkdir test && cd test
req import --format json --input ../export.json

# Compare
diff -r ../original ./
# Should be identical
```

## Use Cases

### Use Case 1: Migrating from Doorstop

**Scenario**: Existing project using Doorstop.

**Goal**: Migrate to Requiem.

**Workflow**:

```bash
# Export from Doorstop (if needed)
doorstop export all doorstop-export.json

# Import to Requiem
mkdir requiem-reqs && cd requiem-reqs
req import --format doorstop --input ../doorstop-project

# Validate
req clean

# Compare manually
# Adjust as needed

# Commit
git init
git add .
git commit -m "Migrate from Doorstop to Requiem"
```

### Use Case 2: Exchanging Requirements with Partners

**Scenario**: Partner uses IBM DOORS; you use Requiem.

**Goal**: Exchange requirements.

**Workflow**:

```bash
# Export from Requiem to ReqIF
req export --format reqif --output requirements.reqif

# Send to partner
# Partner imports into DOORS

# Receive updated ReqIF from partner
# Import updates
req import --format reqif --input updated-requirements.reqif --on-conflict merge

# Review changes
git diff

# Accept or reject
```

### Use Case 3: Spreadsheet Analysis

**Scenario**: Need to analyze requirements in Excel.

**Goal**: Export to CSV, analyze, re-import.

**Workflow**:

```bash
# Export to CSV
req export --format csv --output requirements.csv

# Open in Excel
# Add column "Priority" with values
# Save as requirements-with-priority.csv

# Import with new tags
req import --format csv --input requirements-with-priority.csv \
  --map-column Priority=tag

# Priority values become tags
```

### Use Case 4: Generating Formal Documents

**Scenario**: Need PDF for customer review.

**Goal**: Professional requirement document.

**Workflow**:

```bash
# Export to PDF
req export --format pdf --output requirements.pdf \
  --template formal \
  --include-toc \
  --include-traceability

# Review
open requirements.pdf

# Send to customer
```

## Configuration (Planned)

### Export Settings

```toml
# config.toml (planned)
[export]
# Default export format
default_format = "json"

# Include fingerprints in export
include_fingerprints = true

# Include timestamps
include_timestamps = true

[export.json]
# Pretty-print JSON
pretty = true
indent = 2

[export.csv]
# CSV delimiter
delimiter = ","

# Include header row
include_header = true

[export.pdf]
# Template to use
template = "formal"

# Include table of contents
include_toc = true

# Include traceability matrix
include_traceability = true
```

### Import Settings

```toml
[import]
# Action on UUID conflict
on_conflict = "skip"  # skip, overwrite, rename

# Validate after import
validate = true

# Automatically run req clean after import
auto_clean = true
```

## Troubleshooting

### UUID Conflicts

**Problem**: Importing requirements with UUIDs that already exist.

**Solutions**:
- `--on-conflict skip`: Skip conflicting requirements
- `--on-conflict overwrite`: Replace existing
- `--on-conflict rename`: Generate new UUIDs

### Invalid HRIDs

**Problem**: Imported HRIDs don't match Requiem format.

**Solution**:
```bash
req import --format csv --input reqs.csv --remap-hrids
```

Generates valid HRIDs automatically.

### Broken Links After Import

**Problem**: Parent references don't resolve.

**Diagnosis**:
```bash
req clean
# Reports missing parent requirements
```

**Solution**: Ensure all referenced requirements are imported.

## Workarounds (Until Implemented)

Manual export/import with scripts:

### Export to JSON (Manual)

```python
#!/usr/bin/env python3
import glob
import yaml
import re
import json

requirements = []
for path in glob.glob("*.md"):
    with open(path) as f:
        content = f.read()
    match = re.match(r'^---\n(.*?)\n---\n(.*)$', content, re.DOTALL)
    if match:
        frontmatter = yaml.safe_load(match.group(1))
        body = match.group(2).strip()
        hrid = path.replace('.md', '')
        requirements.append({
            "hrid": hrid,
            "uuid": frontmatter['uuid'],
            "created": frontmatter['created'],
            "content": body,
            "tags": frontmatter.get('tags', []),
            "parents": frontmatter.get('parents', [])
        })

with open("requirements.json", "w") as f:
    json.dump({"requirements": requirements}, f, indent=2, default=str)

print(f"Exported {len(requirements)} requirements to requirements.json")
```

### Import from CSV (Manual)

```python
#!/usr/bin/env python3
import csv
import uuid
from datetime import datetime

with open("requirements.csv") as f:
    reader = csv.DictReader(f)
    for row in reader:
        hrid = row['HRID']
        req_uuid = row.get('UUID', str(uuid.uuid4()))
        content = row['Content']
        tags = row.get('Tags', '').split(',') if row.get('Tags') else []

        frontmatter = f"""---
_version: '1'
uuid: {req_uuid}
created: {datetime.utcnow().isoformat()}Z
"""
        if tags:
            frontmatter += "tags:\n"
            for tag in tags:
                frontmatter += f"- {tag.strip()}\n"

        frontmatter += "---\n\n"

        with open(f"{hrid}.md", "w") as f:
            f.write(frontmatter + content + "\n")

print("Import complete")
```

## Summary

**Planned formats**:
- **Export**: JSON, CSV, ReqIF, HTML, PDF
- **Import**: JSON, CSV, ReqIF, Doorstop

**Use cases**:
- Migration from other tools
- Partner/customer exchange
- Spreadsheet analysis
- Formal document generation
- Backup and archival

**Key features**:
- Selective export/import
- Format transformation
- Conflict resolution
- Round-trip compatibility (JSON, ReqIF)
- Validation

**Timeline**: Implementation planned for future release

## Next Steps

- Use [manual workarounds](#workarounds-until-implemented) for current export/import needs
- Plan your [import/export requirements](#configuration-planned) for when feature is available
- Review [Coverage Reports](./coverage.md) and [Cycle Detection](./cycles.md) for other advanced features
