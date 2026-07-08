//! Rendering of list rows as table, JSON, CSV, or tree output.

use std::collections::HashMap;

use anyhow::Context;
use clap::ValueEnum;
use serde::Serialize;

use super::{
    entry::Entry,
    filter::Filters,
    row::{Direction, Row},
    OutputFormat,
};

/// Available table columns.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default, ValueEnum)]
pub(super) enum ListColumn {
    #[default]
    Hrid,
    Title,
    Kind,
    Namespace,
    Parents,
    Children,
    Tags,
    Path,
    Created,
}

impl ListColumn {
    const fn header(self) -> &'static str {
        match self {
            Self::Hrid => "HRID",
            Self::Title => "Title",
            Self::Kind => "Kind",
            Self::Namespace => "Namespace",
            Self::Parents => "Parents",
            Self::Children => "Children",
            Self::Tags => "Tags",
            Self::Path => "Path",
            Self::Created => "Created",
        }
    }

    fn value(self, entry: &Entry, digits: usize) -> String {
        match self {
            Self::Hrid => entry.hrid.display(digits).to_string(),
            Self::Title => entry.title.clone().unwrap_or_default(),
            Self::Kind => entry.hrid.kind().to_string(),
            Self::Namespace => entry.hrid.namespace().join("-"),
            Self::Parents => entry
                .parents
                .iter()
                .map(|link| link.hrid.display(digits).to_string())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Children => entry
                .children
                .iter()
                .map(|link| link.hrid.display(digits).to_string())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Tags => entry.tags.join(", "),
            Self::Path => entry.path.display().to_string(),
            Self::Created => entry.created.to_rfc3339(),
        }
    }
}

/// A row of output serialized for the JSON format.
#[derive(Debug, Clone, Serialize)]
struct SerializableRow<'a> {
    hrid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_rows(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    output: OutputFormat,
    quiet: bool,
    tree: bool,
    ascii: bool,
    filters: &Filters,
    digits: usize,
) -> anyhow::Result<()> {
    if tree {
        render_tree(&rows, entries, ascii, digits);
        return Ok(());
    }

    match output {
        OutputFormat::Table => {
            render_table(rows, entries, columns, quiet, filters, digits);
            Ok(())
        }
        OutputFormat::Json => render_json(rows, entries, columns, digits),
        OutputFormat::Csv => {
            render_csv(rows, entries, columns, quiet, digits);
            Ok(())
        }
    }
}

fn render_tree(rows: &[Row], entries: &[Entry], ascii: bool, digits: usize) {
    if rows.is_empty() {
        return;
    }

    // Build parent-child relationships
    let mut parent_map: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut parent_stack: Vec<usize> = Vec::new();

    for (idx, row) in rows.iter().enumerate() {
        // Adjust stack to current depth
        while parent_stack.len() > row.depth {
            parent_stack.pop();
        }

        // If we have a parent, record this relationship
        if let Some(&parent_idx) = parent_stack.last() {
            parent_map.entry(parent_idx).or_default().push(idx);
        }

        // This row becomes a potential parent for deeper rows
        parent_stack.push(idx);
    }

    // Now render with proper tree structure
    for (idx, row) in rows.iter().enumerate() {
        let entry = &entries[row.index];

        // Build the tree prefix
        let mut prefix = String::new();

        if row.depth > 0 {
            // Reconstruct parent chain to determine line drawing
            let mut ancestor_chain: Vec<usize> = Vec::new();
            let mut search_idx = idx;

            // Walk back to find all ancestors
            for target_depth in (0..row.depth).rev() {
                let mut found = false;
                for search in (0..search_idx).rev() {
                    if rows[search].depth == target_depth {
                        ancestor_chain.push(search);
                        search_idx = search;
                        found = true;
                        break;
                    }
                }
                if !found {
                    break;
                }
            }

            ancestor_chain.reverse();

            // Draw prefix based on ancestor chain
            for (d, &ancestor_idx) in ancestor_chain.iter().enumerate() {
                if d == ancestor_chain.len() - 1 {
                    // This is our immediate parent - check if we're the last child
                    let is_last_child = parent_map
                        .get(&ancestor_idx)
                        .and_then(|children| children.last())
                        .is_some_and(|&last| last == idx);

                    if is_last_child {
                        prefix.push_str(if ascii { "`- " } else { "└─ " });
                    } else {
                        prefix.push_str(if ascii { "|- " } else { "├─ " });
                    }
                } else {
                    // Earlier ancestor - check if it has more siblings after
                    let has_more_siblings = if d == 0 {
                        // Root level - check if there are more roots after this ancestor
                        rows.iter()
                            .skip(ancestor_idx + 1)
                            .any(|r| r.depth == rows[ancestor_idx].depth)
                    } else {
                        // Non-root - check if ancestor is last child of its parent
                        let ancestor_parent_idx = ancestor_chain.get(d - 1).copied();
                        ancestor_parent_idx
                            .and_then(|parent_idx| parent_map.get(&parent_idx))
                            .and_then(|children| children.last())
                            .is_some_and(|&last| last != ancestor_idx)
                    };

                    if has_more_siblings {
                        prefix.push_str(if ascii { "|  " } else { "│  " });
                    } else {
                        prefix.push_str("   ");
                    }
                }
            }
        }

        // Add direction marker if present
        let marker = match row.direction {
            Direction::Up => "↑ ",
            Direction::Down => "↓ ",
            Direction::None => "",
        };

        let title = entry.title.as_deref().unwrap_or_default();
        println!("{prefix}{marker}{} {title}", entry.hrid.display(digits));
    }
}

