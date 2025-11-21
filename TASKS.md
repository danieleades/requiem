# CLI Redesign Implementation Roadmap

**Version:** 1.0
**Date:** 2025-11-20
**Status:** Planning

---

## Executive Summary

This document outlines the implementation roadmap for redesigning the `req` CLI based on a synthesis of multiple design documents. The redesign addresses naming clarity, completeness of lifecycle operations, and user experience consistency while maintaining backward compatibility through a phased migration approach.

**Key Goals:**
- Replace confusing command names (`clean` → `sync`, `add` → `create`)
- Add missing lifecycle operations (`delete`, `unlink`, `move`, `rename`)
- Consolidate scattered validation (`status`/`suspect`/`diagnose` → `validate`/`review`)
- Implement consistent patterns across all commands
- Clean break- no need to maintain backwards compatibility since we are in prototype phase

**Timeline:** 4-6 months (phased implementation with testing)

---

## Design Synthesis

### Best Elements from Each Design Document

**From CLI-DESIGN.md:**
- ✅ Lifecycle-oriented command structure
- ✅ Verb-first naming principle
- ✅ Flat command hierarchy (no unnecessary nesting)
- ✅ Consistent cross-cutting patterns (--dry-run, --yes, --output)
- ✅ Complete command reference with examples

**From CLI-DESIGN-ADDENDUM.md:**
- ✅ `kind` command for explicit kind management
- ✅ Minimal `init` with no defaults (maximum flexibility)
- ✅ Smart cascade delete (only orphaned descendants)
- ✅ `sync --what` flag for parents/paths/all
- ✅ Non-interactive review by default

**From CLI-GAP-ANALYSIS.md:**
- ✅ Phase-based implementation roadmap
- ✅ Breaking changes analysis with migration strategy
- ✅ Effort estimates and priority matrix
- ✅ Success metrics

**From CLI-REVIEW.md:**
- ⚠️ Noun-based grouping rejected (conflicts with "flat over hierarchical")
- ✅ Good workflow walkthrough examples

---

## Final Command Set (15 Commands)

| # | Command | Purpose | Status | Priority |
|---|---------|---------|--------|----------|
| 1 | `init` | Initialize repository | Enhance | Medium |
| 2 | `kind` | Manage requirement kinds | **New** | Medium |
| 3 | `create` | Create requirement | Rename (from `add`) | Medium |
| 4 | `delete` | Delete requirement | **New** | High |
| 5 | `link` | Link child to parent | Keep | - |
| 6 | `unlink` | Remove link | **New** | High |
| 7 | `validate` | Check integrity | Consolidate | High |
| 8 | `review` | Review suspect links | Rename + Enhance | High |
| 9 | `move` | Move requirement | **New** | Medium |
| 10 | `rename` | Rename HRID | **New** | Medium |
| 11 | `sync` | Synchronize metadata | Rename + Enhance | High |
| 12 | `list` | List/query requirements | Keep + Enhance | Low |
| 13 | `show` | Show details | **New** | Medium |
| 14 | `graph` | Visualize graph | **New** (future) | Low |
| 15 | `config` | Manage configuration | Keep + Enhance | Low |

---

## Implementation Phases

### Phase 1: Critical Fixes (4-6 weeks) [HIGH PRIORITY]

**Goal:** Fix the most confusing/dangerous aspects of current CLI

**Tasks:**

1. **Rename `clean` → `sync` with enhanced functionality** [5 days]
   - **Requirements:** SYS-021 (HRID Correction), SYS-030 (Sync Enhancement)
   - Add `--what` flag (parents/paths/all, default: parents)
   - Implement path synchronization logic
   - Add `--check` flag for CI usage
   - Create deprecation alias for `clean`
   - **Files:** `src/cli.rs`, `src/cli/sync.rs` (rename from clean.rs), `src/storage/directory.rs`

2. **Add `delete` command with smart cascade** [5 days]
   - **Requirements:** SYS-024 (Delete Requirement Command) → USR-009
   - Implement basic delete with child checking
   - Add `--cascade` with orphan-aware logic
   - Add `--orphan` flag to unlink children
   - Add `--dry-run` and `--yes` flags
   - **Files:** `src/cli/delete.rs`, `src/domain/tree.rs`

3. **Add `unlink` command** [2 days]
   - **Requirements:** SYS-027 (Unlink Command) → USR-009
   - Implement link removal
   - Add confirmation prompts
   - Mirror `link` command patterns
   - **Files:** `src/cli/unlink.rs`, `src/domain/tree.rs`

