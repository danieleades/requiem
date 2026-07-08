//! The `req list` command.
//!
//! Listing is a small pipeline, with one submodule per stage:
//!
//! - `entry`: snapshot the requirements into list entries
//! - `filter`: compile and apply the command-line filters
//! - `row`: produce result rows (traversal, sorting, pagination)
//! - `render`: print rows as a table, JSON, CSV, or tree

use std::{collections::HashMap, fmt, path::PathBuf};

use anyhow::Context;
use clap::{Parser, ValueEnum};
use requiem_core::{Directory, Hrid};
use tracing::instrument;
use uuid::Uuid;

use super::parse_hrid;

mod entry;
mod filter;
mod render;
mod row;

use entry::{collect_entries, Entry, LinkRef};
use filter::Filters;
use render::{render_rows, ListColumn};
use row::{
    apply_offset_limit, apply_sort, augment_with_context, produce_base_rows,
    produce_direction_rows, produce_tree_rows, resolve_depth, truncation_count,
};

const DEFAULT_LIMIT: usize = 200;

/// Command arguments for `req list`.
#[derive(Debug, Parser)]
#[command(about = "List requirements with filters and relationship views")]
#[allow(clippy::struct_excessive_bools)]
pub struct List {
    /// HRIDs to use as primary targets.
    #[arg(value_parser = parse_hrid)]
    targets: Vec<Hrid>,

    /// Columns to display (comma-separated).
    #[arg(long, value_delimiter = ',', value_name = "COL")]
    columns: Vec<ListColumn>,

    /// Sort field (default: hrid).
    #[arg(long, value_enum, default_value_t)]
    sort: SortField,

    /// Output format (default: table).
    #[arg(long, value_enum, default_value_t)]
    output: OutputFormat,

    /// Suppress headers and format rows for scripting.
    #[arg(long)]
    quiet: bool,

    /// Filter by requirement kind (comma-separated, case-insensitive).
    #[arg(long, value_delimiter = ',', value_name = "KIND")]
    kind: Vec<String>,

    /// Filter by namespace segment (comma-separated, case-insensitive).
    #[arg(long, value_delimiter = ',', value_name = "NS")]
    namespace: Vec<String>,

    /// Filter by tag (comma-separated, case-insensitive).
    #[arg(long, value_delimiter = ',', value_name = "TAG")]
    tag: Vec<String>,

    /// Show only requirements without parents.
    #[arg(long)]
    orphans: bool,

    /// Show only requirements without children.
    #[arg(long)]
    leaves: bool,

    /// Case-insensitive substring match against title/body.
    #[arg(long, conflicts_with = "regex")]
    contains: Option<String>,

    /// Regular expression match against title/body.
    #[arg(long)]
    regex: Option<String>,

    /// Relationship view to apply (default: summary table).
    #[arg(long, value_enum, default_value_t)]
    view: View,

    /// Depth limit for relationship views (0 = unlimited, defaults vary by
    /// view).
    #[arg(long, value_name = "N")]
    depth: Option<usize>,

    /// Limit number of rows returned.
    #[arg(long)]
    limit: Option<usize>,

    /// Skip the first N rows.
    #[arg(long)]
    offset: Option<usize>,

    /// Use ASCII characters instead of UTF-8 box drawing.
    #[arg(long)]
    ascii: bool,
}

/// Supported output formats.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

/// Sortable fields.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Default)]
pub enum SortField {
    #[default]
    Hrid,
    Kind,
    Title,
    Created,
}

/// View applied to the listing results.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Default)]
pub enum View {
    #[default]
    Summary,
    Parents,
    Children,
    Ancestors,
    Descendants,
    Tree,
    Context,
}

impl View {
    const fn name(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Parents => "parents",
            Self::Children => "children",
            Self::Ancestors => "ancestors",
            Self::Descendants => "descendants",
            Self::Tree => "tree",
            Self::Context => "context",
        }
    }
}

impl List {
    #[instrument(level = "debug", skip_all)]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use crate::cli::terminal::Colorize;

        let directory = Directory::new(root)?;
        let digits = directory.config().digits();