fn render_table(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    quiet: bool,
    filters: &Filters,
    digits: usize,
) {
    let selected_columns = if columns.is_empty() {
        if quiet {
            vec![ListColumn::Hrid]
        } else {
            vec![
                ListColumn::Hrid,
                ListColumn::Title,
                ListColumn::Kind,
                ListColumn::Parents,
                ListColumn::Children,
                ListColumn::Tags,
            ]
        }
    } else {
        columns.to_vec()
    };

    let mut headers = Vec::new();
    let mut data: Vec<Vec<String>> = Vec::new();

    if !quiet {
        headers = selected_columns
            .iter()
            .map(|column| column.header().to_string())
            .collect();
    }

    for row in rows {
        let entry = &entries[row.index];
        let mut row_data = Vec::new();

        for column in &selected_columns {
            let mut value = column.value(entry, digits);
            if *column == ListColumn::Hrid {
                value = prefix_value(value, row.direction, row.depth);
            }

            // Apply match highlighting for Title column
            if *column == ListColumn::Title {
                if let Some(search) = &filters.contains {
                    value = highlight_match(&value, search);
                }
            }

            row_data.push(value);
        }
        data.push(row_data);
    }

    if quiet {
        for row in data {
            println!("{}", row.join("\t"));
        }
        return;
    }

    // Determine column widths for alignment.
    let widths = headers
        .iter()
        .enumerate()
        .map(|(idx, header)| {
            data.iter()
                .map(|row| display_width(&row[idx]))
                .max()
                .unwrap_or(0)
                .max(display_width(header))
        })
        .collect::<Vec<_>>();

    if !headers.is_empty() {
        for (header, width) in headers.iter().zip(&widths) {
            print!("{header:<width$}  ");
        }
        println!();

        for width in &widths {
            print!("{:-<width$}  ", "");
        }
        println!();
    }

    for row in data {
        for (idx, value) in row.iter().enumerate() {
            let width = widths[idx];
            let padding = width.saturating_sub(display_width(value));
            print!("{value}{:padding$}  ", "");
        }
        println!();
    }
}

/// Terminal display width of a value, ignoring ANSI escape sequences.
///
/// Byte length would over-count any non-ASCII text (such as the `↑`/`↓`
/// relationship markers, which are three bytes but one column wide) and
/// misalign every subsequent column.
fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(strip_ansi(text).as_str())
}

