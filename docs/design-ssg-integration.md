# Design Assessment: Sphinx and mdBook Integration

Status: Phases 0–1 implemented on this branch (`req export summary`, dogfooded on the project book, `--check` wired into CI); Phase 2 (`mdbook-requiem` preprocessor) not started — `mdbook-yml-header` covers frontmatter stripping in the interim
Related: CORE-USR-005, CORE-SYS-006, CORE-SYS-007, CORE-SYS-035, CORE-DFT-016 ([#90](https://github.com/danieleades/requiem/issues/90))

## 1. Summary

The static-site-generator (SSG) integration is currently *passive*: requirement
files happen to be Markdown, so SSGs can render them. No code in the workspace
touches Sphinx or mdBook. The result is that the integration works as a demo
but fails at the two things a requirements tool must add over raw Markdown:
**navigation that tracks the corpus** and **traceability visible in the
rendered output**.

The storage format is *not* the problem. Markdown-with-frontmatter is the
right canonical representation, and inverting it (YAML-canonical, Markdown
generated on demand) would be a regression to Doorstop's architecture — the
approach this project was created to escape. What is missing is a thin
**publish/render layer** on top of the existing format.

Recommendation: keep the storage model, add a `req export`/docs-sync
capability first (cheap, SSG-agnostic, closes DFT-016), then an
`mdbook-requiem` preprocessor for frontmatter stripping, traceability blocks,
and automatic navigation. Treat Sphinx as "compatible + documented recipe"
until there is demand for a real extension.

## 2. Current state

### What exists

- **Format compatibility.** HRID in the first heading (`# USR-001 Title`)
  gives every SSG a sensible page title. MyST (Sphinx) treats YAML
  frontmatter as page metadata and hides it. GitHub renders frontmatter as a
  metadata table. This part of the design works and is validated by
  CORE-SYS-006/007.
- **Two examples** (`examples/mdbook`, `examples/sphinx`) — both trivial
  (2 requirements, flat layout, no namespaces, no traceability).
- **Two docs chapters** (`docs/src/integration/{mdbook,sphinx}.md`) — long,
  but the "advanced" sections (traceability pages, custom preprocessors) are
  described as things *the user could build themselves*, i.e. aspirational.
- **Requirements** CORE-SYS-035 and defect CORE-DFT-016 already record the
  navigation-drift problem.

### Where it falls short (evidence)

1. **The project's own site is the counter-example.** `docs/src/SUMMARY.md`
   lists only two hand-written index pages. The individual requirement files
   are not in `SUMMARY.md`, so mdBook does not render them at all. The index
   pages link `./USR-001.md` etc., but the files moved to `CORE/USR/001.md`
   when namespaces were adopted — **every requirement link on the published
   site is broken**. The hand-maintained summaries have also drifted
   (the page claims 11 user requirements, lists 10, and omits USR-008 and
   USR-012 entirely). This is precisely the failure mode DFT-016 predicts,
   happening in the flagship dogfood.
2. **mdBook renders frontmatter as junk.** mdBook (through 0.5.x) has no
   native frontmatter handling. The documented workaround —
   `{{#include USR-001.md:6:}}` with hand-counted line offsets — is fragile in
   a uniquely bad way for Requiem: adding a parent or updating a fingerprint
   *changes the frontmatter line count*, silently truncating or corrupting
   every transclusion that hard-codes an offset.
3. **Traceability is invisible in the rendered output.** Parents live in
   frontmatter as UUIDs (hidden by MyST, rendered as noise by mdBook);
   children are not materialised anywhere. The published site shows isolated
   pages with no links up or down the graph. CORE-USR-005's acceptance
   criterion "generated documentation maintains requirement traceability
   links" is unmet.
4. **Namespaced layouts made it worse.** With `subfolders_are_namespaces`,
   filenames are bare IDs (`CORE/USR/001.md`), so a page's URL no longer
   contains its HRID and hand-maintained links break whenever requirements
   move. Nothing regenerates them.

### Diagnosis

"Compatible with SSGs" was achieved; "integrated with SSGs" was not. Every
gap above is a *rendering/derivation* problem — none of them is caused by the
storage format, and all the data needed to fix them (HRID↔path mapping,
parent/child graph, suspect links) already exists in `req-core`
(`construct_path_from_hrid`, `Tree`, fingerprints).

## 3. Options

### Option A — Documentation-only tweaks

Fix the broken links by hand, recommend third-party preprocessors
(`mdbook-autosummary` et al.) and MyST glob toctrees.

- Cost: hours.
- Verdict: **insufficient alone.** Hand-fixed indexes drift again (they
  already did once); third-party summary generators know nothing about
  namespaces, kinds, HRIDs, or traceability. Does not close DFT-016.

### Option B — YAML-canonical storage, Markdown generated on demand

Store each requirement as YAML (metadata + body as a block scalar); generate
Markdown pages and index/SUMMARY files at build time.

This is Doorstop's architecture (`.yml` items + `doorstop publish`), and the
instinct behind it is half right: *rendered pages and indexes should be
generated*. But inverting the canonical format buys nothing and costs a lot:

- **It regresses the core value proposition** (CORE-USR-001, README):
  requirements you read and edit as ordinary Markdown in any editor, with
  preview, linting, and soft-wrapping. Prose inside YAML block scalars is
  indentation-sensitive and loses all Markdown tooling.
- **In-repo browsing dies.** GitHub/GitLab render the current files
  beautifully (frontmatter as a metadata table, body as prose). YAML files
  render as configuration.
- **Two artifact sets.** Generated Markdown must either be committed
  (drift, diff noise, conflicts in two places) or gitignored (editor links
  and `{{#include}}` paths break outside builds; every build needs a
  generation step even for `mdbook serve`).
- **It doesn't solve the actual problems.** Index generation and
  traceability rendering must be built *either way* — the generator would
  read the same metadata that frontmatter already carries, via a parser that
  already exists and is well-tested. The storage swap is a large migration
  (storage layer, CLI, MCP server, templates, docs, existing user repos)
  that leaves the hard work untouched.

- Verdict: **rejected.** Keep the diagnosis ("generate the derived stuff"),
  drop the prescription ("change the source of truth").

### Option C — Markdown stays canonical; add a publish/render layer

Three sub-components, independently shippable:

**C1. `req export` (SSG-agnostic derivation, ships first).**
A CLI subcommand (or `req sync --docs`) that regenerates derived doc
artifacts from the graph:

- a `SUMMARY.md` section (or complete file) grouping requirements by
  namespace → kind, titles taken from headings;
- per-kind index pages (replacing the hand-written, already-drifted
  `user-requirements.md` / `system-requirements.md` tables);
- optionally a traceability page (parents/children/suspect links as a
  Markdown table with correct relative links).

Runs locally or in CI (fail-if-dirty check, like `req sync`). Works for
mdBook, Sphinx, Hugo, anything. Closes the substance of DFT-016 / #90.
Estimated effort is small: the graph, HRID→path mapping, and JSON
serialisation all exist; this is a formatting exercise over `Tree`.

**C2. `mdbook-requiem` preprocessor (workspace crate, depends on `req-core`).**
Precedent: `mdbook-autosummary`, `mdbook-fs-summary` show preprocessors can
inject/rewrite chapters. The preprocessor would:

- strip frontmatter from requirement chapters (kills the line-offset hack);
- append a rendered traceability footer to each requirement page — parent
  links, computed child links, suspect-link badges;
- inject requirement chapters into a configured section of the book
  automatically, so `SUMMARY.md` never lists requirements by hand;
- (later) support `{{#req USR-001}}`-style transclusion that embeds a
  requirement body cleanly wherever it's referenced.

This is the "excellent" tier for mdBook and makes Requiem's own site the
showcase. The export command from C1 remains the fallback for users who
can't add preprocessors.

**C3. Sphinx: recipe now, extension only on demand.**
Sphinx/MyST is already 80% there natively: frontmatter is metadata, and a
`:glob:` toctree (as in `examples/sphinx`) auto-includes new requirements.
Short term: document the glob-toctree pattern properly and use
`req export` to generate traceability include-files. A real
`sphinx-requiem` Python extension (HRID roles, `req-toctree` directive,
traceability rendered from frontmatter at build time) is a separate
ecosystem and maintenance commitment — defer until users ask.

### Option D — Requiem becomes its own site generator (`req publish` → HTML)

Rejected: enormous scope, competes with mature SSGs, and contradicts
CORE-USR-005's premise ("integrate with existing plain-text documentation
tools"). Doorstop's built-in publisher is one of its least-loved parts.

## 4. Recommendation

**The current approach is sound; it is the execution that stops at
"compatible". Adopt Option C, phased:**

| Phase | Deliverable | Closes |
|-------|-------------|--------|
| 0 | Fix the project's own docs (regenerate indexes/links, add requirement pages to nav) and add a CI link-check so they cannot silently rot again | broken published site |
| 1 | `req export` for SUMMARY/index/traceability generation; wire into docs CI; update `docs/src/integration/*` to describe the real workflow instead of hand-maintenance | CORE-DFT-016 / #90, CORE-SYS-035 |
| 2 | `mdbook-requiem` preprocessor (frontmatter stripping, traceability footers, nav injection); dogfood on Requiem's own book | CORE-USR-005 acceptance criteria |
| 3 | Sphinx extension — only if demand materialises; until then the glob-toctree recipe + exported includes | CORE-SYS-006 depth |

Also recommended as part of Phase 1: spec the export command as SYS/SPC
requirements under CORE-USR-005 (dogfooding the workflow the tool exists to
support), and reword CORE-SYS-035's implementation notes to name the chosen
mechanism.

## 5. Sizing notes

- Phase 1 needs no new dependencies: `Tree` already computes children and
  suspect links; `construct_path_from_hrid` already maps HRIDs to relative
  paths in both layout modes; `serde`/formatting is in place.
- Phase 2's only new dependency surface is the `mdbook` preprocessor
  API (a JSON-over-stdin protocol; the crate can avoid linking `mdbook`
  itself by speaking the protocol directly, keeping the build light).
- Phase 3 is a Python package with its own release cadence — the main reason
  to defer it.