4. **Consolidate `accept` into `review --accept`** [4 days]
   - **Requirements:** SYS-008 (Review Suspect Links), SYS-009, SYS-010 → USR-004
   - **Specification:** SPC-001 (Suspect Link Remediation) - needs update
   - Add `--accept` flag to `review` (rename from `suspect`)
   - Move accept logic into review command
   - Make review non-interactive by default
   - Create deprecation alias for `accept`
   - Update SPC-001 to reflect new command name
   - **Files:** `src/cli/review.rs` (rename from suspect.rs), `src/cli/accept.rs`, `docs/src/requirements/SPC-001.md`

5. **Add path fixing to `sync --what paths`** [3 days]
   - **Requirements:** SYS-030 (Sync Enhancement), SYS-023 (Path Diagnostics)
   - Implement canonical path calculation
   - Add file moving logic
   - Add confirmation prompts
   - **Files:** `src/storage/directory.rs`, `src/cli/sync.rs`

**Deliverables:**
- 3 new commands (delete, unlink, sync-enhanced)
- 2 deprecated commands with aliases
- Updated tests
- Updated documentation

**Success Criteria:**
- All tests pass
- No regression in existing functionality
- Deprecation warnings clear and actionable

---

### Phase 2: Consolidation (3-4 weeks) [HIGH PRIORITY]

**Goal:** Unify scattered validation and improve naming

**Tasks:**

1. **Create unified `validate` command** [10 days]
   - **Requirements:** SYS-028 (Validate Command) → USR-010
   - **Specification:** Create SPC-006 for detailed validation checks and output formats
   - Design modular check system
   - Implement `--check` flag (paths/links/suspect/structure/all)
   - Absorb `status` checking features (SYS-020)
   - Absorb `diagnose paths` checking (SYS-023)
   - Absorb `suspect` link checking (SYS-008)
   - Add `--fix` flag for auto-repair
   - Add `--output` flag (table/json/summary)
   - **Files:** `src/cli/validate.rs`, refactor status/diagnose/suspect, `docs/src/requirements/SPC-006.md`

2. **Rename `suspect` → `review`** [2 days]
   - **Requirements:** SYS-008 (Review Suspect Links)
   - **Specification:** SPC-001 (update for new command name)
   - Update command name
   - Create alias for `suspect`
   - Update help text
   - Update SPC-001 references from `suspect` to `review`
   - **Files:** `src/cli.rs`, `docs/src/requirements/SPC-001.md`

3. **Handle deprecations** [2 days]
   - Add deprecation warnings to old commands
   - Update documentation
   - **Files:** `src/cli.rs`, README.md, docs/

**Deliverables:**
- Unified `validate` command
- Clearer command naming
- Deprecation path for old commands

**Success Criteria:**
- Single source of truth for repository health
- Clear separation: validate checks, review handles suspect links
- All validation tests pass

---

### Phase 3: Convenience Features (3-4 weeks) [MEDIUM PRIORITY]

**Goal:** Fill remaining gaps, improve workflows

**Tasks:**

1. **Rename `add` → `create`** [1 day]
   - **Requirements:** SYS-022 (Create Requirement Command) → USR-001, USR-006
   - Update command name
   - Create alias for `add`
   - **Files:** `src/cli.rs`

2. **Add `kind` management commands** [3 days]
   - **Requirements:** SYS-029 (Kind Management Command) → USR-011
   - Implement `kind add <KIND>...`
   - Implement `kind remove <KIND>...`
   - Implement `kind list`
   - Update config management
   - **Files:** `src/cli/kind.rs`, `src/domain/config.rs`

3. **Enhance `init` command** [3 days]
   - Remove default kinds (aligns with maximum flexibility principle)
   - Add `--kinds` flag
   - Update templates creation
   - **Files:** `src/cli/init.rs`

4. **Add `move` command** [5 days]
   - **Requirements:** SYS-025 (Move Requirement Command) → USR-009
   - Implement file moving with HRID updates
   - Add `--sync` flag
   - Handle namespace changes
   - **Files:** `src/cli/move.rs`, `src/domain/tree.rs`

5. **Add `rename` command** [5 days]
   - **Requirements:** SYS-026 (Rename Requirement Command) → USR-009
   - Implement HRID renaming
   - Update markdown heading
   - Add `--sync` flag for children
   - **Files:** `src/cli/rename.rs`, `src/domain/tree.rs`

6. **Add `show` command** [4 days]
   - **Requirements:** SYS-031 (Show Command) → USR-007
   - Implement detailed single-requirement view
   - Add `--edit` flag
   - Add `--output` formats (pretty/json/markdown/raw)
   - Add `--with-content` flag
   - **Files:** `src/cli/show.rs`

