# Test Traceability System Design

**Version**: 1.0
**Date**: 2025-10-25
**Status**: Proposed

## Executive Summary

This document describes a comprehensive design for automated requirement-to-test traceability in Requiem. The system automatically discovers tests in source code, creates TEST requirements linked to verified requirements, tracks test execution status, and provides project management visibility into verification coverage throughout the development lifecycle.

**Key Features**:
- Automatic synchronization between code tests and TEST requirements (no manual maintenance)
- Language-agnostic design (initial Rust support, extensible to Python, C++, etc.)
- Suspect link detection for failed tests and changed requirements
- Coverage reporting and traceability matrices
- Release snapshot capabilities for compliance documentation

## Goals and Non-Goals

### Goals
1. **Automatic synchronization**: TEST requirements auto-generated from code annotations
2. **Single source of truth**: Test annotations in code drive requirement generation
3. **Language agnostic**: Support multiple programming languages and test frameworks
4. **Change impact analysis**: Track when requirements change and which tests become suspect
5. **Test status tracking**: Record pass/fail status and link to requirement verification
6. **Project visibility**: Enable PMs to track coverage trends and generate release reports
7. **Compliance ready**: Generate documentation suitable for regulatory requirements

### Non-Goals
1. **Test execution**: This tool does not run tests, only tracks their relationship to requirements
2. **Test framework replacement**: Works alongside existing test frameworks, doesn't replace them
3. **Requirement authoring from tests**: Requirements are still manually written; tests verify existing requirements
4. **Real-time test monitoring**: Status updates require explicit import of test results

## Architecture Overview

### 1. Domain Model Extensions

#### 1.1 Test Metadata Structure

Add test-specific metadata to requirements:

```rust
pub struct TestMetadata {
    /// Location of the test in source code
    /// Format: "path/to/file.rs::test_function_name"
    pub location: String,

    /// Test framework identifier
    /// Examples: "rust-test", "pytest", "junit", "gtest"
    pub framework: String,

    /// Last known test execution status
    pub last_status: TestStatus,

    /// When the test was last executed
    pub last_run: Option<DateTime<Utc>>,

    /// Fingerprint of test status (changes when test starts failing)
    pub status_fingerprint: String,
}

pub enum TestStatus {
    Passed,
    Failed { error: String },
    NotRun,
    Skipped,
}
```

This metadata is stored in TEST requirement frontmatter:

```yaml
---
_version: '1'
uuid: a1b2c3d4-...
created: 2025-10-25T16:00:00Z
parents:
  - uuid: 4bfeb7d5-...  # USR-001
    fingerprint: abc123...
    hrid: USR-001
test_metadata:
  location: "src/domain/requirement.rs::test_requirement_creation"
  framework: "rust-test"
  last_status:
    Passed: null
  last_run: 2025-10-25T16:30:00Z
  status_fingerprint: def456...
---
```

#### 1.2 Extended Suspect Link System

Extend the existing `Parent` structure to include test-specific suspect reasons:

```rust
pub struct Parent {
    pub hrid: Hrid,
    pub fingerprint: String,
    pub suspect_reason: Option<SuspectReason>,
}

pub enum SuspectReason {
    /// Parent requirement content changed (existing)
    ParentChanged,

    /// Test failed during last execution (new)
    TestFailed {
        error: String,
        timestamp: DateTime<Utc>,
    },

    /// Test hasn't been executed recently (new)
    TestStale {
        last_run: DateTime<Utc>,
        threshold: Duration,
    },

    /// Test was skipped (new)
    TestSkipped {
        reason: Option<String>,
    },
}
```

**Suspect Link Logic**:
- For **normal requirements** with TEST children: requirement is suspect if any test fails
- For **TEST requirements** with verified parents: link is suspect if:
  - Parent requirement fingerprint changed (existing behavior)
  - Test execution failed
  - Test hasn't run within configured threshold

### 2. Test Discovery System

#### 2.1 Test Scanner Interface

Language-agnostic interface for discovering tests:

```rust
pub trait TestScanner {
    /// Scan a directory tree for tests
    fn scan(&self, root: &Path) -> Result<Vec<DiscoveredTest>>;

    /// Get the framework identifier
    fn framework_id(&self) -> &str;
}

pub struct DiscoveredTest {
    /// Unique location identifier
    /// Format depends on language: "path/to/file.ext::test_name"
    pub location: String,

    /// Test framework
    pub framework: String,

    /// Description from doc comments
    pub description: String,

    /// Requirements this test verifies (from annotations)
    pub verifies: Vec<Hrid>,

    /// Optional tags
    pub tags: BTreeSet<String>,
}
```

#### 2.2 Rust Test Scanner

Parses Rust source files for test annotations:

**Annotation Formats Supported**:

1. **Doc comment annotations** (Phase 1):
```rust
/// Verifies that requirements can be created from HRID
///
/// @verifies USR-001
/// @verifies SYS-003
#[test]
fn test_requirement_creation() {
    // test code
}
```

2. **Attribute macros** (Phase 2, future):
```rust
#[test]
#[requirement_verifies("USR-001", "SYS-003")]
fn test_requirement_creation() {
    // test code
}
```

**Implementation Approach**:
- Use `syn` crate to parse Rust syntax
- Extract `#[test]` and `#[cfg(test)]` annotated functions
- Parse doc comments for `@verifies HRID` patterns
- Support doctests by parsing doc comments on public items

