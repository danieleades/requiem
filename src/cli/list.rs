use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    path::PathBuf,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use regex::Regex;
use requiem::{Directory, Hrid, RequirementView};
use serde::Serialize;
use tracing::instrument;
use uuid::Uuid;

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

/// Available table columns.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub enum ListColumn {
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

impl clap::ValueEnum for ListColumn {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Hrid,
            Self::Title,
            Self::Kind,
            Self::Namespace,
            Self::Parents,
            Self::Children,
            Self::Tags,
            Self::Path,
            Self::Created,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Hrid => clap::builder::PossibleValue::new("hrid"),
            Self::Title => clap::builder::PossibleValue::new("title"),
            Self::Kind => clap::builder::PossibleValue::new("kind"),
            Self::Namespace => clap::builder::PossibleValue::new("namespace"),
            Self::Parents => clap::builder::PossibleValue::new("parents"),
            Self::Children => clap::builder::PossibleValue::new("children"),
            Self::Tags => clap::builder::PossibleValue::new("tags"),
            Self::Path => clap::builder::PossibleValue::new("path"),
            Self::Created => clap::builder::PossibleValue::new("created"),
        })
    }

    fn from_str(label: &str, _case_sensitive: bool) -> Result<Self, String> {
        match label.to_ascii_lowercase().as_str() {
            "hrid" => Ok(Self::Hrid),
            "title" => Ok(Self::Title),
            "kind" => Ok(Self::Kind),
            "namespace" => Ok(Self::Namespace),
            "parents" => Ok(Self::Parents),
            "children" => Ok(Self::Children),
            "tags" => Ok(Self::Tags),
            "path" => Ok(Self::Path),
            "created" => Ok(Self::Created),
            _ => Err(format!("unknown column: {label}")),
        }
    }
}

