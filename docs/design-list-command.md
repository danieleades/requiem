# `req list` Command Design & Implementation Plan

## Overview

The new `req list` command provides fast visibility into requirements from the terminal. It complements existing modification and validation commands (`add`, `link`, `suspect`, `accept`, `clean`) by letting systems engineers browse, filter, and navigate requirements without leaving the CLI.

## Interface Summary

| Feature | Flag / Behavior | Notes |
|---------|-----------------|-------|
| Base listing | `req list [HRID...]` | Shows requirements as a table with HRID, title, kind, parents, children, tags. Defaults to all requirements (sorted by HRID) capped at the standard limit when no filters are supplied. |
| Output selection | `--columns`, `--sort`, `--quiet`, `--output table|json|csv` | Provides customizable views and machine-readable formats. |
| Filtering | `--kind`, `--namespace`, `--tag`, `--contains`, `--regex`, `--orphans`, `--leaves`, `--limit`, `--offset` | Filters combine conjunctively; repeated flags act as OR. Default limit is 200 unless overridden (`--limit 0` disables). |
| Relationship views | `--view parents|children|ancestors|descendants|tree|context`, `--depth` | Traverse requirement graph and render structured views with a single flag. |

Full normative details live in SYS-017 through SYS-019.

## Implementation Plan

1. **Domain Layer Enhancements**
   - Extend `Directory` to expose query APIs: `iter_requirements()`, `find_by_hrid()`, `children_of()`, `ancestors_of()`, `descendants_of()` with depth control.
   - Add helper structures for computed metadata (parent/child counts, namespace segments, tag lookups).
   - Introduce filter evaluators (kind, namespace, tag, text search using `regex` crate).

2. **CLI Command Wiring**
   - Add `List` variant to `Command` in `src/cli.rs` with Clap definitions for all flags.
   - Parse column/output selections into an internal `ListOptions` struct.
   - Reuse existing logging/root handling consistent with other commands.

3. **Presentation Layer**
   - Implement table rendering (aligned text) and streaming writers for CSV/JSON.
   - Implement tree renderer that respects `--depth` and marks context rows with prefixes.
   - Ensure deterministic ordering (stable sort by HRID unless overridden).

4. **Testing & Validation**
   - Unit tests for new directory queries (graph traversal, filters, pagination).
   - CLI integration tests using fixture repositories to verify flag combinations and output formats.
   - Property-style tests for symmetry between ancestors/descendants and consistent counts.

5. **Documentation & Help**
   - Update `docs/src/reference/cli.md` with command usage and examples.
   - Provide man-page style `req list --help` text (Clap auto-generation).
   - Add release notes and changelog entries once implemented.

## Open Questions

- Should `--tag` create tags automatically if not present or warn when unused?
- Is additional output (e.g., requirement file path) desirable in default columns?
- How should pagination interact with tree output (likely disable or document incompatibility)?