**Example Scanner Output**:
```rust
DiscoveredTest {
    location: "src/domain/requirement.rs::test_requirement_creation",
    framework: "rust-test",
    description: "Verifies that requirements can be created from HRID",
    verifies: vec![
        Hrid::from_str("USR-001").unwrap(),
        Hrid::from_str("SYS-003").unwrap(),
    ],
    tags: BTreeSet::new(),
}
```

#### 2.3 Generic Scanner (Language-Agnostic)

For languages without dedicated scanner, parse source files as text:

```rust
pub struct GenericScanner {
    file_patterns: Vec<String>,  // e.g., ["**/*.py", "**/*.cpp"]
    annotation_pattern: Regex,   // e.g., r"@verifies\s+([A-Z]+-\d+)"
}
```

Searches for comment patterns like:
- `@verifies USR-001`
- `@requirement USR-001`
- `VERIFIES: USR-001`

#### 2.4 Future Scanner Extensions

Plugin architecture for additional languages:

```rust
pub struct ScannerRegistry {
    scanners: HashMap<String, Box<dyn TestScanner>>,
}

impl ScannerRegistry {
    pub fn register(&mut self, id: String, scanner: Box<dyn TestScanner>);
    pub fn get(&self, id: &str) -> Option<&dyn TestScanner>;
}
```

Planned scanners:
- `PytestScanner` - Python unittest/pytest with decorators
- `GTestScanner` - Google Test C++ macros
- `JUnitScanner` - Java JUnit annotations

### 3. Test Result Import System

#### 3.1 Result Parser Interface

```rust
pub trait TestResultParser {
    /// Parse test results from a file
    fn parse(&self, path: &Path) -> Result<Vec<TestResult>>;

    /// Get the format identifier
    fn format_id(&self) -> &str;
}

pub struct TestResult {
    /// Location matching DiscoveredTest location
    pub location: String,

    /// Test execution status
    pub status: TestStatus,

    /// When the test ran
    pub timestamp: DateTime<Utc>,

    /// Optional additional metadata
    pub metadata: HashMap<String, String>,
}
```

#### 3.2 Cargo Test JSON Parser

Parses output from `cargo test -- --format json`:

```json
{
  "type": "test",
  "event": "ok",
  "name": "requirement::tests::test_requirement_creation",
  "exec_time": 0.001
}
```