/// Parsed requirement snapshot used for listing.
#[derive(Debug, Clone)]
struct Entry {
    uuid: Uuid,
    hrid: Hrid,
    title: Option<String>,
    tags: Vec<String>,
    created: DateTime<Utc>,
    content: String,
    parents: Vec<LinkRef>,
    children: Vec<LinkRef>,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct LinkRef {
    uuid: Uuid,
    hrid: Hrid,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum Direction {
    None,
    Up,
    Down,
}

#[derive(Debug, Clone)]
struct Row {
    index: usize,
    direction: Direction,
    depth: usize,
}

#[derive(Debug, Clone)]
struct Filters {
    kinds: Vec<String>,
    namespaces: Vec<String>,
    tags: Vec<String>,
    orphans: bool,
    leaves: bool,
    contains: Option<String>,
    regex: Option<Regex>,
}

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

impl List {
    #[instrument(level = "debug", skip_all)]
    #[allow(clippy::too_many_lines)]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        use crate::cli::terminal::Colorize;

        let directory = Directory::new(root)?;

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
            self.resolve_targets(&entries)?
        };

        let mut rows = match self.view {
            View::Summary => produce_base_rows(&entries, &filters, &target_indices),
            View::Parents | View::Children | View::Ancestors | View::Descendants => {
                produce_direction_rows(
                    self.view,
                    &entries,
                    &index_by_uuid,
                    &filters,
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
                produce_tree_rows(
                    &entries,
                    &index_by_uuid,
                    &filters,
                    &tree_targets,
                    self.depth,
                )
            }
            View::Context => {
                let base = produce_base_rows(&entries, &filters, &target_indices);
                let depth = resolve_depth(self.depth, 1);
                augment_with_context(&entries, &index_by_uuid, base, depth)
            }
        };

        // Check for empty results
        if rows.is_empty() && filters.any() {
            println!("No requirements matched the filters provided.");
            return Ok(());
        }

        if self.view != View::Tree {
            rows = apply_sort(rows, &entries, self.sort);
        }

        let effective_limit = self
            .limit
            .and_then(|value| (value > 0).then_some(value))
            .or(Some(DEFAULT_LIMIT));

        let total_rows = rows.len();
        let truncated = total_rows > effective_limit.unwrap_or(usize::MAX);
        let truncated_count = if truncated {
            total_rows.saturating_sub(effective_limit.unwrap_or(0))
        } else {
            0
        };

        rows = apply_offset_limit(rows, self.offset, effective_limit);

        // Print header (unless quiet or json/csv)
        if !self.quiet && self.output == OutputFormat::Table {
            let view_name = match self.view {
                View::Summary => "summary",
                View::Parents => "parents",
                View::Children => "children",
                View::Ancestors => "ancestors",
                View::Descendants => "descendants",
                View::Tree => "tree",
                View::Context => "context",
            };

            let filter_str = if filters.any() {
                let mut parts = Vec::new();
                if !filters.kinds.is_empty() {
                    parts.push(format!("kind: {}", filters.kinds.join(", ")));
                }
                if !filters.namespaces.is_empty() {
                    parts.push(format!("namespace: {}", filters.namespaces.join(", ")));
                }
                if !filters.tags.is_empty() {
                    parts.push(format!("tag: {}", filters.tags.join(", ")));
                }
                if filters.orphans {
                    parts.push("orphans".to_string());
                }
                if filters.leaves {
                    parts.push("leaves".to_string());
                }
                if filters.contains.is_some() {
                    parts.push("text-match".to_string());
                }
                if filters.regex.is_some() {
                    parts.push("regex-match".to_string());
                }
                parts.join(", ")
            } else {
                "none".to_string()
            };

            let limit_str = self
                .limit
                .map_or_else(|| DEFAULT_LIMIT.to_string(), |l| l.to_string());

            println!(
                "{}",
                format!(
                    "Listing requirements (view: {view_name}, filters: {filter_str}, limit: \
                     {limit_str})"
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

    fn resolve_targets(&self, entries: &[Entry]) -> anyhow::Result<Vec<usize>> {
        let mut lookup = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            lookup.insert(entry.hrid.to_string(), idx);
        }

        let mut indices = Vec::new();
        for hrid in &self.targets {
            let key = hrid.to_string();
            let idx = lookup
                .get(&key)
                .copied()
                .with_context(|| format!("requirement {key} not found"))?;
            indices.push(idx);
        }
        Ok(indices)
    }
}

impl Filters {
    fn new(cmd: &List) -> anyhow::Result<Self> {
        let regex = if let Some(pattern) = &cmd.regex {
            Some(Regex::new(pattern).with_context(|| format!("invalid regex: {pattern}"))?)
        } else {
            None
        };

        Ok(Self {
            kinds: cmd
                .kind
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            namespaces: cmd
                .namespace
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            tags: cmd
                .tag
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            orphans: cmd.orphans,
            leaves: cmd.leaves,
            contains: cmd.contains.as_deref().map(str::to_ascii_lowercase),
            regex,
        })
    }

    fn any(&self) -> bool {
        !self.kinds.is_empty()
            || !self.namespaces.is_empty()
            || !self.tags.is_empty()
            || self.orphans
            || self.leaves
            || self.contains.is_some()
            || self.regex.is_some()
    }

    fn matches(&self, entry: &Entry) -> bool {
        if !self.kinds.is_empty() {
            let kind = entry.hrid.kind().to_ascii_lowercase();
            if !self.kinds.iter().any(|k| k == &kind) {
                return false;
            }
        }

        if !self.namespaces.is_empty() {
            let namespace: Vec<String> = entry
                .hrid
                .namespace()
                .into_iter()
                .map(str::to_ascii_lowercase)
                .collect();
            if !self
                .namespaces
                .iter()
                .any(|ns| namespace.iter().any(|segment| segment == ns))
            {
                return false;
            }
        }

        if !self.tags.is_empty() {
            let tag_set: Vec<String> = entry
                .tags
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect();
            if !self
                .tags
                .iter()
                .any(|tag| tag_set.iter().any(|entry_tag| entry_tag == tag))
            {
                return false;
            }
        }

        if self.orphans && !entry.parents.is_empty() {
            return false;
        }

        if self.leaves && !entry.children.is_empty() {
            return false;
        }

        if let Some(search) = &self.contains {
            let title = entry
                .title
                .as_deref()
                .map_or_else(String::new, str::to_ascii_lowercase);
            if title.contains(search) {
                // already matched
            } else if !entry.content.to_ascii_lowercase().contains(search) {
                return false;
            }
        }

        if let Some(regex) = &self.regex {
            let haystack = entry.title.as_deref().map_or_else(
                || entry.content.clone(),
                |title| format!("{title}\n{}", entry.content),
            );
            if !regex.is_match(&haystack) {
                return false;
            }
        }

        true
    }
}

impl LinkRef {
    const fn new(uuid: Uuid, hrid: Hrid) -> Self {
        Self { uuid, hrid }
    }
}

fn collect_entries(directory: &Directory) -> Vec<Entry> {
    let mut entries = Vec::new();

    for requirement in directory.requirements() {
        entries.push(entry_from_requirement(directory, &requirement));
    }

    entries
}

fn entry_from_requirement(directory: &Directory, requirement: &RequirementView) -> Entry {
    let parents = requirement
        .parents
        .iter()
        .map(|(uuid, parent)| LinkRef::new(*uuid, parent.hrid.clone()))
        .collect::<Vec<_>>();

    let tags = requirement.tags.iter().cloned().collect::<Vec<_>>();
    let path = directory.path_for(requirement.hrid);

    Entry {
        uuid: *requirement.uuid,
        hrid: requirement.hrid.clone(),
        title: extract_title(requirement.content),
        tags,
        created: *requirement.created,
        content: requirement.content.to_string(),
        parents,
        children: Vec::new(),
        path,
    }
}

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return Some(stripped.trim().to_string());
        }

        if let Some(stripped) = trimmed.strip_prefix("## ") {
            return Some(stripped.trim().to_string());
        }

        if let Some(stripped) = trimmed.strip_prefix("### ") {
            return Some(stripped.trim().to_string());
        }

        // First non-empty line as fallback.
        return Some(trimmed.to_string());
    }
    None
}

fn produce_base_rows(entries: &[Entry], filters: &Filters, target_indices: &[usize]) -> Vec<Row> {
    let mut rows = Vec::<Row>::new();

    let mut indices = target_indices.to_vec();

    if filters.any() {
        indices.retain(|idx| filters.matches(&entries[*idx]));
    }

    for index in indices {
        rows.push(Row {
            index,
            direction: Direction::None,
            depth: 0,
        });
    }

    rows
}

fn resolve_depth(depth: Option<usize>, default: usize) -> usize {
    depth.map_or(default, |value| if value == 0 { usize::MAX } else { value })
}

fn produce_direction_rows(
    view: View,
    entries: &[Entry],
    index_by_uuid: &HashMap<Uuid, usize>,
    filters: &Filters,
    target_indices: &[usize],
    depth: Option<usize>,
) -> Vec<Row> {
    let direction = match view {
        View::Parents | View::Ancestors => Direction::Up,
        View::Children | View::Descendants => Direction::Down,
        _ => unreachable!("directional view expected"),
    };

    let default_depth = match view {
        View::Parents | View::Children => 1,
        View::Ancestors | View::Descendants => usize::MAX,
        _ => unreachable!("directional view expected"),
    };

    let limit = resolve_depth(depth, default_depth);

    let rows = traverse(
        target_indices.iter().copied(),
        entries,
        index_by_uuid,
        limit,
        direction,
    );

    rows.into_iter()
        .filter(|row| filters.matches(&entries[row.index]))
        .collect()
}

fn produce_tree_rows(
    entries: &[Entry],
    index_by_uuid: &HashMap<Uuid, usize>,
    filters: &Filters,
    target_indices: &[usize],
    depth: Option<usize>,
) -> Vec<Row> {
    // Depth-first traversal for tree rendering
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        index: usize,
        depth: usize,
        limit: usize,
        direction: Direction,
        entries: &[Entry],
        index_by_uuid: &HashMap<Uuid, usize>,
        seen: &mut HashSet<usize>,
        rows: &mut Vec<Row>,
        filters: &Filters,
    ) {
        if limit != usize::MAX && depth > limit {
            return;
        }
        if !seen.insert(index) {
            return;
        }

        rows.push(Row {
            index,
            direction,
            depth,
        });

        if limit != usize::MAX && depth == limit {
            return;
        }

        // Recursively visit children
        for child in &entries[index].children {
            if let Some(&child_idx) = index_by_uuid.get(&child.uuid) {
                if filters.matches(&entries[child_idx]) || depth + 1 < limit {
                    dfs(
                        child_idx,
                        depth + 1,
                        limit,
                        Direction::Down,
                        entries,
                        index_by_uuid,
                        seen,
                        rows,
                        filters,
                    );
                }
            }
        }
    }