fn highlight_match(text: &str, search: &str) -> String {
    use crate::cli::terminal::supports_color;

    let lower_text = text.to_ascii_lowercase();
    let lower_search = search.to_ascii_lowercase();

    lower_text.find(&lower_search).map_or_else(
        || text.to_string(),
        |pos| {
            let before = &text[..pos];
            let matched = &text[pos..pos + search.len()];
            let after = &text[pos + search.len()..];

            if supports_color() {
                // Use underline for matches
                format!("{before}\x1b[4m{matched}\x1b[24m{after}")
            } else {
                // Use << >> markers when no color support
                format!("{before}<<{matched}>>{after}")
            }
        },
    )
}

fn strip_ansi(text: &str) -> String {
    // Simple ANSI escape sequence stripper for width calculation
    let mut result = String::new();
    let mut in_escape = false;

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else if !in_escape {
            result.push(ch);
        }
    }

    result
}

fn render_json(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    digits: usize,
) -> anyhow::Result<()> {
    let selected_columns = if columns.is_empty() {
        vec![
            ListColumn::Hrid,
            ListColumn::Title,
            ListColumn::Kind,
            ListColumn::Namespace,
            ListColumn::Parents,
            ListColumn::Children,
            ListColumn::Tags,
            ListColumn::Path,
            ListColumn::Created,
        ]
    } else {
        columns.to_vec()
    };

    let mut rows_out = Vec::new();
    for row in rows {
        let entry = &entries[row.index];
        rows_out.push(build_serializable_row(entry, &selected_columns, digits));
    }

    serde_json::to_writer_pretty(std::io::stdout(), &rows_out)
        .context("failed to render json output")?;
    println!();
    Ok(())
}

fn render_csv(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    quiet: bool,
    digits: usize,
) {
    let selected_columns = if columns.is_empty() {
        vec![
            ListColumn::Hrid,
            ListColumn::Title,
            ListColumn::Kind,
            ListColumn::Parents,
            ListColumn::Children,
            ListColumn::Tags,
            ListColumn::Path,
        ]
    } else {
        columns.to_vec()
    };

    if !quiet {
        let header_line = selected_columns
            .iter()
            .map(|column| csv_escape(column.header()))
            .collect::<Vec<_>>()
            .join(",");
        println!("{header_line}");
    }

    for row in rows {
        let entry = &entries[row.index];
        let mut values = Vec::new();

        for column in &selected_columns {
            let value = csv_escape(&column.value(entry, digits));
            values.push(value);
        }

        println!("{}", values.join(","));
    }
}

fn prefix_value(mut value: String, direction: Direction, depth: usize) -> String {
    if direction == Direction::None {
        return value;
    }

    let arrows = match direction {
        Direction::Up => "↑",
        Direction::Down => "↓",
        Direction::None => "",
    };

    let repeat = depth.max(1);
    let prefix = arrows.repeat(repeat);
    value.insert_str(0, &format!("{prefix} "));
    value
}