Maps to `TestResult`:
- `location`: Extract from `name` field
- `status`: Map `event` (ok → Passed, failed → Failed)
- `timestamp`: Current time (cargo test doesn't include timestamps)

#### 3.3 JUnit XML Parser

Parses standard JUnit XML format (used by pytest, many CI systems):

```xml
<testsuite name="suite" tests="1" failures="0">
  <testcase name="test_requirement_creation"
            classname="domain.test_requirement"
            time="0.001">
  </testcase>
</testsuite>
```

Maps to `TestResult`:
- `location`: Combine `classname` and `name`
- `status`: Check for `<failure>` or `<error>` elements
- `timestamp`: From testsuite `timestamp` attribute if present

### 4. CLI Commands

#### 4.1 Test Sync Command

**Command**: `req test sync [OPTIONS]`

**Purpose**: Discover tests in source code and create/update TEST requirements

**Options**:
- `--test-root <PATH>`: Where to scan for tests (default: `src/`)
- `--framework <ID>`: Test framework (default: from config or auto-detect)
- `--dry-run`: Show what would be created without making changes
- `--force`: Regenerate all TEST requirements from scratch

**Behavior**:
1. Scan source tree using configured scanner
2. For each discovered test:
   - Check if TEST requirement exists (by location)
   - If exists: update description, verify parent links
   - If not exists: create new TEST requirement with next available ID
3. Mark TEST requirements with no matching code as "orphaned"

**Output**:
```
Scanning for tests in src/...
Found 47 tests

TEST-001 ✓ up-to-date (src/domain/requirement.rs::test_creation)
TEST-002 + created (src/domain/hrid.rs::test_hrid_parsing)
TEST-003 ~ updated (src/storage/tree.rs::test_add_requirement)
TEST-004 ⚠ orphaned (no matching test found)

Summary: 1 created, 1 updated, 1 orphaned, 44 unchanged
```

#### 4.2 Test Import Results Command

**Command**: `req test import-results <FILE> [OPTIONS]`

**Purpose**: Import test execution results and update TEST requirement status

**Options**:
- `--format <FORMAT>`: Result format (cargo-json, junit-xml, auto-detect)
- `--timestamp <ISO8601>`: Override timestamp (default: file mtime or current time)
- `--mark-stale`: Mark tests not in results as stale

**Behavior**:
1. Parse test results file using appropriate parser
2. For each result:
   - Find TEST requirement by location
   - Update `test_metadata.last_status`
   - Update `test_metadata.last_run`
   - Recalculate `status_fingerprint`
   - Update suspect status on parent links
3. Optionally mark other tests as stale

**Output**:
```
Importing results from target/test-results.json...

TEST-001 ✓ passed (src/domain/requirement.rs::test_creation)
TEST-002 ✗ failed (src/domain/hrid.rs::test_hrid_parsing)
  Error: assertion failed: hrid.id() == 1
TEST-003 ✓ passed (src/storage/tree.rs::test_add_requirement)

Summary: 45 passed, 2 failed, 0 skipped

Suspect links created: 2
  - SYS-003 ← TEST-002 (test failed)
```

#### 4.3 Test Coverage Command

**Command**: `req test coverage [OPTIONS]`

**Purpose**: Show requirement verification coverage

**Options**:
- `--kind <KIND>`: Filter by requirement kind (USR, SYS, etc.)
- `--untested-only`: Show only requirements without tests
- `--format <FORMAT>`: Output format (text, json, csv, markdown)
- `--threshold <PERCENT>`: Warn if coverage below threshold

**Output (text format)**:
```
Requirement Test Coverage
==========================

USR Requirements: 5 total
  ✓ Tested: 5 (100%)
  ⚠ Suspect: 1 (20%)
  ✗ Failed: 0 (0%)

SYS Requirements: 15 total
  ✓ Tested: 12 (80%)
  ⚠ Suspect: 2 (13%)
  ✗ Failed: 1 (7%)
  ! Untested: 3 (20%)
    - SYS-008 (List suspect links command)
    - SYS-013 (Configuration validation)
    - SYS-014 (Parallel loading)

Overall: 17/20 tested (85%)

Legend:
  ✓ Tested - Has passing tests
  ⚠ Suspect - Tests need re-running (parent changed)
  ✗ Failed - Has failing tests
  ! Untested - No test coverage
```

**Output (JSON format)**:
```json
{
  "timestamp": "2025-10-25T16:00:00Z",
  "summary": {
    "total_requirements": 20,
    "tested": 17,
    "untested": 3,
    "passing": 14,
    "suspect": 2,
    "failing": 1,
    "coverage_percent": 85.0
  },
  "by_kind": {
    "USR": { "total": 5, "tested": 5, "coverage": 100.0 },
    "SYS": { "total": 15, "tested": 12, "coverage": 80.0 }
  },
  "untested_requirements": [
    { "hrid": "SYS-008", "title": "List suspect links command" },
    { "hrid": "SYS-013", "title": "Configuration validation" },
    { "hrid": "SYS-014", "title": "Parallel loading" }
  ]
}
```

#### 4.4 Test Matrix Command

**Command**: `req test matrix [OPTIONS]`

**Purpose**: Generate full traceability matrix

**Options**:
- `--kind <KIND>`: Filter by requirement kind
- `--format <FORMAT>`: Output format (text, markdown, html, csv)
- `--output <FILE>`: Write to file instead of stdout

**Output (text format)**:
```
Requirement Traceability Matrix
================================
Generated: 2025-10-25 16:00:00 UTC

Requirement | Title                    | Tests    | Status      | Last Run
------------|--------------------------|----------|-------------|----------
USR-001     | Plain text storage       | TEST-001 | ✓ Passed    | 2025-10-25
            |                          | TEST-005 | ✓ Passed    | 2025-10-25
USR-002     | HRID format              | TEST-002 | ✗ Failed    | 2025-10-25
SYS-001     | Markdown with frontmatter| TEST-003 | ✓ Passed    | 2025-10-25
SYS-002     | UUID stability           | TEST-004 | ↻ Suspect   | 2025-10-24
SYS-003     | Multiple parents         | (none)   | ! Untested  | N/A

Summary: 4 passed, 1 failed, 1 suspect, 1 untested

Legend:
  ✓ Passed   - All tests passing
  ✗ Failed   - One or more tests failed
  ↻ Suspect  - Requirement changed or test stale
  ! Untested - No test coverage
```

**Output (HTML format)**: Interactive table with:
- Sorting by any column
- Filtering by status
- Expandable test details showing error messages
- Links to requirement and test files

#### 4.5 Test Suspect Command

**Command**: `req test suspect [OPTIONS]`

**Purpose**: Show all suspect test links (failed tests and changed requirements)

**Options**:
- `--kind <KIND>`: Filter by requirement kind
- `--reason <REASON>`: Filter by suspect reason (failed, stale, changed)
- `--exit-code`: Exit with non-zero if suspects found (CI integration)

**Output**:
```
Suspect Test Links
==================

SYS-003: Multiple parents support
  ✗ TEST-002 FAILED (2025-10-25 15:30:00)
    Location: src/domain/requirement.rs::test_multiple_parents
    Error: assertion failed: parents.len() == 2

SYS-002: UUID stability
  ↻ TEST-004 SUSPECT (parent changed)
    Location: src/domain/requirement.rs::test_uuid_stability
    Parent fingerprint changed: abc123... → def456...
    Last run: 2025-10-24 10:00:00 (1 day ago)

USR-003: Parent relationships
  ⏰ TEST-007 STALE (not run recently)
    Location: src/storage/tree.rs::test_parent_links
    Last run: 2025-10-10 09:00:00 (15 days ago)
    Threshold: 7 days

Summary: 3 suspect links (1 failed, 1 changed, 1 stale)
Exit code: 1
```

#### 4.6 Test Validate Command

**Command**: `req test validate [OPTIONS]`

**Purpose**: Verify TEST requirements match actual code

**Options**:
- `--fix`: Automatically fix discrepancies
- `--check`: Exit with non-zero if validation fails (CI mode)

**Behavior**:
1. Scan for tests in code
2. Load TEST requirements
3. Compare:
   - TEST requirements with no matching code (orphaned)
   - Tests with no TEST requirement (undocumented)
   - TEST requirements with wrong parent links
   - TEST requirements with stale descriptions

**Output**:
```
Validating test traceability...

✗ Orphaned TEST requirements (no matching code):
  - TEST-023: src/cli.rs::test_version (file moved?)

⚠ Tests without TEST requirements:
  - src/domain/config.rs::test_default_config
  - src/storage/directory.rs::test_load_error

⚠ Mismatched parent links:
  - TEST-005: Claims to verify USR-002, but code says USR-003

Run 'req test sync' to fix these issues.
Exit code: 1
```

#### 4.7 Test Lookup Command

**Command**: `req test lookup <LOCATION>`

**Purpose**: Reverse lookup - find which requirements a test verifies

**Example**:
```bash
$ req test lookup src/domain/requirement.rs::test_creation

Test: test_creation
Location: src/domain/requirement.rs::test_creation
Framework: rust-test
Status: ✓ Passed (2025-10-25 16:00:00)

Verifies:
  - USR-001: Plain text storage
  - SYS-003: Multiple parents

Tracked by:
  - TEST-001: Created 2025-07-22
```

### 5. Configuration

Extend `config.toml` with test traceability settings:

```toml
_version = "1"
allowed_kinds = ["USR", "SYS", "TEST"]

[test_traceability]
# Enable test traceability features
enabled = true

# Test framework identifier
# Options: "rust-test", "pytest", "junit", "generic"
framework = "rust-test"

# Where to scan for tests
test_root = "src/"

# Where to scan for test results
result_path = "target/test-results.json"

# Result file format
# Options: "cargo-json", "junit-xml", "auto"
result_format = "cargo-json"

# Auto-sync TEST requirements when validating
auto_sync = true

# Mark tests as stale after this many days
stale_threshold_days = 7

# Annotation pattern for generic scanner
annotation_pattern = "@verifies"

# Require all requirements to have tests
require_coverage = false

# Minimum coverage threshold (percent)
coverage_threshold = 80.0

# Generate HTML reports
html_reports = true
html_output_dir = "target/traceability/"
```

### 6. Storage and Persistence

#### 6.1 TEST Requirement Files

TEST requirements are stored as markdown files like other requirements:

**File**: `reqs/TEST-001.md`
```markdown
---
_version: '1'
uuid: a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d
created: 2025-10-25T16:00:00Z
parents:
  - uuid: 4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a  # USR-001
    fingerprint: c8f9e2b1a3d4c5e6f7a8b9c0d1e2f3a4
    hrid: USR-001
test_metadata:
  location: "src/domain/requirement.rs::test_requirement_creation"
  framework: "rust-test"
  last_status:
    Passed: null
  last_run: 2025-10-25T16:30:00Z
  status_fingerprint: abc123def456...
---

# Test: Requirement Creation

Verifies that requirements can be created from HRID and content.

**Test Location**: `src/domain/requirement.rs::test_requirement_creation`

**Framework**: Rust test

**Verified Requirements**:
- USR-001: Plain text storage

**Status**: ✓ Passed

**Last Run**: 2025-10-25 16:30:00 UTC

## Test Description

This test ensures that:
1. A requirement can be created from an HRID string
2. The requirement has a valid UUID
3. The content is stored correctly
```

#### 6.2 Traceability Cache

To improve performance, maintain an in-memory cache:

```rust
pub struct TraceabilityCache {
    /// Map from test location to TEST requirement UUID
    location_to_test: HashMap<String, Uuid>,

    /// Map from requirement UUID to verifying TEST UUIDs
    requirement_to_tests: HashMap<Uuid, Vec<Uuid>>,

    /// Map from TEST requirement UUID to verified requirement UUIDs
    test_to_requirements: HashMap<Uuid, Vec<Uuid>>,

    /// Last sync timestamp
    last_sync: DateTime<Utc>,
}
```

Cache invalidation:
- Rebuild when TEST requirements are added/removed
- Update when test results are imported
- Persist to `.req-cache/traceability.json` for faster startup

## Project Management Lifecycle

### 7. Development Phase Workflow

#### 7.1 Continuous Coverage Monitoring

**Scenario**: PM wants to track verification progress during sprint

**Setup**:
1. Configure CI to run tests and import results
2. Generate coverage report after each build
3. Track coverage trends over time

**CI Integration** (.github/workflows/test.yml):
```yaml
name: Test Coverage

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run tests with JSON output
        run: cargo test -- --format json > test-results.json
        continue-on-error: true

      - name: Import test results
        run: cargo run -- test import-results test-results.json

      - name: Generate coverage report
        run: |
          cargo run -- test coverage --format json > coverage.json
          cargo run -- test coverage --format markdown > coverage.md

      - name: Check coverage threshold
        run: cargo run -- test coverage --threshold 80

      - name: Comment PR with coverage
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const coverage = fs.readFileSync('coverage.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: coverage
            });

      - name: Upload coverage report
        uses: actions/upload-artifact@v3
        with:
          name: coverage-report
          path: |
            coverage.json
            coverage.md
```

**Developer Dashboard** (local):
```bash
# Watch mode - regenerate coverage on file changes
watch -n 10 'cargo test && cargo run -- test import-results target/test-results.json && cargo run -- test coverage'

# Quick status check before committing
cargo run -- test validate && cargo run -- test suspect
```

#### 7.2 Coverage Trend Tracking

**Historical Coverage Database**:

Store coverage snapshots for trend analysis:

**File**: `.req-cache/coverage-history.jsonl`
```jsonl
{"timestamp":"2025-10-01T00:00:00Z","commit":"abc123","total":20,"tested":15,"coverage":75.0}
{"timestamp":"2025-10-08T00:00:00Z","commit":"def456","total":22,"tested":18,"coverage":81.8}
{"timestamp":"2025-10-15T00:00:00Z","commit":"ghi789","total":23,"tested":20,"coverage":87.0}
```

**Command**: `req test trends [OPTIONS]`

**Purpose**: Show coverage trends over time

**Options**:
- `--since <DATE>`: Show trends since date
- `--commits <N>`: Show last N commits
- `--format <FORMAT>`: Output format (text, json, chart)

**Output**:
```
Coverage Trends
===============

Date       | Commit  | Requirements | Tested | Coverage | Change
-----------|---------|--------------|--------|----------|-------
2025-10-01 | abc123  | 20          | 15     | 75.0%    | -
2025-10-08 | def456  | 22          | 18     | 81.8%    | +6.8%
2025-10-15 | ghi789  | 23          | 20     | 87.0%    | +5.2%
2025-10-22 | jkl012  | 25          | 22     | 88.0%    | +1.0%

Trend: ↗ Improving (13% increase over 21 days)
Velocity: +0.62% per day

Projection:
  90% coverage: 2025-10-25 (3 days)
  95% coverage: 2025-11-01 (10 days)
  100% coverage: 2025-11-09 (18 days)
```

**Chart Output** (ASCII):
```
Coverage over time
100% |                                              ▓
 90% |                                          ▓▓▓▓
 80% |                              ▓▓▓▓▓▓▓▓▓▓▓▓
 70% |                  ▓▓▓▓▓▓▓▓▓▓▓▓
 60% |      ▓▓▓▓▓▓▓▓▓▓▓▓
 50% |  ▓▓▓▓
     |___________________________________________________
      Oct 1          Oct 15          Nov 1         Nov 15
```

### 8. Pre-Release Workflow

#### 8.1 Release Readiness Check

**Command**: `req test release-check [OPTIONS]`

**Purpose**: Validate traceability is ready for release

**Options**:
- `--require-coverage <PERCENT>`: Minimum coverage required
- `--allow-suspect`: Allow suspect links (otherwise fail)
- `--allow-untested <KIND>`: Allow untested requirements of specific kinds

**Behavior**:
1. Run validation checks
2. Check for suspect links
3. Verify coverage threshold
4. Generate release checklist

**Output**:
```
Release Readiness Check
=======================
Release: v1.0.0
Date: 2025-10-25

✓ Test synchronization up-to-date
✓ No orphaned TEST requirements
✓ Coverage threshold met (88% ≥ 80%)
✗ Suspect links found (2)
  - SYS-003 ← TEST-002 (test failed)
  - SYS-007 ← TEST-008 (parent changed)
✗ Untested requirements (3)
  - SYS-013: Configuration validation
  - SYS-014: Parallel loading
  - SYS-015: Error recovery

BLOCKERS: 2 failed tests, 3 untested requirements

Release Status: NOT READY

Action Items:
1. Fix failing tests:
   - TEST-002: src/domain/requirement.rs::test_multiple_parents
2. Add test coverage for:
   - SYS-013, SYS-014, SYS-015
3. Re-run release-check when fixed
```

#### 8.2 Release Candidate Process

**Workflow**:

1. **Create release branch**:
```bash
git checkout -b release-v1.0.0
```

2. **Run release readiness check**:
```bash
cargo run -- test release-check --require-coverage 80
```

3. **Fix blockers**:
```bash
# Fix failing tests
cargo test

# Import results
cargo test -- --format json > test-results.json
cargo run -- test import-results test-results.json

# Verify suspect links cleared
cargo run -- test suspect
```

4. **Generate pre-release snapshot**:
```bash
cargo run -- test snapshot --tag v1.0.0-rc1 --output docs/release-v1.0.0-rc1/
```

5. **Review with stakeholders**:
- Share generated traceability matrix
- Review untested requirements
- Approve any exceptions

6. **Final release preparation** (see next section)

### 9. Release Snapshot and Archival

#### 9.1 Snapshot Generation

**Command**: `req test snapshot [OPTIONS]`

**Purpose**: Capture point-in-time traceability state for release documentation

**Options**:
- `--tag <TAG>`: Git tag or version identifier
- `--output <DIR>`: Output directory for snapshot files
- `--format <FORMAT>`: Formats to generate (all, html, pdf, json, markdown)
- `--include-source`: Include full requirement and test source
- `--sign`: Cryptographically sign snapshot for compliance

**Behavior**:
1. Validate current state (no uncommitted changes)
2. Run test sync and import latest results
3. Generate comprehensive reports
4. Create snapshot metadata
5. Optional: Generate cryptographic signature

**Output Structure**:
```
docs/release-v1.0.0/
├── snapshot.json              # Machine-readable snapshot metadata
├── snapshot.json.sig          # Digital signature (if --sign)
├── traceability-matrix.html   # Interactive HTML matrix
├── traceability-matrix.pdf    # PDF for printing/archival
├── coverage-report.html       # Detailed coverage report
├── coverage-report.json       # Machine-readable coverage
├── requirements/              # Full requirement text (if --include-source)
│   ├── USR-001.md
│   ├── SYS-001.md
│   └── TEST-001.md
└── README.md                  # Human-readable summary
```

**snapshot.json** format:
```json
{
  "version": "1.0",
  "release": {
    "tag": "v1.0.0",
    "date": "2025-10-25T18:00:00Z",
    "commit": "abc123def456",
    "branch": "main"
  },
  "coverage": {
    "total_requirements": 25,
    "tested_requirements": 22,
    "coverage_percent": 88.0,
    "by_kind": {
      "USR": { "total": 5, "tested": 5, "coverage": 100.0 },
      "SYS": { "total": 20, "tested": 17, "coverage": 85.0 }
    }
  },
  "test_summary": {
    "total_tests": 47,
    "passing": 47,
    "failing": 0,
    "skipped": 0
  },
  "traceability": [
    {
      "requirement": {
        "hrid": "USR-001",
        "uuid": "4bfeb7d5-d168-44a7-b0f1-e292c1c89b9a",
        "title": "Plain text storage",
        "fingerprint": "c8f9e2b1a3d4c5e6f7a8b9c0d1e2f3a4"
      },
      "tests": [
        {
          "hrid": "TEST-001",
          "uuid": "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d",
          "location": "src/domain/requirement.rs::test_creation",
          "status": "passed",
          "last_run": "2025-10-25T17:30:00Z"
        }
      ]
    }
  ],
  "untested_requirements": [
    {
      "hrid": "SYS-013",
      "uuid": "...",
      "title": "Configuration validation",
      "rationale": "Manual testing sufficient for MVP"
    }
  ],
  "signature": {
    "algorithm": "ed25519",
    "public_key": "...",
    "signature": "..."
  }
}
```

#### 9.2 Release Documentation Generation

**README.md** (auto-generated):
```markdown
# Traceability Report: Release v1.0.0

**Date**: 2025-10-25
**Commit**: abc123def456
**Coverage**: 88% (22/25 requirements)

## Summary

This release includes 25 requirements verified by 47 automated tests.
All tests passed at the time of release.

### Coverage by Type

| Kind | Total | Tested | Coverage |
|------|-------|--------|----------|
| USR  | 5     | 5      | 100%     |
| SYS  | 20    | 17     | 85%      |

### Test Execution

- **Total Tests**: 47
- **Passing**: 47 (100%)
- **Failing**: 0
- **Skipped**: 0

Last test run: 2025-10-25 17:30:00 UTC

### Untested Requirements

The following requirements have no automated test coverage:

- **SYS-013**: Configuration validation
  - Rationale: Manual testing sufficient for MVP
- **SYS-014**: Parallel loading
  - Rationale: Performance verified via benchmarks
- **SYS-015**: Error recovery
  - Planned for v1.1.0

## Compliance Statement

All critical requirements (USR-*) have automated test coverage.
System requirements are 85% covered by automated tests.
This traceability report was generated automatically from source code
and verified at release time.

## Files

- `traceability-matrix.html` - Interactive traceability matrix
- `traceability-matrix.pdf` - Printable traceability matrix
- `coverage-report.html` - Detailed coverage report
- `snapshot.json` - Machine-readable traceability data
- `requirements/` - Full requirement text at release time

## Verification

This snapshot can be cryptographically verified:
```bash
req test verify-snapshot snapshot.json snapshot.json.sig
```

Public key: [key fingerprint]
```

#### 9.3 Release Tagging Integration

**Git Hook** (.git/hooks/pre-push):
```bash
#!/bin/bash
# Check if pushing a release tag
if [[ $ref == refs/tags/v* ]]; then
  echo "Release tag detected: generating snapshot..."

  cargo run -- test release-check --require-coverage 80 || {
    echo "❌ Release check failed"
    exit 1
  }

  TAG=$(basename $ref)
  cargo run -- test snapshot --tag $TAG --output "docs/release-$TAG/" || {
    echo "❌ Snapshot generation failed"
    exit 1
  }

  git add "docs/release-$TAG/"
  git commit --amend --no-edit

  echo "✓ Snapshot generated and committed"
fi
```

**Automated Release Process**:
```bash
# 1. Create release candidate
git checkout -b release-v1.0.0

# 2. Run release checks
cargo run -- test release-check --require-coverage 80

# 3. Generate snapshot
cargo run -- test snapshot --tag v1.0.0 --output docs/release-v1.0.0/

# 4. Commit snapshot
git add docs/release-v1.0.0/
git commit -m "docs: add traceability snapshot for v1.0.0"

# 5. Create tag
git tag -a v1.0.0 -m "Release v1.0.0"

# 6. Push (triggers CI to publish)
git push origin v1.0.0
```

### 10. Post-Release Workflow

#### 10.1 Release Comparison

**Command**: `req test diff-releases <TAG1> <TAG2>`

**Purpose**: Compare traceability between releases

**Output**:
```
Traceability Diff: v0.9.0 → v1.0.0
====================================

Requirements:
  +5 added (USR-006, SYS-016, SYS-017, SYS-018, SYS-019)
  -0 removed
  ~3 modified (SYS-001, SYS-007, USR-003)

Tests:
  +12 added
  -2 removed
  ~5 modified

Coverage:
  v0.9.0: 75% (15/20)
  v1.0.0: 88% (22/25)
  Change: +13% (7 more requirements tested)

Notable changes:
  - All USR requirements now have test coverage
  - SYS-013, SYS-014, SYS-015 remain untested
  - 2 tests removed due to refactoring (TEST-017, TEST-019)
```

#### 10.2 Compliance Reporting

For regulatory/compliance purposes, generate formal reports:

**Command**: `req test compliance-report <TAG> [OPTIONS]`

**Options**:
- `--standard <STANDARD>`: Compliance standard (ISO-26262, DO-178C, etc.)
- `--format <FORMAT>`: Output format (pdf, docx, html)
- `--template <PATH>`: Custom report template

**Output**: Formal document with:
- Executive summary
- Traceability matrix
- Coverage statistics
- Test results
- Change history
- Signatures and approvals

### 11. Ongoing Maintenance

#### 11.1 Test Debt Tracking

Track requirements that need better test coverage:

**Command**: `req test debt [OPTIONS]`

**Options**:
- `--priority`: Order by requirement priority/risk
- `--age`: Show how long requirements have been untested

**Output**:
```
Test Debt Report
================

High Priority Untested:
  SYS-013 - Configuration validation (untested 45 days)
  SYS-015 - Error recovery (untested 38 days)

Medium Priority Untested:
  SYS-014 - Parallel loading (untested 22 days)

Recommendations:
  1. Add tests for SYS-013 (critical path)
  2. Add integration test for SYS-015
  3. Consider manual test procedure for SYS-014
```

#### 11.2 Test Maintenance Alerts

Monitor test health over time:

**Alerts**:
- Tests failing intermittently (flaky tests)
- Tests not run in X days
- Requirements changed but tests not updated
- TEST requirements orphaned (test deleted)

**Command**: `req test health`

**Output**:
```
Test Health Report
==================

⚠ Flaky Tests (passed < 90% in last 10 runs):
  TEST-007: Passes 7/10 times (70%)

⏰ Stale Tests (not run in 14+ days):
  TEST-015: Last run 2025-10-01 (24 days ago)

🔄 Changed Requirements (tests may need updates):
  SYS-003: Changed 2025-10-20, tests last run 2025-10-15

👻 Orphaned Tests:
  TEST-023: No matching test found in code
```

## Implementation Phases

### Phase 1: Core Foundation (Weeks 1-2)

**Goals**: Basic structure and Rust test discovery

**Deliverables**:
1. Add `test_metadata` field to Requirement domain model
2. Implement `TestScanner` trait
3. Implement `RustTestScanner` (doc comment parsing)
4. Implement `req test sync` command
5. Basic TEST requirement creation

**Success Criteria**:
- Can discover Rust tests with `@verifies` annotations
- Can create TEST-XXX.md files automatically
- TEST requirements link to parent requirements

### Phase 2: Status Tracking (Weeks 3-4)

**Goals**: Import test results and track status

**Deliverables**:
1. Implement `TestResultParser` trait
2. Implement `CargoTestJsonParser`
3. Implement `req test import-results` command
4. Extend suspect link system with test failures
5. Add status to TEST requirement metadata

**Success Criteria**:
- Can import cargo test JSON output
- TEST requirements show pass/fail status
- Suspect links created when tests fail

### Phase 3: Reporting (Weeks 5-6)

**Goals**: Visibility and coverage reporting

**Deliverables**:
1. Implement `req test coverage` command
2. Implement `req test matrix` command
3. Implement `req test suspect` command
4. Add JSON/CSV/Markdown output formats
5. Basic HTML report generation

**Success Criteria**:
- Can see which requirements lack tests
- Can generate traceability matrix
- Reports suitable for stakeholder review

### Phase 4: Release Management (Weeks 7-8)

**Goals**: Snapshot and release workflows

**Deliverables**:
1. Implement `req test snapshot` command
2. Implement `req test release-check` command
3. Coverage trend tracking
4. Release documentation generation
5. Snapshot verification

**Success Criteria**:
- Can generate release snapshots
- Snapshots include all traceability data
- Can compare releases

### Phase 5: Language Extensions (Weeks 9-10)

**Goals**: Support beyond Rust

**Deliverables**:
1. Implement `GenericScanner` (comment-based)
2. Add JUnit XML parser
3. Plugin architecture for scanners
4. Documentation for adding languages
5. Example: Python/pytest support

**Success Criteria**:
- Can scan Python files for `@verifies` comments
- Can import pytest results
- Clear path to add more languages

### Phase 6: Polish & Compliance (Weeks 11-12)

**Goals**: Production readiness

**Deliverables**:
1. Implement `req test trends` command
2. Implement `req test debt` command
3. Implement `req test health` command
4. CI/CD integration examples
5. Compliance report templates
6. Comprehensive documentation

**Success Criteria**:
- CI integration examples work
- Documentation complete
- Ready for production use

## Testing Strategy

### Unit Tests

Test each component in isolation:
- `RustTestScanner`: Parse various annotation formats
- `CargoTestJsonParser`: Parse test result JSON
- `TraceabilityCache`: Cache operations
- Requirement with `test_metadata`: Serialization/deserialization

### Integration Tests

Test end-to-end workflows:
- Sync workflow: Scan → Create TEST reqs → Verify parent links
- Import workflow: Run tests → Parse results → Update status
- Coverage workflow: Calculate coverage → Generate reports

### Compliance Tests

Verify correctness:
- All discovered tests have TEST requirements
- All TEST requirements have matching code
- Parent links match `@verifies` annotations
- Fingerprints detect changes correctly

### Performance Tests

Ensure scalability:
- Scan 1000+ test files in <5 seconds
- Import 10000+ test results in <10 seconds
- Generate matrix for 500+ requirements in <30 seconds

## Open Questions

1. **Test granularity**: Should we support multiple test assertions verifying different aspects of one requirement?
   - Proposal: Yes, one test can verify multiple requirements, multiple tests can verify one requirement

2. **Integration tests**: How to handle integration tests that verify multiple requirements?
   - Proposal: Annotate with all verified requirements

3. **Manual tests**: Should we support manual test case documents?
   - Proposal: Phase 7 feature - separate MANUAL-XXX requirements

4. **Test parameters**: How to handle parameterized tests?
   - Proposal: Treat each parameter set as separate test instance

5. **Partial verification**: Can a test partially verify a requirement?
   - Proposal: Add optional `@verifies USR-001[partial]` annotation

6. **Historical test results**: Should we store history beyond "last run"?
   - Proposal: Store last 10 runs for flaky test detection

## Future Enhancements

### Phase 7+: Advanced Features

1. **Manual test integration**: Support for manual test procedures
2. **Test case detail linking**: Link to specific test sections/assertions
3. **Requirement verification levels**: Full vs. partial verification
4. **Risk-based testing**: Prioritize tests based on requirement criticality
5. **Real-time dashboards**: Web UI for live coverage monitoring
6. **Test result timeline**: Historical view of test status over time
7. **AI-powered suggestions**: Suggest which tests to write based on requirements
8. **Coverage heatmaps**: Visual representation of tested vs. untested areas
9. **Multi-repo support**: Track tests across multiple repositories
10. **Requirements change notifications**: Alert when verified requirements change

## Appendix A: Example Scenarios

### Scenario 1: New Feature Development

1. PM creates requirement: `USR-006: Export requirements as JSON`
2. Dev implements feature in `src/export.rs`
3. Dev writes test:
   ```rust
   /// @verifies USR-006
   #[test]
   fn test_json_export() { ... }
   ```
4. CI runs: `req test sync` → Creates TEST-025
5. CI runs tests, imports results
6. PM sees coverage increase: 88% → 92%
7. At sprint review, PM shows passing test for USR-006

### Scenario 2: Bug Fix

1. Bug found: HRID parsing fails on edge case
2. Dev identifies SYS-002 (HRID format requirement)
3. Dev checks: `req test lookup SYS-002`
4. Finds TEST-003 is supposed to verify this
5. Dev improves TEST-003 to catch bug
6. Test fails (as expected)
7. Dev fixes bug in `src/domain/hrid.rs`
8. Test passes
9. `req test import-results` clears suspect link

### Scenario 3: Pre-Release Audit

1. PM runs: `req test release-check --require-coverage 85`
2. Finds 3 untested requirements
3. Team decides: 2 need tests, 1 is OK (benchmarks instead)
4. Devs write 2 new tests
5. PM runs release-check again: passes
6. PM generates snapshot: `req test snapshot --tag v1.0.0`
7. PM reviews HTML matrix with stakeholders
8. Tag created, snapshot archived

### Scenario 4: Post-Release Analysis

1. After v1.0.0 release, v1.1.0 planning begins
2. PM runs: `req test debt`
3. Identifies high-priority untested requirements
4. Creates backlog items for test coverage
5. Team adds tests incrementally
6. PM runs: `req test trends` weekly
7. Observes coverage trending toward 95%
8. At v1.1.0: `req test diff-releases v1.0.0 v1.1.0`
9. Shows +10% coverage improvement in release notes

## Appendix B: Data Model Diagram

```
┌─────────────────────┐
│   Requirement       │
│   (USR/SYS)         │
│                     │
│ - uuid              │
│ - hrid              │
│ - content           │
│ - fingerprint       │
└──────────┬──────────┘
           │
           │ verified by
           ├───────────────────────┐
           │                       │
           ▼                       ▼
┌──────────────────┐    ┌──────────────────┐
│  TEST            │    │  TEST            │
│  Requirement     │    │  Requirement     │
│                  │    │                  │
│ - uuid           │    │ - uuid           │
│ - hrid: TEST-001 │    │ - hrid: TEST-002 │
│ - parents: [USR] │    │ - parents: [USR] │
│ - test_metadata  │    │ - test_metadata  │
│   - location     │    │   - location     │
│   - status       │    │   - status       │
│   - last_run     │    │   - last_run     │
└────────┬─────────┘    └────────┬─────────┘
         │                       │
         │ corresponds to        │
         │ (auto-synced)         │
         ▼                       ▼
┌──────────────────┐    ┌──────────────────┐
│  Test Code       │    │  Test Code       │
│                  │    │                  │
│ /// @verifies    │    │ /// @verifies    │
│ ///   USR-001    │    │ ///   USR-001    │
│ #[test]          │    │ #[test]          │
│ fn test_foo()    │    │ fn test_bar()    │
└──────────────────┘    └──────────────────┘
```

## Appendix C: Configuration Reference

Complete configuration options:

```toml
[test_traceability]
# Core settings
enabled = true
framework = "rust-test"
test_root = "src/"

# Scanning
scan_ignore = ["target/", "vendor/"]
annotation_pattern = "@verifies"
include_doctests = true

# Results
result_path = "target/test-results.json"
result_format = "cargo-json"
store_history = true
history_limit = 10

# Synchronization
auto_sync = true
auto_create_tests = true
warn_orphaned = true

# Status tracking
stale_threshold_days = 7
mark_stale_on_import = true
require_recent_results = false

# Coverage
require_coverage = false
coverage_threshold = 80.0
coverage_by_kind = { USR = 100.0, SYS = 80.0 }

# Reporting
html_reports = true
html_output_dir = "target/traceability/"
html_template = "default"
generate_pdf = false

# Release
snapshot_sign = false
snapshot_include_source = true
snapshot_compress = false

# Advanced
cache_traceability = true
parallel_scan = true
scan_threads = 0  # 0 = auto-detect
```

---

**End of Design Document**
