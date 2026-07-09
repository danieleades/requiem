//! The `req export` command: generate documentation artifacts from the
//! requirements graph.
//!
//! Requiem does not own the documentation files it exports into — an mdBook
//! `SUMMARY.md` typically mixes hand-written chapters with requirement
//! listings. Exports therefore write into an explicitly marked region of the
//! target file, delimited by `<!-- requiem:summary:start -->` and
//! `<!-- requiem:summary:end -->`, and leave everything outside the markers
//! untouched.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context};
use requiem_core::Directory;

use crate::cli::terminal::Colorize;

/// Marker opening the generated region of a `SUMMARY.md`.
const START_MARKER: &str = "<!-- requiem:summary:start -->";
/// Marker closing the generated region of a `SUMMARY.md`.
const END_MARKER: &str = "<!-- requiem:summary:end -->";

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// Regenerate the requirements section of an mdBook SUMMARY.md
    ///
    /// The section is delimited by `<!-- requiem:summary:start -->` and
    /// `<!-- requiem:summary:end -->` markers; content outside the markers
    /// is left untouched.
    Summary(Summary),
}

impl Command {
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        match self {
            Self::Summary(command) => command.run(root),
        }
    }
}

#[derive(Debug, clap::Parser)]
pub struct Summary {
    /// Path to the SUMMARY.md file (defaults to SUMMARY.md in the
    /// requirements root)
    #[arg(long)]
    file: Option<PathBuf>,

    /// Check that the generated section is up to date without writing
    /// (exits with code 2 if it is stale)
    #[arg(long)]
    check: bool,

    /// Suppress output
    #[arg(long, short)]
    quiet: bool,
}

impl Summary {
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let file = self.file.unwrap_or_else(|| root.join("SUMMARY.md"));
        let directory = Directory::new(root)?;

        let existing = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let summary_dir = file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let entries = collect_entries(&directory, summary_dir)?;
        let updated = splice(&existing, &render_entries(&entries))?;

        if updated == existing {
            if !self.quiet {
                println!(
                    "{}",
                    format!("✅ {} is up to date", file.display()).success()
                );
            }
            return Ok(());
        }

        if self.check {
            if !self.quiet {
                println!(
                    "{}",
                    format!(
                        "⚠️  The generated section of {} is out of date. Run 'req export summary' to update it.",
                        file.display()
                    )
                    .warning()
                );
            }
            std::process::exit(2);
        }

        fs::write(&file, updated).with_context(|| format!("failed to write {}", file.display()))?;

        if !self.quiet {
            println!(
                "{}",
                format!(
                    "✅ Updated {} ({} requirements)",
                    file.display(),
                    entries.len()
                )
                .success()
            );
        }
        Ok(())
    }
}

/// One requirement, ready to render as a summary line.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    /// Namespace segments (empty for un-namespaced requirements).
    namespace: Vec<String>,
    /// Requirement kind, e.g. `USR`.
    kind: String,
    /// Numeric ID within the namespace/kind, used for ordering.
    id: usize,
    /// Full formatted HRID, e.g. `CORE-USR-001`.
    hrid: String,
    /// Requirement title (may be empty).
    title: String,
    /// Link target relative to the SUMMARY.md directory, using `/` separators.
    link: String,
}

/// Collect one [`Entry`] per requirement, sorted by namespace → kind → ID.
fn collect_entries(directory: &Directory, summary_dir: &Path) -> anyhow::Result<Vec<Entry>> {
    let digits = directory.config().digits();
    let summary_dir = std::path::absolute(summary_dir)
        .with_context(|| format!("failed to resolve {}", summary_dir.display()))?;

    let mut entries = Vec::new();
    for requirement in directory.requirements() {
        let hrid = requirement.hrid;
        let path = directory
            .path_for(hrid)
            .map_or_else(|| directory.canonical_path_for(hrid), Path::to_path_buf);
        let path = std::path::absolute(&path)
            .with_context(|| format!("failed to resolve {}", path.display()))?;

        entries.push(Entry {
            namespace: hrid.namespace().iter().map(ToString::to_string).collect(),
            kind: hrid.kind().to_string(),
            id: hrid.id().get(),
            hrid: hrid.display(digits).to_string(),
            title: requirement.title.trim().to_string(),
            link: relative_link(&summary_dir, &path),
        });
    }

    entries.sort();
    Ok(entries)
}