    let seeds = if target_indices.is_empty() {
        // Find all root requirements (those with no parents)
        (0..entries.len())
            .filter(|&idx| entries[idx].parents.is_empty())
            .collect()
    } else {
        target_indices.to_vec()
    };

    let limit = resolve_depth(depth, usize::MAX);
    let mut rows = Vec::new();
    let mut seen = HashSet::new();

    // Start DFS from each seed
    for &seed in &seeds {
        dfs(
            seed,
            0,
            limit,
            Direction::None,
            entries,
            index_by_uuid,
            &mut seen,
            &mut rows,
            filters,
        );
    }

    // Apply filters to the results
    rows.retain(|row| filters.matches(&entries[row.index]) || row.depth == 0);

    rows
}

fn augment_with_context(
    entries: &[Entry],
    index_by_uuid: &HashMap<Uuid, usize>,
    mut rows: Vec<Row>,
    depth: usize,
) -> Vec<Row> {
    if depth == 0 {
        return rows;
    }

    let existing: HashSet<usize> = rows.iter().map(|row| row.index).collect();

    let mut additional = Vec::new();

    for row in &rows {
        let seeds = std::iter::once(row.index);
        let parents = traverse(seeds.clone(), entries, index_by_uuid, depth, Direction::Up);
        let children = traverse(seeds, entries, index_by_uuid, depth, Direction::Down);

        for rel_row in parents.into_iter().chain(children) {
            if existing.contains(&rel_row.index) {
                continue;
            }
            if additional
                .iter()
                .any(|existing_row: &Row| existing_row.index == rel_row.index)
            {
                continue;
            }
            additional.push(rel_row);
        }
    }

    rows.extend(additional);
    rows
}

