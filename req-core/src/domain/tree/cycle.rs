//! Cycle detection and prevention for the requirement graph.

use uuid::Uuid;

use super::Tree;
use crate::domain::Hrid;

/// Vertex coloring state for cycle detection using the three-color DFS
/// algorithm.
///
/// This implements a standard graph algorithm for detecting cycles in directed
/// graphs. The three states track DFS traversal:
///
/// - **Gray**: The node is currently being visited (appears in the current
///   recursion stack). A back edge to a Gray node indicates a cycle.
/// - **Black**: The node has been completely processed (finished recursing on
///   all descendants). A back edge to a Black node is a cross edge, not a
///   cycle.
/// - **Unvisited**: Nodes not in the map have never been visited.
///
/// # Algorithm
///
/// This is the standard three-color depth-first search algorithm:
/// 1. Mark node Gray when entering (push to recursion stack)
/// 2. Recursively visit unvisited neighbors
/// 3. If we encounter a Gray ancestor, we've found a cycle (back edge)
/// 4. Mark node Black when leaving (pop from recursion stack)
///
/// This allows us to detect cycles in a single O(V+E) pass while simultaneously
/// extracting the complete cycle paths (e.g., `[A, B, C, A]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DfsColorForDetection {
    /// Node is currently being visited (on the recursion stack).
    /// A back edge to a Gray node indicates a cycle.
    Gray,
    /// Node has been completely finished. Edges to Black nodes are cross edges,
    /// not cycles.
    Black,
    // Unvisited nodes are represented by the absence of an entry in the HashMap.
}

impl Tree {
    /// Detect all cycles in the requirement graph using three-color depth-first
    /// search.
    ///
    /// Returns a list of cycles, where each cycle is represented as a path of
    /// HRIDs that forms a loop (e.g., `vec![USR-001, SYS-002, USR-001]`).
    ///
    /// # Algorithm
    ///
    /// Uses the standard three-color DFS algorithm to detect back edges in
    /// O(V+E) time. When a back edge to a Gray ancestor is found, the full
    /// cycle path is extracted and deduplicated before being added to the
    /// result.
    ///
    /// # Example
    ///
    /// If the graph has edges: `A→B→C→A`, this returns `vec![vec![A, B, C, A]]`
    #[must_use]
    pub fn detect_cycles(&self) -> Vec<Vec<Hrid>> {
        use std::collections::HashMap;

        let mut colors: HashMap<Uuid, DfsColorForDetection> = HashMap::new();
        let mut cycles = Vec::new();

        // Start DFS from each unvisited node to ensure we find all cycles
        for start_node in self.graph.nodes() {
            if !colors.contains_key(&start_node) {
                self.dfs_detect_cycles(start_node, &mut colors, &mut cycles);
            }
        }

        cycles
    }

    /// Iterative helper for three-color DFS cycle detection.
    ///
    /// Each stack frame holds a node, its outgoing neighbours, and the index
    /// of the next neighbour to visit, so deep parent chains cannot overflow
    /// the call stack. The current DFS path is maintained to extract full
    /// cycle information when back edges (edges to Gray nodes) are found.
    fn dfs_detect_cycles(
        &self,
        start: Uuid,
        colors: &mut std::collections::HashMap<Uuid, DfsColorForDetection>,
        cycles: &mut Vec<Vec<Hrid>>,
    ) {
        use self::DfsColorForDetection::{Black, Gray};

        let neighbours_of =
            |node: Uuid| -> Vec<Uuid> { self.graph.edges(node).map(|(_, p, _)| p).collect() };

        let mut path: Vec<Uuid> = Vec::new();
        let mut stack: Vec<(Uuid, Vec<Uuid>, usize)> = Vec::new();

        colors.insert(start, Gray);
        path.push(start);
        stack.push((start, neighbours_of(start), 0));

        while let Some(frame) = stack.last_mut() {
            let node = frame.0;
            let next = frame.1.get(frame.2).copied();
            frame.2 += 1;

            let Some(parent_uuid) = next else {
                // All outgoing edges visited: mark Black and leave the node.
                colors.insert(node, Black);
                path.pop();
                stack.pop();
                continue;
            };

            match colors.get(&parent_uuid) {
                Some(Gray) => {
                    // Back edge found! This node is an ancestor in the current path.
                    // Extract and record the cycle.
                    if let Some(pos) = path.iter().position(|&u| u == parent_uuid) {
                        let cycle_path: Vec<Hrid> = path[pos..]
                            .iter()
                            .chain(std::iter::once(&parent_uuid))
                            .filter_map(|&uuid| self.hrids.get(&uuid).cloned())
                            .collect();
                        if !cycle_path.is_empty()
                            && !cycles.iter().any(|c| Self::cycles_equal(c, &cycle_path))
                        {
                            cycles.push(cycle_path);
                        }
                    }
                }
                Some(Black) => {
                    // Already processed this subtree, skip (cross edge)
                }
                None => {
                    // Unvisited node, descend
                    colors.insert(parent_uuid, Gray);
                    path.push(parent_uuid);
                    stack.push((parent_uuid, neighbours_of(parent_uuid), 0));
                }
            }
        }
    }