/// Render entries as an mdBook summary fragment: draft chapters group by
/// namespace segment and kind, leaves link to the requirement files.
fn render_entries(entries: &[Entry]) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let mut current_namespace: &[String] = &[];
    let mut current_kind: Option<&str> = None;

    for entry in entries {
        // Emit headers for namespace segments that differ from the previous
        // entry, then for the kind if it changed (or the namespace did).
        let common = current_namespace
            .iter()
            .zip(&entry.namespace)
            .take_while(|(a, b)| a == b)
            .count();
        if common < current_namespace.len() || common < entry.namespace.len() {
            current_kind = None;
        }
        for (depth, segment) in entry.namespace.iter().enumerate().skip(common) {
            let _ = writeln!(out, "{}- [{segment}]()", "  ".repeat(depth));
        }
        current_namespace = &entry.namespace;

        if current_kind != Some(entry.kind.as_str()) {
            let _ = writeln!(
                out,
                "{}- [{}]()",
                "  ".repeat(entry.namespace.len()),
                entry.kind
            );
            current_kind = Some(&entry.kind);
        }

        let label = if entry.title.is_empty() {
            entry.hrid.clone()
        } else {
            format!("{}: {}", entry.hrid, entry.title)
        };
        let _ = writeln!(
            out,
            "{}- [{label}]({})",
            "  ".repeat(entry.namespace.len() + 1),
            entry.link
        );
    }

    out
}

/// Replace the marked region of `existing` with `block`, leaving everything
/// outside the markers untouched.
fn splice(existing: &str, block: &str) -> anyhow::Result<String> {
    let Some(start) = existing.find(START_MARKER) else {
        bail!(
            "no '{START_MARKER}' marker found. Add the following to the file where generated \
             entries should go:\n\n{START_MARKER}\n{END_MARKER}"
        );
    };
    let after_start = start + START_MARKER.len();
    let Some(end_offset) = existing[after_start..].find(END_MARKER) else {
        bail!("found '{START_MARKER}' but no matching '{END_MARKER}' marker after it");
    };
    let end = after_start + end_offset;

    if existing[after_start..].contains(START_MARKER) {
        bail!("multiple '{START_MARKER}' markers found; expected exactly one");
    }
    if existing[end + END_MARKER.len()..].contains(END_MARKER) {
        bail!("multiple '{END_MARKER}' markers found; expected exactly one");
    }

    Ok(format!(
        "{}{START_MARKER}\n{block}{}",
        &existing[..start],
        &existing[end..]
    ))
}

