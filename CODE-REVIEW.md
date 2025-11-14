# Code Review

The project has a solid foundation (domain layer neatly separated from storage/CLI, pedantic lints enabled, and comprehensive tests). While reading through the modules I noted several correctness and usability issues that should be addressed.

## Findings

### 1. `allow_invalid` configuration flag is never honored (High)
- References: `src/domain/config.rs:28-57`, `src/storage/directory.rs:79-93`
- The `Config` struct exposes `allow_invalid`, docs emphasize it, and the CLI prints it, yet the loader never checks it. Every parse error (malformed front matter, duplicate HRID, etc.) still leads to `Directory::new` either failing outright (when `allow_unrecognised` is `false`) or silently discarding the file as "unrecognised". Users therefore cannot temporarily suppress invalid files as advertised, which makes migrations/debugging quite painful.
- Recommendation: propagate the actual `LoadError` from `try_load_requirement` and respect `allow_invalid` separately from `allow_unrecognised`. Invalid requirements should either log a warning while remaining addressable (when `allow_invalid=true`) or cause a dedicated error message that points at the offending file and reason.

### 2. Duplicate HRIDs silently overwrite one another in the tree (High)
- Reference: `src/domain/tree.rs:116-146`
- `Tree::insert` asserts only on UUID collisions. When two requirements share the same HRID, the later insert simply overwrites the entry in `hrid_to_uuid`, leaving both payloads stored but only one reachable via HRID-based lookups. As a result, linking, saving, and CLI commands all operate on an arbitrary requirement and the other copy becomes orphaned with no warning, leading to data loss when flushed.
- Recommendation: treat HRIDs as unique keys. When inserting, check `self.hrid_to_uuid.insert(...)`'s return value and either reject duplicates with a descriptive error (preferably when loading the directory) or support multiple versions explicitly. At minimum log/return an error instead of silently overriding.

### 3. Broken parent references are hidden from `req suspect` (High)
- Reference: `src/domain/tree.rs:471-498`
- `Tree::suspect_links` simply `continue`s when `self.requirements.get(&parent_uuid)` returns `None` (meaning the parent file is missing or failed to load). That means a requirement referencing a deleted/non-existent parent never surfaces anywhere—no suspect link, no diagnostic—and downstream commands assume everything is fine. This hides data corruption and makes it impossible to detect missing parents.
- Recommendation: treat a missing parent node as an error condition. Either emit a separate “dangling link” entry so `req suspect`/`req status` can signal it, or load failures should bubble up during `Directory::new`. Silently swallowing the condition should be avoided.

### 4. `req suspect --group-by none` causes infinite recursion and stack overflow (High)
- References: `src/cli.rs:505-602`, `src/cli.rs:605-694`
- The `GroupBy` enum includes a `None` variant, but `output_table` calls `output_grouped` whenever `self.group_by.is_some()`. Inside `output_grouped` the fallback arm calls `self.output_table(...)` again. Passing `--group-by none` therefore bounces between the two functions until the stack overflows (and any future enum expansion would suffer the same fate).
- Recommendation: normalize `GroupBy::None` to `None` before dispatch (e.g., treat it as “no grouping”) or handle it explicitly in `output_grouped` without recursing. A quick fix is to replace the `_ => ...` arm with a no-op or `return Ok(())`.

### 5. `req list` enforces the default limit even when the user asks for “all” rows (Medium)
- Reference: `src/cli/list.rs:332-345`
- `effective_limit` is always `Some(DEFAULT_LIMIT)` because the logic unconditionally calls `.or(Some(DEFAULT_LIMIT))`. Even with `--limit 0` or `--limit` omitted, the command truncates results to 200 rows and merely prints a “… +N more” footer. There is no supported way to list the full dataset without guessing an arbitrarily large limit, which makes scripted use painful.
- Recommendation: only apply the default when the user did not pass `--limit`. Allow `--limit 0` (or `None`) to mean “unbounded” by leaving `effective_limit` as `None`, and update the footer message accordingly.

---

Addressing the above will make the CLI safer (no silent data loss), closer to the documented behavior, and friendlier for power users who rely on the advanced commands.