    /// Check if two cycles are equivalent (same HRIDs, possibly rotated).
    ///
    /// Cycle paths are closed walks (`[A, B, C, A]`): the comparison drops the
    /// closing element that repeats the start, then tests whether one sequence
    /// is a rotation of the other.
    fn cycles_equal(a: &[Hrid], b: &[Hrid]) -> bool {
        fn open(cycle: &[Hrid]) -> &[Hrid] {
            match cycle {
                [rest @ .., last] if rest.first() == Some(last) => rest,
                _ => cycle,
            }
        }

        let a = open(a);
        let b = open(b);
        if a.len() != b.len() {
            return false;
        }
        a.is_empty()
            || (0..b.len()).any(|offset| (0..a.len()).all(|i| a[i] == b[(offset + i) % b.len()]))
    }

    /// Check if adding a link from child to parent would create a cycle.
    ///
    /// Returns `Ok(())` if the link is safe, or `Err()` with the cycle path
    /// that would be created.
    ///
    /// # Errors
    ///
    /// Returns an error if creating the link would form a cycle in the graph.
    pub fn check_would_create_cycle(
        &self,
        child_uuid: Uuid,
        parent_uuid: Uuid,
    ) -> anyhow::Result<()> {
        // If the parent can reach the child in the graph, then child -> parent would
        // create a cycle
        if self.can_reach(parent_uuid, child_uuid) {
            // Find the cycle path for error reporting
            let cycle_path = self.find_cycle_path(parent_uuid, child_uuid);
            let cycle_str = cycle_path
                .iter()
                .filter_map(|&uuid| self.hrids.get(&uuid).map(|h| format!("{}", h.display(3))))
                .collect::<Vec<_>>()
                .join(" → ");

            let child_hrid = self.hrids.get(&child_uuid).map_or_else(
                || format!("(UUID: {child_uuid})"),
                |h| format!("{}", h.display(3)),
            );

            anyhow::bail!("Cannot create link: would form a cycle: {cycle_str} → {child_hrid}");
        }
        Ok(())
    }

    /// Check if there's a path from source to target in the graph.
    fn can_reach(&self, source: Uuid, target: Uuid) -> bool {
        use std::collections::{HashSet, VecDeque};

        if !self.graph.contains_node(source) {
            return false;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source);
        visited.insert(source);

        while let Some(node) = queue.pop_front() {
            if node == target {
                return true;
            }

            for (_, parent_uuid, _) in self.graph.edges(node) {
                if visited.insert(parent_uuid) {
                    queue.push_back(parent_uuid);
                }
            }
        }

        false
    }