**Deliverables:**
- Complete lifecycle management (create, delete, move, rename)
- Intuitive verb-based naming
- Kind management tools

**Success Criteria:**
- No manual file editing required for common operations
- Consistent command patterns
- Clear help text

---

### Phase 4: Polish & Future (1-2 weeks) [LOW PRIORITY]

**Goal:** Cleanup and advanced features

**Tasks:**

1. **Remove `check` alias** [1 day]
   - Remove command
   - Update documentation
   - **Files:** `src/cli.rs`

2. **Enhance `list` with more views** [2 days]
   - **Requirements:** SYS-017, SYS-018, SYS-019 → USR-007
   - **Specification:** SPC-002 (already exists)
   - Add additional view modes
   - Improve tree rendering
   - **Files:** `src/cli/list.rs`

3. **Improve `config` help text** [1 day]
   - Better documentation
   - Add `get` subcommand
   - **Files:** `src/cli/config.rs`

4. **Add `graph` visualization** [FUTURE]
   - **Requirements:** SYS-032 (Graph Visualization Command) → USR-007
   - DOT format export
   - ASCII tree visualization
   - SVG generation (optional)
   - **Files:** `src/cli/graph.rs`

**Deliverables:**
- Cleanup and consistency
- Foundation for future features

---

## Breaking Changes & Migration Strategy

### Breaking Changes

| Change | Impact | Mitigation | Timeline |
|--------|--------|------------|----------|
| `clean` → `sync` | HIGH | Alias with deprecation warning | 2-3 releases |
| `accept` → `review --accept` | HIGH | Alias forwarding | 2-3 releases |
| `add` → `create` | LOW | Alias | 1-2 releases |
| `suspect` → `review` | MEDIUM | Alias or dual names | 2-3 releases |
| `status` → `validate` | MEDIUM | Keep `status` as subset | 2-3 releases |
| `diagnose` removed | LOW | Forward to `validate` or `sync` | 1-2 releases |
| `check` removed | LOW | Use `review` or `suspect` | 1 release |
| `init` defaults removed | LOW | Document, provide --kinds flag | 1 release |

### Migration Timeline

**Release X.0 (Month 1-3): Add New Commands + Aliases**
- New commands work
- Old commands work with warnings
- Documentation updated

**Release X.1 (Month 4): Deprecation Period**
- Warnings become more prominent
- Migration guide published
- Examples updated

**Release X+1.0 (Month 5-6): Remove Aliases (Major Version)**
- Remove deprecated commands
- Update --help text
- Release notes document breaking changes

---

## Requirements Documentation Updates

### New User Requirements Needed

1. **USR-009: Command Line Interface Lifecycle Management**
   - Statement: Users shall be able to manage the complete lifecycle of requirements (create, delete, move, rename) through the CLI without manual file editing
   - Children: SYS-024 (delete), SYS-025 (move), SYS-026 (rename), SYS-027 (unlink)

2. **USR-010: Repository Validation and Health**
   - Statement: Users shall be able to validate repository health and integrity through a unified command
   - Children: SYS-028 (validate)

3. **USR-011: Requirement Kind Management**
   - Statement: Users shall be able to manage requirement kinds through the CLI
   - Children: SYS-029 (kind)

### New System Requirements Needed

| ID | Title | Parent | Priority |
|----|-------|--------|----------|
| SYS-024 | Delete Requirement Command | USR-009 | High |
| SYS-025 | Move Requirement Command | USR-009 | Medium |
| SYS-026 | Rename Requirement Command | USR-009 | Medium |
| SYS-027 | Unlink Command | USR-009 | High |
| SYS-028 | Validate Command | USR-010 | High |
| SYS-029 | Kind Management Command | USR-011 | Medium |
| SYS-030 | Sync Command Enhancement | - | High |
| SYS-031 | Show Command | USR-007 | Medium |
| SYS-032 | Graph Visualization Command | USR-007 | Low |

### Requirements to Update

| ID | Current | Update Needed |
|----|---------|---------------|
| SYS-002 | UUID/HRID fields | Add duplicate HRID detection clause |
| SYS-008 | Suspect command | Add missing parent detection, rename to review |
| SYS-009 | Accept individual | Merge into review --accept |
| SYS-010 | Accept all | Merge into review --accept --all |
| SYS-017 | List command | Document --limit 0 for unlimited |
| SYS-020 | Status command | Note consolidation into validate |
| SYS-021 | Clean command | Rename to sync, add --what flag |
| SYS-022 | Add command | Rename to create, add duplicate handling |
| SYS-023 | Diagnose paths | Merge into validate/sync |