        let mut entries = collect_entries(&directory);
        let index_by_uuid: HashMap<Uuid, usize> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| (entry.uuid, idx))
            .collect();

        // Populate children based on parent links.
        for idx in 0..entries.len() {
            let entry = entries[idx].clone();
            for parent in &entry.parents {
                if let Some(parent_idx) = index_by_uuid.get(&parent.uuid) {
                    entries[*parent_idx]
                        .children
                        .push(LinkRef::new(entry.uuid, entry.hrid.clone()));
                }
            }
        }

        let filters = Filters::new(&self)?;

        let target_indices = if self.targets.is_empty() {
            (0..entries.len()).collect::<Vec<_>>()
        } else {
            self.resolve_targets(&entries, digits)?
        };

        let mut rows = self.produce_rows(&entries, &index_by_uuid, &filters, target_indices);

        // Check for empty results
        if rows.is_empty() && filters.any() {
            println!("No requirements matched the filters provided.");
            return Ok(());
        }

        if self.view != View::Tree {
            rows = apply_sort(rows, &entries, self.sort, digits);
        }

        // Apply default limit only when user didn't specify --limit
        // Allow --limit 0 to mean "unlimited"
        let effective_limit = match self.limit {
            None => Some(DEFAULT_LIMIT), // Not specified, use default
            Some(0) => None,             // Explicitly 0, unlimited
            Some(n) => Some(n),          // Explicit positive value
        };

        let total_rows = rows.len();
        let truncated_count = truncation_count(total_rows, self.offset, effective_limit);
        let truncated = truncated_count > 0;

        rows = apply_offset_limit(rows, self.offset, effective_limit);

        // Print header (unless quiet or json/csv)
        if !self.quiet && self.output == OutputFormat::Table {
            let limit_str = match self.limit {
                None => DEFAULT_LIMIT.to_string(),
                Some(0) => "unlimited".to_string(),
                Some(limit) => limit.to_string(),
            };

            println!(
                "{}",
                format!(
                    "Listing requirements (view: {}, filters: {}, limit: {limit_str})",
                    self.view.name(),
                    filters.describe()
                )
                .dim()
            );
        }

        render_rows(
            rows,
            &entries,
            &self.columns,
            self.output,
            self.quiet,
            self.view == View::Tree,
            self.ascii,
            &filters,
            digits,
        )?;

        // Print footer if truncated
        if truncated && !self.quiet && self.output == OutputFormat::Table {
            println!(
                "\n{} +{} more (use --limit or --offset)",
                "…".dim(),
                truncated_count
            );
        }

        Ok(())
    }

    /// Produce the result rows for the selected relationship view.
    fn produce_rows(
        &self,
        entries: &[Entry],
        index_by_uuid: &HashMap<Uuid, usize>,
        filters: &Filters,
        target_indices: Vec<usize>,
    ) -> Vec<row::Row> {
        match self.view {
            View::Summary => produce_base_rows(entries, filters, &target_indices),
            View::Parents | View::Children | View::Ancestors | View::Descendants => {
                produce_direction_rows(
                    self.view,
                    entries,
                    index_by_uuid,
                    filters,
                    &target_indices,
                    self.depth,
                )
            }
            View::Tree => {
                // For tree view, pass empty targets if no specific targets were requested
                // This allows produce_tree_rows to automatically find and show roots
                let tree_targets = if self.targets.is_empty() {
                    Vec::new()
                } else {
                    target_indices
                };
                produce_tree_rows(entries, index_by_uuid, filters, &tree_targets, self.depth)
            }
            View::Context => {
                let base = produce_base_rows(entries, filters, &target_indices);
                let depth = resolve_depth(self.depth, 1);
                augment_with_context(entries, index_by_uuid, base, depth)
            }
        }
    }

    fn resolve_targets(&self, entries: &[Entry], digits: usize) -> anyhow::Result<Vec<usize>> {
        let mut lookup = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            lookup.insert(entry.hrid.display(digits).to_string(), idx);
        }

        let mut indices = Vec::new();
        for hrid in &self.targets {
            let key = hrid.display(digits).to_string();
            let idx = lookup
                .get(&key)
                .copied()
                .with_context(|| format!("requirement {key} not found"))?;
            indices.push(idx);
        }
        Ok(indices)
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Table => "table",
            Self::Json => "json",
            Self::Csv => "csv",
        })
    }
}

/// Shared entry/row fixtures used by the submodule tests.
#[cfg(test)]
mod fixtures {
    use std::{collections::HashMap, path::PathBuf};

    use chrono::{Duration, TimeZone, Utc};
    use uuid::Uuid;

    use super::{
        entry::{Entry, LinkRef},
        filter::Filters,
        row::{Direction, Row},
        Hrid,
    };

    pub(super) struct SampleEntries {
        entries: Vec<Entry>,
        index_by_uuid: HashMap<Uuid, usize>,
        root_uuid: Uuid,
        child_uuid: Uuid,
        leaf_uuid: Uuid,
    }