fn build_serializable_row<'a>(
    entry: &'a Entry,
    columns: &[ListColumn],
    digits: usize,
) -> SerializableRow<'a> {
    let mut row = SerializableRow {
        hrid: entry.hrid.display(digits).to_string(),
        title: None,
        kind: None,
        namespace: None,
        parents: None,
        children: None,
        tags: None,
        path: None,
        created: None,
    };

    for column in columns {
        match column {
            ListColumn::Hrid => {}
            ListColumn::Title => {
                row.title = entry.title.as_deref();
            }
            ListColumn::Kind => {
                row.kind = Some(entry.hrid.kind());
            }
            ListColumn::Namespace => {
                let namespace = entry.hrid.namespace().join("-");
                if !namespace.is_empty() {
                    row.namespace = Some(namespace);
                }
            }
            ListColumn::Parents => {
                if !entry.parents.is_empty() {
                    let parents = entry
                        .parents
                        .iter()
                        .map(|link| link.hrid.display(digits).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    row.parents = Some(parents);
                }
            }
            ListColumn::Children => {
                if !entry.children.is_empty() {
                    let children = entry
                        .children
                        .iter()
                        .map(|link| link.hrid.display(digits).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    row.children = Some(children);
                }
            }
            ListColumn::Tags => {
                if !entry.tags.is_empty() {
                    row.tags = Some(entry.tags.join(", "));
                }
            }
            ListColumn::Path => {
                row.path = Some(entry.path.display().to_string());
            }
            ListColumn::Created => {
                row.created = Some(entry.created.to_rfc3339());
            }
        }
    }

    row
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::fixtures::{empty_filters, row, SampleEntries},
        *,
    };

    #[test]
    fn render_rows_supports_all_formats() {
        let fixtures = SampleEntries::new();
        let entries = fixtures.entries();
        let rows = vec![
            row(fixtures.root_index(), Direction::None, 0),
            row(fixtures.child_index(), Direction::Down, 1),
        ];

        let empty_filters = empty_filters();

        render_rows(
            rows.clone(),
            entries,
            &[ListColumn::Hrid, ListColumn::Title, ListColumn::Tags],
            OutputFormat::Table,
            false,
            false,
            false,
            &empty_filters,
            3,
        )
        .unwrap();

        render_rows(
            rows.clone(),
            entries,
            &[ListColumn::Hrid],
            OutputFormat::Table,
            true,
            false,
            false,
            &empty_filters,
            3,
        )
        .unwrap();

        render_rows(
            rows.clone(),
            entries,
            &[
                ListColumn::Hrid,
                ListColumn::Title,
                ListColumn::Path,
                ListColumn::Created,
            ],
            OutputFormat::Json,
            false,
            false,
            false,
            &empty_filters,
            3,
        )
        .unwrap();

        render_rows(
            rows.clone(),
            entries,
            &[ListColumn::Hrid, ListColumn::Title, ListColumn::Tags],
            OutputFormat::Csv,
            false,
            false,
            false,
            &empty_filters,
            3,
        )
        .unwrap();

        render_rows(
            rows,
            entries,
            &[ListColumn::Hrid],
            OutputFormat::Table,
            false,
            true,
            false,
            &empty_filters,
            3,
        )
        .unwrap();
    }

    #[test]
    fn display_width_ignores_ansi_and_counts_columns() {
        // ANSI escapes take no columns.
        assert_eq!(display_width("\x1b[4mabc\x1b[24m"), 3);
        // The relationship markers are multi-byte but single-column.
        assert_eq!(display_width("\u{2191} SYS-001"), 9);
        assert_eq!("\u{2191} SYS-001".len(), 11);
    }

    #[test]
    fn prefix_value_adds_direction_markers() {
        assert_eq!(
            prefix_value("value".to_string(), Direction::None, 0),
            "value"
        );
        assert_eq!(
            prefix_value("value".to_string(), Direction::Up, 1),
            "↑ value"
        );
        assert_eq!(
            prefix_value("value".to_string(), Direction::Down, 3),
            "↓↓↓ value"
        );
    }

    #[test]
    fn build_serializable_row_populates_fields() {
        let fixtures = SampleEntries::new();
        let entry = fixtures.entry(fixtures.leaf_index());
        let row = build_serializable_row(
            entry,
            &[
                ListColumn::Hrid,
                ListColumn::Title,
                ListColumn::Kind,
                ListColumn::Namespace,
                ListColumn::Parents,
                ListColumn::Children,
                ListColumn::Tags,
                ListColumn::Path,
                ListColumn::Created,
            ],
            3,
        );

        assert_eq!(row.hrid, entry.hrid.display(3).to_string());
        assert_eq!(row.title, entry.title.as_deref());
        assert_eq!(row.kind, Some(entry.hrid.kind()));
        assert_eq!(row.namespace.as_deref(), Some("SYSTEM-AUTH"));
        assert!(row.parents.as_ref().unwrap().contains("SYS-002"));
        assert!(row.children.is_none());
        assert!(row.tags.as_ref().unwrap().contains("Security"));
        assert_eq!(row.path.as_deref(), Some("system/auth/USR-007.md"));
        assert!(row.created.is_some());
    }

    #[test]
    fn csv_escape_quotes_and_commas() {
        assert_eq!(csv_escape("simple"), "simple");
        assert_eq!(csv_escape("needs,comma"), "\"needs,comma\"");
        assert_eq!(csv_escape("quote\"here"), "\"quote\"\"here\"");
    }
}