### Specifications to Update

| ID | Current | Update Needed |
|----|---------|---------------|
| SPC-001 | Suspect/accept workflow | Update command name from `suspect` to `review`, merge accept functionality |
| SPC-002 | List command spec | Add --limit 0 semantics (already detailed) |
| SPC-003 | Status dashboard | Note relationship to validate command |

### New Specifications Needed

Based on existing specification patterns (detailed CLI UX guidance for complex commands), **only one new specification is recommended:**

| ID | Title | Purpose | Priority | Parent Requirements |
|----|-------|---------|----------|---------------------|
| **SPC-006** | **Validate Command Specification** | Detailed UX for unified validation: check types, output formats, --fix behavior, error reporting patterns | **HIGH** | SYS-028 → USR-010 |

**Rationale for SPC-006:**
- Validates consolidates 3 existing commands (status, suspect, diagnose)
- Multiple check types (paths, links, suspect, structure)
- Multiple output modes (table, json, summary)
- Auto-fix capability requires careful UX design
- Exit codes and error messages need standardization
- Follows same pattern as SPC-001 (complex CLI workflow) and SPC-002 (multiple modes)

**Specifications NOT needed:**
- SYS-024 (delete): Straightforward CRUD, confirmation patterns are standard
- SYS-025/026 (move/rename): Simple operations, requirements are sufficient
- SYS-027 (unlink): Very simple inverse of link
- SYS-029 (kind): Simple CRUD operations
- SYS-030 (sync): Enhancement to existing command, --what flag is self-explanatory
- SYS-031 (show): Multiple outputs but straightforward, requirement is detailed enough
- SYS-032 (graph): Visualization is future work, can defer spec until implementation

---

## Testing Strategy

### Unit Tests
- All new commands have comprehensive unit tests
- Test all flags and combinations
- Test error conditions

### Integration Tests
- Workflow tests (create → link → review → accept)
- Migration tests (old commands → new commands)
- Backward compatibility tests for aliases

### Regression Tests
- Ensure no existing functionality breaks
- Validate all current tests still pass

### Documentation Tests
- Examples in help text are accurate
- README examples work
- Documentation examples are up-to-date

---

## Success Metrics

### Quantitative

| Metric | Current | Target |
|--------|---------|--------|
| Commands with clear names | 7/11 (64%) | 15/15 (100%) |
| Complete CRUD operations | 2/4 (50%) | 4/4 (100%) |
| Validation commands | 3 | 1 |
| Test coverage | ~85% | >90% |
| Command count | 11 | 15 |

### Qualitative

- **Discoverability:** Can users find the right command via `--help`?
- **Predictability:** Do flags work the same way across commands?
- **Safety:** Are destructive operations clearly marked and protected?
- **Completeness:** Can users accomplish all tasks without manual file editing?

---

## Risks & Mitigation

### Risk 1: Breaking Changes Disrupt Users
**Likelihood:** High
**Impact:** High
**Mitigation:**
- Gradual deprecation over 2-3 releases
- Clear migration guide
- Aliases maintain backward compatibility
- Comprehensive changelog

### Risk 2: Implementation Takes Longer Than Estimated
**Likelihood:** Medium
**Impact:** Medium
**Mitigation:**
- Phased approach allows partial delivery
- Each phase delivers value independently
- Buffer time included in estimates

### Risk 3: New Commands Have Bugs
**Likelihood:** Medium
**Impact:** High
**Mitigation:**
- Comprehensive testing strategy
- Beta period for testing
- Conservative rollout

### Risk 4: Documentation Becomes Out of Sync
**Likelihood:** Medium
**Impact:** Medium
**Mitigation:**
- Update docs in same PR as code
- Documentation tests
- Examples in help text

---

## Next Steps

### Immediate (This Week)
1. ✅ Create this TASKS.md roadmap
2. Create new user requirements (USR-009, USR-010, USR-011)
3. Create new system requirements (SYS-024 through SYS-032)
4. Update existing requirements

### Short Term (Next 2 Weeks)
1. Begin Phase 1 implementation
2. Start with `sync` rename and enhancement
3. Implement `delete` command
4. Set up deprecation infrastructure

### Medium Term (Next 1-2 Months)
1. Complete Phase 1
2. Begin Phase 2 (validation consolidation)
3. Update all documentation
4. Publish beta release

### Long Term (Next 3-6 Months)
1. Complete all phases
2. Deprecation period
3. Major version release with breaking changes removed
4. Close out migration

---

## Dependencies

### External
- None (all internal Rust implementation)