    /// Find a path from source to target for cycle reporting.
    fn find_cycle_path(&self, source: Uuid, target: Uuid) -> Vec<Uuid> {
        use std::collections::{HashMap, VecDeque};

        let mut parent_map: HashMap<Uuid, Uuid> = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(source);
        parent_map.insert(source, source);

        while let Some(node) = queue.pop_front() {
            if node == target {
                // Reconstruct path
                let mut path = vec![target];
                let mut current = target;
                while current != source {
                    current = parent_map[&current];
                    path.push(current);
                }
                path.reverse();
                return path;
            }

            for (_, parent_uuid, _) in self.graph.edges(node) {
                if let std::collections::hash_map::Entry::Vacant(e) = parent_map.entry(parent_uuid)
                {
                    e.insert(node);
                    queue.push_back(parent_uuid);
                }
            }
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Requirement;

    #[test]
    fn test_detect_cycles_finds_direct_cycle() {
        let mut tree = Tree::default();

        // Create two requirements
        let req_a = Requirement::new("USR-001".parse().unwrap(), "A".to_string(), String::new());
        let uuid_a = req_a.uuid();
        tree.insert(req_a).unwrap();

        let req_b = Requirement::new("USR-002".parse().unwrap(), "B".to_string(), String::new());
        let uuid_b = req_b.uuid();
        tree.insert(req_b).unwrap();

        // Create A → B and B → A (cycle)
        tree.upsert_parent_link(uuid_a, uuid_b, "fingerprint".to_string())
            .unwrap();
        tree.upsert_parent_link(uuid_b, uuid_a, "fingerprint".to_string())
            .unwrap();

        // Detect cycles
        let cycles = tree.detect_cycles();
        assert!(!cycles.is_empty(), "Should detect at least one cycle");
        assert_eq!(cycles.len(), 1, "Should detect exactly one cycle (A→B→A)");
    }

    #[test]
    fn test_check_would_create_cycle_detects_prevention() {
        let mut tree = Tree::default();

        // Create three requirements: A → B → C
        let req_a = Requirement::new("USR-001".parse().unwrap(), "A".to_string(), String::new());
        let uuid_a = req_a.uuid();
        tree.insert(req_a).unwrap();

        let req_b = Requirement::new("USR-002".parse().unwrap(), "B".to_string(), String::new());
        let uuid_b = req_b.uuid();
        tree.insert(req_b).unwrap();

        let req_c = Requirement::new("USR-003".parse().unwrap(), "C".to_string(), String::new());
        let uuid_c = req_c.uuid();
        tree.insert(req_c).unwrap();

        let req_d = Requirement::new("USR-004".parse().unwrap(), "D".to_string(), String::new());
        let uuid_d = req_d.uuid();
        tree.insert(req_d).unwrap();

        // Create A → B and B → C
        tree.upsert_parent_link(uuid_a, uuid_b, "fingerprint".to_string())
            .unwrap();
        tree.upsert_parent_link(uuid_b, uuid_c, "fingerprint".to_string())
            .unwrap();

        // Attempting to create C → A should fail (would create A→B→C→A cycle)
        let result = tree.check_would_create_cycle(uuid_c, uuid_a);
        assert!(
            result.is_err(),
            "Should detect that C→A would create a cycle"
        );

        // Attempting to create C → B should fail (would create B→C→B cycle)
        let result = tree.check_would_create_cycle(uuid_c, uuid_b);
        assert!(
            result.is_err(),
            "Should detect that C→B would create a cycle"
        );

        // Attempting to create D → A should succeed (no cycle)
        let result = tree.check_would_create_cycle(uuid_d, uuid_a);
        assert!(
            result.is_ok(),
            "Should allow D→A since it doesn't create a cycle"
        );
    }

    #[test]
    fn test_check_would_create_cycle_simple_reverse() {
        // Test the specific scenario from the review comment:
        // B → A exists (B depends on A)
        // Try to add A → B (would create a cycle)
        // This should FAIL but currently SUCCEEDS due to wrong traversal direction
        let mut tree = Tree::default();

        let req_a = Requirement::new("USR-001".parse().unwrap(), "A".to_string(), String::new());
        let uuid_a = req_a.uuid();
        tree.insert(req_a).unwrap();

        let req_b = Requirement::new("USR-002".parse().unwrap(), "B".to_string(), String::new());
        let uuid_b = req_b.uuid();
        tree.insert(req_b).unwrap();

        // Create B → A (B depends on A)
        tree.upsert_parent_link(uuid_b, uuid_a, "fp".to_string())
            .unwrap();

        // Now try to add A → B (which would create cycle A → B → A)
        let result = tree.check_would_create_cycle(uuid_a, uuid_b);
        assert!(
            result.is_err(),
            "Should detect that A→B would create cycle (B→A already exists)"
        );
    }

    #[test]
    fn test_check_would_create_cycle_detects_ancestor_to_descendant() {
        let mut tree = Tree::default();

        // Create chain: A → B → C → D
        let req_a = Requirement::new("USR-001".parse().unwrap(), "A".to_string(), String::new());
        let uuid_a = req_a.uuid();
        tree.insert(req_a).unwrap();

        let req_b = Requirement::new("USR-002".parse().unwrap(), "B".to_string(), String::new());
        let uuid_b = req_b.uuid();
        tree.insert(req_b).unwrap();

        let req_c = Requirement::new("USR-003".parse().unwrap(), "C".to_string(), String::new());
        let uuid_c = req_c.uuid();
        tree.insert(req_c).unwrap();

        let req_d = Requirement::new("USR-004".parse().unwrap(), "D".to_string(), String::new());
        let uuid_d = req_d.uuid();
        tree.insert(req_d).unwrap();

        // Create A → B, B → C, C → D
        tree.upsert_parent_link(uuid_a, uuid_b, "fp".to_string())
            .unwrap();
        tree.upsert_parent_link(uuid_b, uuid_c, "fp".to_string())
            .unwrap();
        tree.upsert_parent_link(uuid_c, uuid_d, "fp".to_string())
            .unwrap();

        // This test case specifically targets the bug in can_reach:
        // Trying to link D → A should fail because A already reaches D
        let result = tree.check_would_create_cycle(uuid_d, uuid_a);
        assert!(
            result.is_err(),
            "Should detect that D→A would create cycle (A→B→C→D→A)"
        );
    }

    #[test]
    fn cycles_equal_detects_wrapped_rotations() {
        fn cycle(ids: &[&str]) -> Vec<Hrid> {
            ids.iter().map(|s| s.parse().unwrap()).collect()
        }

        // The same cycle recorded from different entry points: rotations that
        // wrap around the start must compare equal.
        let from_a = cycle(&["USR-001", "USR-002", "USR-003", "USR-001"]);
        let from_b = cycle(&["USR-002", "USR-003", "USR-001", "USR-002"]);
        let from_c = cycle(&["USR-003", "USR-001", "USR-002", "USR-003"]);
        assert!(Tree::cycles_equal(&from_a, &from_b));
        assert!(Tree::cycles_equal(&from_a, &from_c));

        // Same length and membership but opposite direction: not equal.
        let reversed = cycle(&["USR-001", "USR-003", "USR-002", "USR-001"]);
        assert!(!Tree::cycles_equal(&from_a, &reversed));

        // Self-loops.
        assert!(Tree::cycles_equal(
            &cycle(&["USR-001", "USR-001"]),
            &cycle(&["USR-001", "USR-001"])
        ));
        assert!(!Tree::cycles_equal(
            &cycle(&["USR-001", "USR-001"]),
            &cycle(&["USR-002", "USR-002"])
        ));
    }

    #[test]
    fn detect_cycles_survives_deep_chains() {
        // A linear parent chain deep enough to overflow the call stack if
        // cycle detection were recursive.
        const DEPTH: usize = 20_000;

        let mut tree = Tree::default();
        let mut uuids = Vec::with_capacity(DEPTH);
        for i in 1..=DEPTH {
            let req = Requirement::new(
                format!("USR-{i}").parse().unwrap(),
                String::new(),
                String::new(),
            );
            uuids.push(req.uuid());
            tree.insert(req).unwrap();
        }
        for pair in uuids.windows(2) {
            tree.upsert_parent_link(pair[0], pair[1], "fp".to_string())
                .unwrap();
        }

        assert!(tree.detect_cycles().is_empty());
    }
}
