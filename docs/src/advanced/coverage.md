# Coverage Reports

> **Note**: Coverage reporting is **planned but not yet implemented**. This chapter describes how the feature will work when available.

Coverage reports analyze requirement traceability, identifying gaps in the requirement hierarchy and ensuring all requirements are properly traced.

## What is Coverage?

Coverage measures how completely requirements are traced across levels:

- **Downstream coverage**: Do all user requirements have system requirements?
- **Upstream coverage**: Do all test cases trace to requirements?
- **Bidirectional traceability**: Can you trace from user needs to implementation and tests?

### Example

```
USR-001 ← User requirement
  ├─ SYS-001 ← System requirement (traced)
  └─ SYS-002 ← System requirement (traced)

USR-002 ← User requirement
  └─ (no children) ← Coverage gap!

SYS-003 ← System requirement
  └─ (no parents) ← Orphan requirement!
```

**Coverage report identifies**:
- USR-002 has no system requirements (gap)
- SYS-003 has no parent (orphan)

## Planned Functionality

### Coverage Analysis Command

```bash
req coverage
```

**Output**:
```
Coverage Report

User Requirements (USR):
  Total: 25
  With children: 23 (92%)
  Without children: 2 (8%)
    - USR-002: User data export
    - USR-018: Password recovery

System Requirements (SYS):
  Total: 47
  With parents: 45 (96%)
  With children: 43 (91%)
  Orphans: 2 (4%)
    - SYS-003: Logging service
    - SYS-029: Cache invalidation

Test Cases (TST):
  Total: 156
  With parents: 150 (96%)
  Orphans: 6 (4%)

Overall Coverage: 93%
```

### Coverage by Kind

Analyze specific requirement kinds:

```bash
req coverage --kind USR
```

**Output**:
```
User Requirement Coverage

Total: 25

With children: 23 (92%)
  USR-001 → SYS-001, SYS-002
  USR-003 → SYS-005, SYS-006, SYS-007
  ...

Without children (gaps): 2 (8%)
  USR-002: User data export
  USR-018: Password recovery

Recommendation: Add system requirements for USR-002 and USR-018
```

### Detailed Reports

Generate detailed coverage information:

```bash
req coverage --detailed > coverage-report.md
```

**Output format** (Markdown):
```markdown
# Coverage Report

## Summary
- User Requirements: 92% covered
- System Requirements: 96% traced upstream, 91% traced downstream
- Tests: 96% traced

## Gaps

### User Requirements Without Children
- **USR-002**: User data export
  - Status: No system requirements
  - Action: Create SYS requirements

- **USR-018**: Password recovery
  - Status: No system requirements
  - Action: Create SYS requirements

### Orphan System Requirements
- **SYS-003**: Logging service
  - Status: No parent requirement
  - Action: Link to USR requirement or remove

...
```

### Visual Reports

Generate coverage diagrams:

```bash
req coverage --format html > coverage.html
```

**Features**:
- Interactive traceability matrix
- Heatmap showing coverage density
- Clickable requirement links
- Filterable by kind, tag, or namespace

## Use Cases

### Use Case 1: Requirement Review

**Scenario**: Preparing for a requirements review.

**Goal**: Identify incomplete traceability.

**Workflow**:

```bash
# Generate coverage report
req coverage --detailed > review-report.md

# Review gaps
# - USR-002: No system requirements
# - SYS-003: Orphan requirement

# Fix gaps
req add SYS --parent USR-002  # Add missing SYS requirement
req link SYS-003 USR-007      # Link orphan to parent

# Verify
req coverage
# Coverage improved to 100%
```

### Use Case 2: Release Readiness

**Scenario**: Ensuring all requirements are traced before release.

**Goal**: 100% coverage required for release.

**Workflow**:

```bash
# Check coverage
req coverage --minimum 100

# Output (if < 100%):
# Error: Coverage is 93%, minimum required is 100%
# Gaps:
#   - USR-002: No children
#   - USR-018: No children
#   - SYS-003: No parent

# Fix gaps...

# Verify
req coverage --minimum 100
# Success: Coverage is 100%
```

### Use Case 3: Compliance Audit

**Scenario**: Demonstrating traceability for compliance audit.

**Goal**: Prove all user requirements are traced to implementation and tests.

**Workflow**:

```bash
# Generate audit report
req coverage --audit \
  --trace-from USR \
  --trace-to TST \
  --output audit-report.pdf

# Report shows:
# - Complete traceability chain: USR → SYS → SWR → TST
# - No gaps
# - All requirements covered
```

## Configuration (Planned)

### Coverage Rules

```toml
# config.toml (planned)
[coverage]
# Minimum coverage percentage required
minimum = 95

# Require specific kinds to have children
require_children = ["USR", "SYS"]

# Require specific kinds to have parents
require_parents = ["SYS", "SWR", "TST"]

# Allowed to have no children (leaf requirements)
leaf_kinds = ["TST", "DOC"]

# Allowed to have no parents (root requirements)
root_kinds = ["USR"]
```