### Internal
- Existing domain model (tree, requirement, HRID)
- Storage layer (directory, markdown)
- CLI framework (clap)

---

## Appendix A: Command Quick Reference

### Current → Proposed Mapping

```bash
# Initialization
req init                      # Enhanced (no defaults, add --kinds)
                             # NEW: req kind add/remove/list

# Creation & Deletion
req add                       → req create
                             # NEW: req delete (with --cascade, --orphan)

# Relationships
req link                      # Keep as-is
                             # NEW: req unlink

# Review & Validation
req status                    → req validate (consolidate)
req suspect                   → req review
req accept                    → req review --accept
req diagnose paths            → req validate --check paths / req sync --what paths

# Maintenance
req clean                     → req sync (add --what flag)
                             # NEW: req move
                             # NEW: req rename

# Querying & Analysis
req list                      # Keep + enhance
                             # NEW: req show
                             # NEW: req graph (future)

# Configuration
req config                    # Keep + enhance
```

---

## Appendix B: Effort Estimates Summary

| Phase | Duration | Effort (Person-Days) |
|-------|----------|---------------------|
| Phase 1: Critical Fixes | 4-6 weeks | 25-30 days |
| Phase 2: Consolidation | 3-4 weeks | 20-25 days |
| Phase 3: Convenience | 3-4 weeks | 25-30 days |
| Phase 4: Polish | 1-2 weeks | 5-10 days |
| **Total** | **11-16 weeks** | **75-95 days** |

With buffer for testing, documentation, and bug fixes: **16-24 weeks (4-6 months)**

---

## Appendix C: Requirements Traceability Matrix

### User Requirements (3 new)

| ID | Title | Status | Children |
|----|-------|--------|----------|
| USR-009 | Command Line Interface Lifecycle Management | ✅ Created | SYS-024, SYS-025, SYS-026, SYS-027 |
| USR-010 | Repository Validation and Health | ✅ Created | SYS-028 |
| USR-011 | Requirement Kind Management | ✅ Created | SYS-029 |

### System Requirements (9 new)

| ID | Title | Parent | Status |
|----|-------|--------|--------|
| SYS-024 | Delete Requirement Command | USR-009 | ✅ Created |
| SYS-025 | Move Requirement Command | USR-009 | ✅ Created |
| SYS-026 | Rename Requirement Command | USR-009 | ✅ Created |
| SYS-027 | Unlink Command | USR-009 | ✅ Created |
| SYS-028 | Validate Command | USR-010 | ✅ Created |
| SYS-029 | Kind Management Command | USR-011 | ✅ Created |
| SYS-030 | Sync Command Enhancement | - | ✅ Created |
| SYS-031 | Show Command | USR-007 | ✅ Created |
| SYS-032 | Graph Visualization Command | USR-007 | ✅ Created |

### System Requirements Updated (5)

| ID | Title | Updates |
|----|-------|---------|
| SYS-002 | UUID and HRID Fields | ✅ Added duplicate HRID detection |
| SYS-008 | Review Suspect Links | ✅ Renamed from suspect, added missing parent detection, merged accept |
| SYS-021 | HRID Correction (Sync) | ✅ Renamed from clean, noted --what enhancement |
| SYS-022 | Create Requirement | ✅ Renamed from add, added duplicate handling |
| SYS-023 | Path Diagnostics | ✅ Noted consolidation into validate/sync |

### Specifications

| ID | Title | Status | Parent | Notes |
|----|-------|--------|--------|-------|
| SPC-001 | Suspect Link Remediation | 🔄 Needs Update | SYS-008, SYS-009, SYS-010 | Update `suspect` → `review` |
| SPC-002 | List Command Experience | ✅ Current | SYS-017, SYS-018, SYS-019 | Add --limit 0 note |
| SPC-003 | Status Dashboard | 🔄 Needs Update | SYS-020 | Note relationship to validate |
| SPC-006 | Validate Command | 📝 **NEW** | SYS-028 | HIGH priority - consolidates 3 commands |

**Total Requirements:**
- User: 11 (was 8, +3 new)
- System: 29 (was 20, +9 new)
- Specifications: 6 (was 5, +1 new)

---

**Document Owner:** Development Team
**Last Updated:** 2025-11-20
**Status:** Ready for Implementation

**Requirements Files:**
- User: `docs/src/requirements/USR-{001-011}.md`
- System: `docs/src/requirements/SYS-{001-032}.md`
- Specifications: `docs/src/requirements/SPC-{001-006}.md`
- Index: `docs/src/requirements/user-requirements.md`, `system-requirements.md`