/// Compute a markdown link target for `to`, relative to `from_dir`.
///
/// Both paths must be absolute. `..` components are resolved lexically, and
/// the result always uses `/` separators.
fn relative_link(from_dir: &Path, to: &Path) -> String {
    let from = normalize(from_dir);
    let to = normalize(to);

    let common = from
        .iter()
        .zip(to.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut segments: Vec<String> = std::iter::repeat_n("..".to_string(), from.len() - common)
        .chain(to[common..].iter().cloned())
        .collect();
    if segments.first().is_none_or(|s| s != "..") {
        segments.insert(0, ".".to_string());
    }
    segments.join("/")
}

/// Lexically normalize an absolute path into its component strings,
/// resolving `.` and `..`.
fn normalize(path: &Path) -> Vec<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            Component::Normal(segment) => {
                components.push(segment.to_string_lossy().into_owned());
            }
            Component::RootDir | Component::Prefix(_) => components.clear(),
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(namespace: &[&str], kind: &str, id: usize, title: &str, link: &str) -> Entry {
        let hrid = namespace
            .iter()
            .copied()
            .chain([kind])
            .collect::<Vec<_>>()
            .join("-");
        Entry {
            namespace: namespace.iter().map(ToString::to_string).collect(),
            kind: kind.to_string(),
            id,
            hrid: format!("{hrid}-{id:03}"),
            title: title.to_string(),
            link: link.to_string(),
        }
    }

    #[test]
    fn renders_flat_repository_grouped_by_kind() {
        let entries = vec![
            entry(&[], "SYS", 1, "Auth Service", "./SYS-001.md"),
            entry(&[], "USR", 1, "Login", "./USR-001.md"),
        ];
        let expected = "\
- [SYS]()
  - [SYS-001: Auth Service](./SYS-001.md)
- [USR]()
  - [USR-001: Login](./USR-001.md)
";
        assert_eq!(render_entries(&entries), expected);
    }

    #[test]
    fn renders_namespaces_as_nested_draft_chapters() {
        let entries = vec![
            entry(&["CLI"], "USR", 9, "Lifecycle", "./CLI/USR/009.md"),
            entry(&["CORE"], "SYS", 6, "Sphinx", "./CORE/SYS/006.md"),
            entry(&["CORE"], "SYS", 7, "MdBook", "./CORE/SYS/007.md"),
            entry(&["CORE"], "USR", 5, "SSG Integration", "./CORE/USR/005.md"),
        ];
        let expected = "\
- [CLI]()
  - [USR]()
    - [CLI-USR-009: Lifecycle](./CLI/USR/009.md)
- [CORE]()
  - [SYS]()
    - [CORE-SYS-006: Sphinx](./CORE/SYS/006.md)
    - [CORE-SYS-007: MdBook](./CORE/SYS/007.md)
  - [USR]()
    - [CORE-USR-005: SSG Integration](./CORE/USR/005.md)
";
        assert_eq!(render_entries(&entries), expected);
    }

    #[test]
    fn repeats_kind_header_when_namespace_changes() {
        let entries = vec![
            entry(&["A"], "USR", 1, "One", "./A/USR/001.md"),
            entry(&["B"], "USR", 1, "Two", "./B/USR/001.md"),
        ];
        let expected = "\
- [A]()
  - [USR]()
    - [A-USR-001: One](./A/USR/001.md)
- [B]()
  - [USR]()
    - [B-USR-001: Two](./B/USR/001.md)
";
        assert_eq!(render_entries(&entries), expected);
    }

    #[test]
    fn renders_empty_title_as_bare_hrid() {
        let entries = vec![entry(&[], "DFT", 15, "", "./DFT-015.md")];
        assert_eq!(
            render_entries(&entries),
            "- [DFT]()\n  - [DFT-015](./DFT-015.md)\n"
        );
    }

    #[test]
    fn splice_replaces_only_the_marked_region() {
        let existing = "# Summary\n\n- [Intro](./intro.md)\n\n<!-- requiem:summary:start -->\nstale\n<!-- requiem:summary:end -->\n\n# Reference\n";
        let result = splice(existing, "- [USR]()\n").unwrap();
        assert_eq!(
            result,
            "# Summary\n\n- [Intro](./intro.md)\n\n<!-- requiem:summary:start -->\n- [USR]()\n<!-- requiem:summary:end -->\n\n# Reference\n"
        );
    }

    #[test]
    fn splice_is_idempotent() {
        let existing = "a\n<!-- requiem:summary:start -->\n<!-- requiem:summary:end -->\nb\n";
        let once = splice(existing, "- [X]()\n").unwrap();
        let twice = splice(&once, "- [X]()\n").unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn splice_rejects_missing_or_malformed_markers() {
        assert!(splice("no markers here", "x").is_err());
        assert!(splice("<!-- requiem:summary:start -->", "x").is_err());
        assert!(splice(
            "<!-- requiem:summary:end -->\n<!-- requiem:summary:start -->",
            "x"
        )
        .is_err());
        assert!(splice(
            "<!-- requiem:summary:start -->\n<!-- requiem:summary:start -->\n<!-- requiem:summary:end -->",
            "x"
        )
        .is_err());
        assert!(splice(
            "<!-- requiem:summary:start -->\n<!-- requiem:summary:end -->\n<!-- requiem:summary:end -->",
            "x"
        )
        .is_err());
    }

    #[test]
    fn relative_link_walks_up_and_down() {
        assert_eq!(
            relative_link(
                Path::new("/repo/docs/src"),
                Path::new("/repo/docs/src/requirements/CORE/USR/001.md")
            ),
            "./requirements/CORE/USR/001.md"
        );
        assert_eq!(
            relative_link(
                Path::new("/repo/docs/src/nested"),
                Path::new("/repo/docs/src/USR-001.md")
            ),
            "../USR-001.md"
        );
        assert_eq!(
            relative_link(
                Path::new("/repo/a/./b"),
                Path::new("/repo/a/c/../d/USR-001.md")
            ),
            "../d/USR-001.md"
        );
    }
}