### Coverage Thresholds

```toml
[coverage.thresholds]
# Per-kind minimum coverage
USR = 100  # All user requirements must have children
SYS = 95   # 95% of system requirements must have children
SWR = 90   # 90% of software requirements must have children
TST = 100  # All tests must have parents
```

## Report Formats

### Plain Text

Simple text output:

```bash
req coverage
```

### Markdown

Detailed Markdown report:

```bash
req coverage --format markdown > coverage.md
```

### HTML

Interactive HTML dashboard:

```bash
req coverage --format html > coverage.html
```

### JSON

Machine-readable output:

```bash
req coverage --format json > coverage.json
```

**Example JSON**:
```json
{
  "timestamp": "2025-07-22T12:00:00Z",
  "overall_coverage": 93,
  "by_kind": {
    "USR": {
      "total": 25,
      "with_children": 23,
      "without_children": 2,
      "coverage": 92
    },
    "SYS": {
      "total": 47,
      "with_parents": 45,
      "with_children": 43,
      "coverage": 96
    }
  },
  "gaps": [
    {
      "hrid": "USR-002",
      "type": "no_children",
      "description": "User data export"
    }
  ]
}
```

### CSV

Spreadsheet-compatible output:

```bash
req coverage --format csv > coverage.csv
```

## Traceability Matrix

Generate traceability matrices showing parent-child relationships:

```bash
req coverage --matrix
```

**Output** (simplified):

```
         | USR-001 | USR-002 | USR-003
---------|---------|---------|---------
SYS-001  |    X    |         |
SYS-002  |    X    |         |    X
SYS-003  |         |         |
SYS-004  |         |         |    X
```

**Features**:
- Rows: Child requirements
- Columns: Parent requirements
- X: Link exists
- Empty: No link
- Highlights gaps and orphans

## Integration

### CI/CD

Enforce coverage in CI:

```yaml
# .github/workflows/requirements.yml (planned)
- name: Check requirement coverage
  run: |
    req coverage --minimum 95
    if [ $? -ne 0 ]; then
      echo "Coverage below minimum"
      req coverage --detailed
      exit 1
    fi
```

### Pull Request Comments

Automatically comment on PRs with coverage impact:

```yaml
- name: Coverage report
  run: |
    req coverage --format markdown > coverage.md
    gh pr comment ${{ github.event.number }} --body-file coverage.md
```

### Dashboard Integration

Export coverage to dashboards:

```bash
# Export for Grafana/Prometheus
req coverage --format prometheus > metrics.txt
```

## Metrics (Planned)

### Coverage Percentage

```
coverage = (requirements_with_links / total_requirements) * 100
```

### Downstream Coverage

```
downstream_coverage = (parents_with_children / total_parents) * 100
```

### Upstream Coverage

```
upstream_coverage = (children_with_parents / total_children) * 100
```

### Orphan Rate

```
orphan_rate = (requirements_without_parents / total_requirements) * 100
```

## Workarounds (Until Implemented)

Manual coverage analysis:

### Script to Find Gaps

```bash
#!/bin/bash
# find-gaps.sh

echo "USR requirements without SYS children:"
comm -23 \
  <(ls USR-*.md | sed 's/.md//' | sort) \
  <(grep -oh "USR-[0-9]*" SYS-*.md | sort -u)

echo ""
echo "SYS requirements without parents:"
for sys in SYS-*.md; do
    if ! grep -q "parents:" "$sys"; then
        echo "  $(basename "$sys" .md)"
    fi
done
```

### Spreadsheet Analysis

Create a spreadsheet:

| Requirement | Has Parents | Has Children | Coverage |
|-------------|-------------|--------------|----------|
| USR-001     | N/A         | Yes          | ✓        |
| USR-002     | N/A         | No           | ✗        |
| SYS-001     | Yes         | Yes          | ✓        |
| SYS-003     | No          | No           | ✗        |

Track manually until coverage reporting is implemented.

## Summary

**Planned functionality**:

- Coverage percentage by requirement kind
- Gap identification (requirements without parents/children)
- Orphan detection
- Traceability matrices
- Multiple report formats (text, Markdown, HTML, JSON, CSV)
- CI/CD integration
- Configurable coverage thresholds

**Use cases**:
- Requirement reviews
- Release readiness checks
- Compliance audits
- Quality assurance

**Timeline**: Implementation planned for future release

## Next Steps

- Use [workarounds](#workarounds-until-implemented) for manual coverage analysis
- Plan your [coverage requirements](#configuration-planned) for when feature is available
- See [Cycle Detection](./cycles.md) for finding circular dependencies