fn traverse<I>(
    seeds: I,
    entries: &[Entry],
    index_by_uuid: &HashMap<Uuid, usize>,
    depth_limit: usize,
    direction: Direction,
) -> Vec<Row>
where
    I: IntoIterator<Item = usize>,
{
    let mut seen = HashSet::<usize>::new();
    let mut queue = VecDeque::<(usize, usize)>::new();

    for seed in seeds {
        match direction {
            Direction::Up => {
                for parent in &entries[seed].parents {
                    if let Some(&idx) = index_by_uuid.get(&parent.uuid) {
                        queue.push_back((idx, 1));
                    }
                }
            }
            Direction::Down => {
                for child in &entries[seed].children {
                    if let Some(&idx) = index_by_uuid.get(&child.uuid) {
                        queue.push_back((idx, 1));
                    }
                }
            }
            Direction::None => {}
        }
    }

    let mut results = Vec::new();
    while let Some((index, depth)) = queue.pop_front() {
        if depth_limit != usize::MAX && depth > depth_limit {
            continue;
        }
        if !seen.insert(index) {
            continue;
        }

        results.push(Row {
            index,
            direction,
            depth,
        });

        if depth_limit != usize::MAX && depth == depth_limit {
            continue;
        }

        match direction {
            Direction::Up => {
                for parent in &entries[index].parents {
                    if let Some(&parent_idx) = index_by_uuid.get(&parent.uuid) {
                        queue.push_back((parent_idx, depth + 1));
                    }
                }
            }
            Direction::Down => {
                for child in &entries[index].children {
                    if let Some(&child_idx) = index_by_uuid.get(&child.uuid) {
                        queue.push_back((child_idx, depth + 1));
                    }
                }
            }
            Direction::None => {}
        }
    }

    results
}

