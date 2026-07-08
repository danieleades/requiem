//! Row production: graph traversal, sorting, and pagination.

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
};

use uuid::Uuid;

use super::{entry::Entry, filter::Filters, SortField, View};

/// Relationship of a row to the primary targets.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(super) enum Direction {
    None,
    Up,
    Down,
}

/// A single result row: an entry index plus its relationship to the targets.
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub(super) index: usize,
    pub(super) direction: Direction,
    pub(super) depth: usize,
}

pub(super) fn produce_base_rows(
    entries: &[Entry],
    filters: &Filters,
    target_indices: &[usize],
) -> Vec<Row> {
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

pub(super) fn resolve_depth(depth: Option<usize>, default: usize) -> usize {
    depth.map_or(default, |value| if value == 0 { usize::MAX } else { value })
}

pub(super) fn produce_direction_rows(
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

pub(super) fn produce_tree_rows(
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

pub(super) fn augment_with_context(
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

pub(super) fn apply_sort(
    mut rows: Vec<Row>,
    entries: &[Entry],
    sort_field: SortField,
    digits: usize,
) -> Vec<Row> {
    rows.sort_by(|a, b| compare_rows(a, b, entries, sort_field, digits));
    rows
}

fn compare_rows(
    a: &Row,
    b: &Row,
    entries: &[Entry],
    sort_field: SortField,
    digits: usize,
) -> Ordering {
    let entry_a = &entries[a.index];
    let entry_b = &entries[b.index];

    let primary = match sort_field {
        SortField::Hrid => entry_a
            .hrid
            .display(digits)
            .to_string()
            .cmp(&entry_b.hrid.display(digits).to_string()),
        SortField::Kind => entry_a.hrid.kind().cmp(entry_b.hrid.kind()).then_with(|| {
            entry_a
                .hrid
                .display(digits)
                .to_string()
                .cmp(&entry_b.hrid.display(digits).to_string())
        }),
        SortField::Title => entry_a
            .title
            .as_deref()
            .unwrap_or_default()
            .cmp(entry_b.title.as_deref().unwrap_or_default())
            .then_with(|| {
                entry_a
                    .hrid
                    .display(digits)
                    .to_string()
                    .cmp(&entry_b.hrid.display(digits).to_string())
            }),
        SortField::Created => entry_a.created.cmp(&entry_b.created).then_with(|| {
            entry_a
                .hrid
                .display(digits)
                .to_string()
                .cmp(&entry_b.hrid.display(digits).to_string())
        }),
    };

    if primary == Ordering::Equal {
        a.depth.cmp(&b.depth)
    } else {
        primary
    }
}

/// Number of rows cut off by the limit, not counting rows skipped by
/// `--offset`.
pub(super) fn truncation_count(
    total_rows: usize,
    offset: Option<usize>,
    limit: Option<usize>,
) -> usize {
    let remaining = total_rows.saturating_sub(offset.unwrap_or(0));
    limit.map_or(0, |limit| remaining.saturating_sub(limit))
}

pub(super) fn apply_offset_limit(
    mut rows: Vec<Row>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vec<Row> {
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

#[cfg(test)]
mod tests {
    use super::{
        super::fixtures::{empty_filters, row, SampleEntries},
        *,
    };

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
            .any(|row| { fixtures.entries()[row.index].hrid.display(3).to_string() == "SYS-001" }));

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
                .display(3)
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
            .any(|row| { fixtures.entries()[row.index].hrid.display(3).to_string() == "SYS-001" }));
        assert!(rows.iter().any(|row| {
            fixtures.entries()[row.index]
                .hrid
                .display(3)
                .to_string()
                .contains("USR-007")
        }));
    }

    #[test]
    fn truncation_count_accounts_for_offset() {
        // 10 rows, no offset, limit 4: 6 cut off.
        assert_eq!(truncation_count(10, None, Some(4)), 6);
        // 10 rows, skip 3, limit 4: rows 4-7 shown, 3 cut off.
        assert_eq!(truncation_count(10, Some(3), Some(4)), 3);
        // Offset past the end: nothing left to cut off.
        assert_eq!(truncation_count(10, Some(20), Some(4)), 0);
        // Unlimited: never truncated.
        assert_eq!(truncation_count(10, Some(3), None), 0);
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

        let ordered = apply_sort(rows.clone(), entries, SortField::Hrid, 3);
        assert_eq!(
            entries[ordered[0].index].hrid.display(3).to_string(),
            "SYS-001"
        );

        let ordered_kind = apply_sort(rows.clone(), entries, SortField::Kind, 3);
        assert_eq!(entries[ordered_kind[0].index].hrid.kind(), "SYS");

        let ordered_title = apply_sort(rows.clone(), entries, SortField::Title, 3);
        assert_eq!(
            entries[ordered_title[0].index].title.as_deref(),
            Some("Child")
        );

        let ordered_created = apply_sort(rows, entries, SortField::Created, 3);
        assert_eq!(
            entries[ordered_created[0].index]
                .hrid
                .display(3)
                .to_string(),
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
    fn resolve_depth_zero_becomes_unbounded() {
        assert_eq!(resolve_depth(Some(0), 2), usize::MAX);
        assert_eq!(resolve_depth(None, 3), 3);
    }
}
