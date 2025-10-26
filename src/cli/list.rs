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
use requiem::{storage::directory::Loaded, Directory, Hrid, Requirement};
use serde::Serialize;
use tracing::instrument;
use uuid::Uuid;

const DEFAULT_LIMIT: usize = 200;

/// Command arguments for `req list`.
#[derive(Debug, Parser)]
#[command(about = "List requirements with filters and relationship views")]
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
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root).load_all()?;

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
            View::Tree => produce_tree_rows(
                &entries,
                &index_by_uuid,
                &filters,
                &target_indices,
                self.depth,
            ),
            View::Context => {
                let base = produce_base_rows(&entries, &filters, &target_indices);
                let depth = resolve_depth(self.depth, 1);
                augment_with_context(&entries, &index_by_uuid, base, depth)
            }
        };

        if self.view != View::Tree {
            rows = apply_sort(rows, &entries, self.sort);
        }

        let effective_limit = self
            .limit
            .and_then(|value| (value > 0).then_some(value))
            .or(Some(DEFAULT_LIMIT));

        rows = apply_offset_limit(rows, self.offset, effective_limit);

        render_rows(
            rows,
            &entries,
            &self.columns,
            self.output,
            self.quiet,
            self.view == View::Tree,
        )
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

fn collect_entries(directory: &Directory<Loaded>) -> Vec<Entry> {
    let mut entries = Vec::new();

    for requirement in directory.requirements() {
        entries.push(entry_from_requirement(directory, requirement));
    }

    entries
}

fn entry_from_requirement(directory: &Directory<Loaded>, requirement: &Requirement) -> Entry {
    let parents = requirement
        .parents()
        .map(|(uuid, parent)| LinkRef::new(uuid, parent.hrid.clone()))
        .collect::<Vec<_>>();

    let tags = requirement.tags().iter().cloned().collect::<Vec<_>>();
    let path = directory.path_for(requirement.hrid());

    Entry {
        uuid: requirement.uuid(),
        hrid: requirement.hrid().clone(),
        title: extract_title(requirement.content()),
        tags,
        created: requirement.created(),
        content: requirement.content().to_string(),
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
    let seeds = if target_indices.is_empty() {
        (0..entries.len()).collect::<Vec<_>>()
    } else {
        target_indices.to_vec()
    };

    let mut rows = Vec::new();
    let mut seed_set = HashSet::new();

    for &index in &seeds {
        if seed_set.insert(index) {
            rows.push(Row {
                index,
                direction: Direction::None,
                depth: 0,
            });
        }
    }

    let limit = resolve_depth(depth, usize::MAX);
    let descendants = traverse(
        seeds.iter().copied(),
        entries,
        index_by_uuid,
        limit,
        Direction::Down,
    );

    append_unique_rows(&mut rows, descendants);

    rows.retain(|row| seed_set.contains(&row.index) || filters.matches(&entries[row.index]));

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

fn render_rows(
    rows: Vec<Row>,
    entries: &[Entry],
    columns: &[ListColumn],
    output: OutputFormat,
    quiet: bool,
    tree: bool,
) -> anyhow::Result<()> {
    if tree {
        render_tree(rows, entries);
        return Ok(());
    }

    match output {
        OutputFormat::Table => {
            render_table(rows, entries, columns, quiet);
            Ok(())
        }
        OutputFormat::Json => render_json(rows, entries, columns),
        OutputFormat::Csv => {
            render_csv(rows, entries, columns, quiet);
            Ok(())
        }
    }
}

fn render_tree(rows: Vec<Row>, entries: &[Entry]) {
    for row in rows {
        let entry = &entries[row.index];
        let indent = "  ".repeat(row.depth);
        let marker = match row.direction {
            Direction::Up => "↑ ",
            Direction::Down => "↓ ",
            Direction::None => "",
        };
        let title = entry.title.as_deref().unwrap_or_default();
        println!("{indent}{marker}{} {title}", entry.hrid);
    }
}

fn render_table(rows: Vec<Row>, entries: &[Entry], columns: &[ListColumn], quiet: bool) {
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
                .map(|row| row[idx].len())
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
            print!("{value:<width$}  ");
        }
        println!();
    }
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