#[allow(dead_code)]
fn append_unique_rows(rows: &mut Vec<Row>, new_rows: Vec<Row>) {
    let mut existing: HashSet<(usize, Direction)> =
        rows.iter().map(|row| (row.index, row.direction)).collect();

    for row in new_rows {
        if existing.insert((row.index, row.direction)) {
            rows.push(row);
        }
    }
}

fn apply_sort(mut rows: Vec<Row>, entries: &[Entry], sort_field: SortField) -> Vec<Row> {
    rows.sort_by(|a, b| compare_rows(a, b, entries, sort_field));
    rows
}

fn compare_rows(a: &Row, b: &Row, entries: &[Entry], sort_field: SortField) -> Ordering {
    let entry_a = &entries[a.index];
    let entry_b = &entries[b.index];

    let primary = match sort_field {
        SortField::Hrid => entry_a.hrid.to_string().cmp(&entry_b.hrid.to_string()),
        SortField::Kind => entry_a
            .hrid
            .kind()
            .cmp(entry_b.hrid.kind())
            .then_with(|| entry_a.hrid.to_string().cmp(&entry_b.hrid.to_string())),
        SortField::Title => entry_a
            .title
            .as_deref()
            .unwrap_or_default()
            .cmp(entry_b.title.as_deref().unwrap_or_default())
            .then_with(|| entry_a.hrid.to_string().cmp(&entry_b.hrid.to_string())),
        SortField::Created => entry_a
            .created
            .cmp(&entry_b.created)
            .then_with(|| entry_a.hrid.to_string().cmp(&entry_b.hrid.to_string())),
    };

    if primary == Ordering::Equal {
        a.depth.cmp(&b.depth)
    } else {
        primary
    }
}

fn apply_offset_limit(mut rows: Vec<Row>, offset: Option<usize>, limit: Option<usize>) -> Vec<Row> {
    if let Some(off) = offset {
        if off < rows.len() {
            rows = rows.into_iter().skip(off).collect();
        } else {
            rows.clear();
        }
    }

    if let Some(max) = limit {
        rows.truncate(max);
    }

    rows
}

#[allow(clippy::too_many_arguments)]
fn render_rows(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    output: OutputFormat,
    quiet: bool,
    tree: bool,
    ascii: bool,
    filters: &Filters,
) -> anyhow::Result<()> {
    if tree {
        render_tree(&rows, entries, ascii);
        return Ok(());
    }

    match output {
        OutputFormat::Table => {
            render_table(rows, entries, columns, quiet, filters);
            Ok(())
        }
        OutputFormat::Json => render_json(rows, entries, columns),
        OutputFormat::Csv => {
            render_csv(rows, entries, columns, quiet);
            Ok(())
        }
    }
}

fn render_tree(rows: &[Row], entries: &[Entry], ascii: bool) {
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
        println!("{prefix}{marker}{} {title}", entry.hrid);
    }
}

fn render_table(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    quiet: bool,
    filters: &Filters,
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
            let mut value = column.value(entry);
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
                .map(|row| strip_ansi(&row[idx]).len())
                .max()
                .unwrap_or(0)
                .max(header.len())
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
            let stripped_len = strip_ansi(value).len();
            let padding = width.saturating_sub(stripped_len);
            print!("{value}{:padding$}  ", "");
        }
        println!();
    }
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

fn render_json(rows: Vec<Row>, entries: &[Entry], columns: &[ListColumn]) -> anyhow::Result<()> {
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
        rows_out.push(build_serializable_row(entry, &selected_columns));
    }

    serde_json::to_writer_pretty(std::io::stdout(), &rows_out)
        .context("failed to render json output")?;
    println!();
    Ok(())
}