    impl SampleEntries {
        pub(super) fn new() -> Self {
            let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

            let root_uuid = Uuid::new_v4();
            let child_uuid = Uuid::new_v4();
            let leaf_uuid = Uuid::new_v4();

            let mut root = Entry {
                uuid: root_uuid,
                hrid: Hrid::try_from("SYS-001").unwrap(),
                title: Some("Root".to_string()),
                tags: vec!["core".to_string()],
                created: base_time,
                content: "# Root requirement\nPrimary".to_string(),
                parents: Vec::new(),
                children: Vec::new(),
                path: PathBuf::from("SYS-001.md"),
            };

            let mut child = Entry {
                uuid: child_uuid,
                hrid: Hrid::try_from("SYS-002").unwrap(),
                title: Some("Child".to_string()),
                tags: Vec::new(),
                created: base_time + Duration::days(1),
                content: "## Child details\nImplements root".to_string(),
                parents: vec![LinkRef::new(root_uuid, root.hrid.clone())],
                children: Vec::new(),
                path: PathBuf::from("SYS-002.md"),
            };

            let leaf = Entry {
                uuid: leaf_uuid,
                hrid: {
                    use std::num::NonZeroUsize;

                    use requiem_core::domain::hrid::{KindString, NamespaceSegment};
                    Hrid::new_with_namespace(
                        vec![
                            NamespaceSegment::new("SYSTEM".to_string()).unwrap(),
                            NamespaceSegment::new("AUTH".to_string()).unwrap(),
                        ],
                        KindString::new("USR".to_string()).unwrap(),
                        NonZeroUsize::new(7).unwrap(),
                    )
                },
                title: Some("Login".to_string()),
                tags: vec!["Security".to_string(), "UI".to_string()],
                created: base_time + Duration::days(2),
                content: "Implements login".to_string(),
                parents: vec![LinkRef::new(child_uuid, child.hrid.clone())],
                children: Vec::new(),
                path: PathBuf::from("system/auth/USR-007.md"),
            };

            root.children
                .push(LinkRef::new(child_uuid, child.hrid.clone()));
            child
                .children
                .push(LinkRef::new(leaf_uuid, leaf.hrid.clone()));

            let entries = vec![root, child, leaf];
            let mut index_by_uuid = HashMap::new();
            for (idx, entry) in entries.iter().enumerate() {
                index_by_uuid.insert(entry.uuid, idx);
            }

            Self {
                entries,
                index_by_uuid,
                root_uuid,
                child_uuid,
                leaf_uuid,
            }
        }

        pub(super) fn entries(&self) -> &[Entry] {
            &self.entries
        }

        pub(super) fn index_map(&self) -> &HashMap<Uuid, usize> {
            &self.index_by_uuid
        }

        pub(super) fn entry(&self, index: usize) -> &Entry {
            &self.entries[index]
        }

        pub(super) fn root_index(&self) -> usize {
            self.index_by_uuid[&self.root_uuid]
        }

        pub(super) fn child_index(&self) -> usize {
            self.index_by_uuid[&self.child_uuid]
        }

        pub(super) fn leaf_index(&self) -> usize {
            self.index_by_uuid[&self.leaf_uuid]
        }

        pub(super) fn leaf(&self) -> &Entry {
            self.entry(self.leaf_index())
        }
    }

    pub(super) const fn row(index: usize, direction: Direction, depth: usize) -> Row {
        Row {
            index,
            direction,
            depth,
        }
    }

    pub(super) fn empty_filters() -> Filters {
        Filters {
            kinds: Vec::new(),
            namespaces: Vec::new(),
            tags: Vec::new(),
            orphans: false,
            leaves: false,
            contains: None,
            regex: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use requiem_core::{Directory, Requirement};
    use tempfile::tempdir;

    use super::*;

    fn base_list() -> List {
        List {
            targets: Vec::new(),
            columns: vec![
                ListColumn::Hrid,
                ListColumn::Title,
                ListColumn::Parents,
                ListColumn::Children,
                ListColumn::Tags,
            ],
            sort: SortField::Hrid,
            output: OutputFormat::Table,
            quiet: false,
            kind: Vec::new(),
            namespace: Vec::new(),
            tag: Vec::new(),
            orphans: false,
            leaves: false,
            contains: None,
            regex: None,
            view: View::Summary,
            depth: None,
            limit: Some(10),
            offset: Some(0),
            ascii: false,
        }
    }

    fn add_requirement(directory: &mut Directory, kind: &str, content: &str) -> Requirement {
        directory
            .add_requirement(kind, content.to_string())
            .unwrap()
    }

    #[test]
    fn output_format_display_matches_expected_strings() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Csv.to_string(), "csv");
    }

    #[test]
    fn list_run_covers_views_and_formats() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).unwrap();
        let parent = add_requirement(&mut directory, "SYS", "# Parent");
        let child = add_requirement(&mut directory, "USR", "# Child\nImplements parent");

        directory
            .link_requirement(child.hrid(), parent.hrid())
            .unwrap();
        directory.flush().unwrap();

        let mut table = base_list();
        table.sort = SortField::Title;
        table.run(root.clone()).unwrap();

        let mut json = base_list();
        json.output = OutputFormat::Json;
        json.view = View::Context;
        json.depth = Some(1);
        json.targets = vec![child.hrid().clone()];
        json.run(root.clone()).unwrap();

        let mut csv = base_list();
        csv.output = OutputFormat::Csv;
        csv.quiet = true;
        csv.offset = Some(1);
        csv.limit = Some(1);
        csv.run(root.clone()).unwrap();

        let mut tree = base_list();
        tree.view = View::Tree;
        tree.depth = Some(2);
        tree.run(root).unwrap();
    }
}