fn render_csv(rows: Vec<Row>, entries: &[Entry], columns: &[ListColumn], quiet: bool) {
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
            let value = csv_escape(&column.value(entry));
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

fn build_serializable_row<'a>(entry: &'a Entry, columns: &[ListColumn]) -> SerializableRow<'a> {
    let mut row = SerializableRow {
        hrid: entry.hrid.to_string(),
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
                        .map(|link| link.hrid.to_string())
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
                        .map(|link| link.hrid.to_string())
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

    fn value(self, entry: &Entry) -> String {
        match self {
            Self::Hrid => entry.hrid.to_string(),
            Self::Title => entry.title.clone().unwrap_or_default(),
            Self::Kind => entry.hrid.kind().to_string(),
            Self::Namespace => entry.hrid.namespace().join("-"),
            Self::Parents => entry
                .parents
                .iter()
                .map(|link| link.hrid.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Children => entry
                .children
                .iter()
                .map(|link| link.hrid.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Tags => entry.tags.join(", "),
            Self::Path => entry.path.display().to_string(),
            Self::Created => entry.created.to_rfc3339(),
        }
    }
}

fn parse_hrid(value: &str) -> Result<Hrid, String> {
    Hrid::try_from(value).map_err(|err| err.to_string())
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use chrono::{Duration, TimeZone, Utc};
    use regex::Regex;
    use requiem::{Directory, Requirement};
    use tempfile::tempdir;
    use uuid::Uuid;

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

    struct SampleEntries {
        entries: Vec<Entry>,
        index_by_uuid: HashMap<Uuid, usize>,
        root_uuid: Uuid,
        child_uuid: Uuid,
        leaf_uuid: Uuid,
    }

    impl SampleEntries {
        fn new() -> Self {
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

                    use requiem::domain::hrid::KindString;
                    Hrid::new_with_namespace(
                        vec![
                            KindString::new("SYSTEM".to_string()).unwrap(),
                            KindString::new("AUTH".to_string()).unwrap(),
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

        fn entries(&self) -> &[Entry] {
            &self.entries
        }

        fn index_map(&self) -> &HashMap<Uuid, usize> {
            &self.index_by_uuid
        }

        fn entry(&self, index: usize) -> &Entry {
            &self.entries[index]
        }

        fn root_index(&self) -> usize {
            self.index_by_uuid[&self.root_uuid]
        }

        fn child_index(&self) -> usize {
            self.index_by_uuid[&self.child_uuid]
        }

        fn leaf_index(&self) -> usize {
            self.index_by_uuid[&self.leaf_uuid]
        }

        fn leaf(&self) -> &Entry {
            self.entry(self.leaf_index())
        }
    }

    const fn row(index: usize, direction: Direction, depth: usize) -> Row {
        Row {
            index,
            direction,
            depth,
        }
    }

    fn empty_filters() -> Filters {
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

    fn add_requirement(directory: &mut Directory, kind: &str, content: &str) -> Requirement {
        directory
            .add_requirement(kind, content.to_string())
            .unwrap()
    }

    #[test]
    fn extract_title_prefers_headings_and_fallbacks() {
        assert_eq!(
            extract_title("# Heading\nBody"),
            Some("Heading".to_string())
        );
        assert_eq!(
            extract_title("  Plain content\nMore"),
            Some("Plain content".to_string())
        );
        assert_eq!(extract_title(""), None);
    }

    #[test]
    fn filters_match_combined_conditions() {
        let fixtures = SampleEntries::new();
        let leaf = fixtures.leaf();

        let build_filters = || Filters {
            kinds: vec!["usr".to_string()],
            namespaces: vec!["auth".to_string()],
            tags: vec!["security".to_string()],
            orphans: false,
            leaves: false,
            contains: Some("login".to_string()),
            regex: Some(Regex::new(r"(?i)login").unwrap()),
        };

        let filters = build_filters();
        assert!(filters.matches(leaf));

        let mut filters_with_orphans = build_filters();
        filters_with_orphans.orphans = true;
        assert!(!filters_with_orphans.matches(leaf));

        let mut filters_with_leaves = build_filters();
        filters_with_leaves.leaves = true;
        assert!(filters_with_leaves.matches(leaf));

        let mut filters_with_tags = build_filters();
        filters_with_tags.tags = vec!["missing".to_string()];
        assert!(!filters_with_tags.matches(leaf));
    }

    #[test]
    fn produce_direction_rows_walks_graph() {
        let fixtures = SampleEntries::new();
        let filters = empty_filters();
        let target = vec![fixtures.leaf_index()];

        let parents = produce_direction_rows(
            View::Ancestors,
            fixtures.entries(),
            fixtures.index_map(),
            &filters,
            &target,
            Some(2),
        );
        assert_eq!(parents.len(), 2);
        assert!(parents
            .iter()
            .any(|row| { fixtures.entries()[row.index].hrid.to_string() == "SYS-001" }));

        let children = produce_direction_rows(
            View::Descendants,
            fixtures.entries(),
            fixtures.index_map(),
            &filters,
            &[fixtures.root_index()],
            Some(3),
        );
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|row| {
            fixtures.entries()[row.index]
                .hrid
                .to_string()
                .contains("USR-007")
        }));
    }

    #[test]
    fn produce_tree_rows_and_context() {
        let fixtures = SampleEntries::new();
        let filters = empty_filters();

        let mut rows = produce_tree_rows(
            fixtures.entries(),
            fixtures.index_map(),
            &filters,
            &[fixtures.root_index()],
            Some(5),
        );
        assert!(rows.iter().any(|row| row.depth == 2));

        rows = augment_with_context(
            fixtures.entries(),
            fixtures.index_map(),
            vec![row(fixtures.child_index(), Direction::None, 0)],
            2,
        );

        assert!(rows
            .iter()
            .any(|row| { fixtures.entries()[row.index].hrid.to_string() == "SYS-001" }));
        assert!(rows.iter().any(|row| {
            fixtures.entries()[row.index]
                .hrid
                .to_string()
                .contains("USR-007")
        }));
    }

    #[test]
    fn append_unique_rows_avoids_duplicates() {
        let mut rows = vec![row(0, Direction::None, 0)];

        append_unique_rows(
            &mut rows,
            vec![row(0, Direction::None, 1), row(1, Direction::Down, 1)],
        );

        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn apply_sort_orders_by_fields() {
        let fixtures = SampleEntries::new();
        let entries = fixtures.entries();
        let rows = vec![
            row(fixtures.child_index(), Direction::None, 1),
            row(fixtures.root_index(), Direction::None, 0),
            row(fixtures.leaf_index(), Direction::None, 2),
        ];

        let ordered = apply_sort(rows.clone(), entries, SortField::Hrid);
        assert_eq!(entries[ordered[0].index].hrid.to_string(), "SYS-001");

        let ordered_kind = apply_sort(rows.clone(), entries, SortField::Kind);
        assert_eq!(entries[ordered_kind[0].index].hrid.kind(), "SYS");

        let ordered_title = apply_sort(rows.clone(), entries, SortField::Title);
        assert_eq!(
            entries[ordered_title[0].index].title.as_deref(),
            Some("Child")
        );

        let ordered_created = apply_sort(rows, entries, SortField::Created);
        assert_eq!(
            entries[ordered_created[0].index].hrid.to_string(),
            "SYS-001"
        );
    }

    #[test]
    fn apply_offset_limit_slices_rows() {
        let rows = vec![
            row(0, Direction::None, 0),
            row(1, Direction::None, 0),
            row(2, Direction::None, 0),
        ];

        let truncated = apply_offset_limit(rows.clone(), Some(1), Some(1));
        assert_eq!(truncated.len(), 1);
        assert_eq!(truncated[0].index, 1);

        let cleared = apply_offset_limit(rows, Some(5), None);
        assert!(cleared.is_empty());
    }

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
        )
        .unwrap();
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
        );

        assert_eq!(row.hrid, entry.hrid.to_string());
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

    #[test]
    fn parse_hrid_accepts_valid_values() {
        let hrid = parse_hrid("SYS-001").unwrap();
        assert_eq!(hrid.to_string(), "SYS-001");
    }

    #[test]
    fn output_format_display_matches_expected_strings() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Csv.to_string(), "csv");
    }

    #[test]
    fn resolve_depth_zero_becomes_unbounded() {
        assert_eq!(resolve_depth(Some(0), 2), usize::MAX);
        assert_eq!(resolve_depth(None, 3), 3);
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
