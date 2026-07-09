//! In-memory tree structure for requirements with decomposed storage
//!
//! The [`Tree`] knows nothing about the filesystem or the directory structure.
//! It stores requirements in a decomposed format for better maintainability and
//! performance.
//!
//! The core storage and accessors live here; related behaviour is split into
//! focused submodules:
//!
//! - `error`: error types for tree operations
//! - `link`: creating, removing, and renaming parent-child links
//! - `cycle`: cycle detection and prevention
//! - `suspect`: fingerprint-based change detection and HRID drift repair

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    num::NonZeroUsize,
};

use petgraph::graphmap::DiGraphMap;
use uuid::Uuid;

use crate::{
    domain::{
        hrid::{KindString, NamespaceSegment},
        requirement::Parent,
        requirement_data::RequirementData,
        requirement_view::RequirementView,
        Hrid,
    },
    Requirement,
};

mod cycle;
mod error;
mod link;
mod suspect;

pub use error::{AcceptLinkError, LinkError, LinkRequirementError, TreeInsertError};
pub use link::LinkOutcome;
pub use suspect::SuspectLink;

/// Data stored on each edge in the dependency graph.
///
/// Each edge represents a parent-child relationship, pointing from child to
/// parent. The edge stores the parent's HRID and the expected fingerprint for
/// change detection.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EdgeData {
    /// The HRID of the parent requirement at the time the link was created.
    /// This can become stale if the parent's HRID is changed.
    parent_hrid: Hrid,

    /// The fingerprint of the parent requirement at the time the link was
    /// created. Used to detect if the parent has been modified since the
    /// link was established.
    fingerprint: String,
}

/// An in-memory representation of the set of requirements with decomposed
/// storage.
///
/// Requirements are stored as separate components:
/// - Content data: `HashMap<Uuid, RequirementData>`
/// - HRIDs: `HashMap<Uuid, Hrid>` (separate to allow O(1) lookup)
/// - HRID lookup: `BTreeMap<Hrid, Uuid>`
/// - Relationships: `DiGraphMap<Uuid, EdgeData>` (edges are child→parent,
///   `EdgeData` contains parent info)
#[derive(Debug)]
pub struct Tree {
    /// Requirements data, keyed by UUID.
    requirements: HashMap<Uuid, RequirementData>,

    /// HRID for each requirement, keyed by UUID.
    /// Stored separately from `RequirementData` for efficient updates.
    hrids: HashMap<Uuid, Hrid>,

    /// Forward lookup map from HRID to UUID.
    /// `BTreeMap` for Hrid range lookups.
    hrid_to_uuid: BTreeMap<Hrid, Uuid>,

    /// Dependency graph. Nodes are UUIDs, edges point from child to parent.
    /// Edge data contains parent HRID and fingerprint for change detection.
    /// This is the sole source of truth for parent relationships.
    graph: DiGraphMap<Uuid, EdgeData>,
}

impl Default for Tree {
    fn default() -> Self {
        Self {
            requirements: HashMap::new(),
            hrids: HashMap::new(),
            hrid_to_uuid: BTreeMap::new(),
            graph: DiGraphMap::new(),
        }
    }
}

impl Tree {
    /// Creates a new tree with pre-allocated capacity for the given number of
    /// requirements.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            requirements: HashMap::with_capacity(capacity),
            hrids: HashMap::with_capacity(capacity),
            hrid_to_uuid: BTreeMap::new(),
            graph: DiGraphMap::with_capacity(capacity, capacity * 2),
        }
    }

    /// Inserts a requirement into the tree.
    ///
    /// # Errors
    ///
    /// Returns an error if a requirement with the same UUID or HRID already
    /// exists.
    pub fn insert(&mut self, requirement: Requirement) -> Result<(), TreeInsertError> {
        let uuid = requirement.metadata.uuid;

        // Check for duplicate UUID
        if self.requirements.contains_key(&uuid) {
            return Err(TreeInsertError::DuplicateUuid { uuid });
        }

        let hrid = requirement.metadata.hrid.clone();

        // Check for duplicate HRID
        if let Some(&existing_uuid) = self.hrid_to_uuid.get(&hrid) {
            return Err(TreeInsertError::DuplicateHrid {
                hrid,
                new_uuid: uuid,
                existing_uuid,
            });
        }

        // Add node to graph (if it doesn't already exist from being referenced as a
        // parent)
        self.graph.add_node(uuid);

        // Add edges for parent relationships
        // Note: add_edge() will automatically create parent nodes if they don't exist
        // yet
        for (parent_uuid, parent_info) in &requirement.metadata.parents {
            let edge_data = EdgeData {
                parent_hrid: parent_info.hrid.clone(),
                fingerprint: parent_info.fingerprint.clone(),
            };
            self.graph.add_edge(uuid, *parent_uuid, edge_data);
        }

        // Store HRID
        self.hrids.insert(uuid, hrid.clone());
        self.hrid_to_uuid.insert(hrid, uuid);

        // Store decomposed data
        let data = RequirementData::from(requirement);
        self.requirements.insert(uuid, data);

        Ok(())
    }

    /// Retrieves just the HRID for a requirement by UUID.
    ///
    /// This is more efficient than `requirement()` when only the HRID is
    /// needed.
    #[must_use]
    pub fn hrid(&self, uuid: Uuid) -> Option<&Hrid> {
        self.hrids.get(&uuid)
    }

    /// Retrieves a requirement by UUID as an owned Requirement.
    ///
    /// This is more efficient than calling `requirement().to_requirement()`
    /// when you need an owned Requirement, as it avoids creating the
    /// intermediate view.
    #[must_use]
    pub fn get_requirement(&self, uuid: Uuid) -> Option<Requirement> {
        let data = self.requirements.get(&uuid)?;
        let hrid = self.hrids.get(&uuid)?;

        let parents = self.parent_links(uuid).into_iter().collect();

        Some(Requirement {
            content: crate::domain::requirement::Content {
                title: data.title.clone(),
                body: data.body.clone(),
                tags: data.tags.clone(),
            },
            metadata: crate::domain::requirement::Metadata {
                uuid,
                hrid: hrid.clone(),
                created: data.created,
                parents,
            },
        })
    }

    /// Updates the content of the requirement with the given UUID.
    ///
    /// Fields passed as `None` are left unchanged. Returns `None` when no
    /// requirement with this UUID exists, otherwise whether any field actually
    /// changed.
    pub fn update_requirement_content(
        &mut self,
        uuid: Uuid,
        title: Option<String>,
        body: Option<String>,
        tags: Option<BTreeSet<String>>,
    ) -> Option<bool> {
        let data = self.requirements.get_mut(&uuid)?;

        let mut changed = false;
        if let Some(title) = title {
            if data.title != title {
                data.title = title;
                changed = true;
            }
        }
        if let Some(body) = body {
            if data.body != body {
                data.body = body;
                changed = true;
            }
        }
        if let Some(tags) = tags {
            if data.tags != tags {
                data.tags = tags;
                changed = true;
            }
        }

        Some(changed)
    }

    /// Retrieves a requirement by UUID as a borrowed view.
    ///
    /// Note: Since UUID is passed by value, we need to find a way to get a
    /// reference to it. The UUID is stored as a key in the requirements
    /// `HashMap`.
    #[must_use]
    pub fn requirement(&self, uuid: Uuid) -> Option<RequirementView<'_>> {
        let data = self.requirements.get(&uuid)?;
        let hrid = self.hrids.get(&uuid)?;

        let parents = self.parent_links(uuid);
        let children = self.child_uuids(uuid);

        // Get a reference to the UUID from the requirements HashMap key
        // This is safe because we know it exists (we just got data from it)
        let uuid_ref = self.requirements.get_key_value(&uuid)?.0;

        Some(RequirementView {
            uuid: uuid_ref,
            hrid,
            created: &data.created,
            title: &data.title,
            body: &data.body,
            tags: &data.tags,
            parents,
            children,
        })
    }

    /// Returns the next available index for a requirement of the given
    /// namespace and kind.
    ///
    /// This method uses a range query on the `hrid_to_uuid` `BTreeMap` to find
    /// the maximum ID for the given namespace and kind combination. Time
    /// complexity is O(log n) where n is the total number of requirements.
    ///
    /// # Panics
    ///
    /// Panics if the next requirement ID would overflow `usize`.
    #[must_use]
    pub fn next_index(&self, namespace: &[NamespaceSegment], kind: &KindString) -> NonZeroUsize {
        // Construct range bounds for this namespace+kind combination
        // Start: namespace+kind with ID 1 (MIN), End: namespace+kind with ID MAX
        let start = crate::domain::Hrid::new_with_namespace(
            namespace.to_vec(),
            kind.clone(),
            NonZeroUsize::MIN,
        );
        let end = crate::domain::Hrid::new_with_namespace(
            namespace.to_vec(),
            kind.clone(),
            NonZeroUsize::MAX,
        );

        // Use range query to find all HRIDs of this namespace+kind, then get the last
        // one
        self.hrid_to_uuid
            .range(start..=end)
            .next_back()
            .map_or(NonZeroUsize::MIN, |(hrid, _)| {
                hrid.id().checked_add(1).expect("requirement ID overflow!")
            })
    }

    /// Returns an iterator over all requirements in the tree as borrowed views.
    pub fn iter(&self) -> impl Iterator<Item = RequirementView<'_>> + '_ {
        self.requirements.iter().filter_map(move |(uuid, data)| {
            let hrid = self.hrids.get(uuid)?;

            Some(RequirementView {
                uuid,
                hrid,
                created: &data.created,
                title: &data.title,
                body: &data.body,
                tags: &data.tags,
                parents: self.parent_links(*uuid),
                children: self.child_uuids(*uuid),
            })
        })
    }

    /// Collect the parent links of a node from its outgoing graph edges.
    fn parent_links(&self, uuid: Uuid) -> Vec<(Uuid, Parent)> {
        self.graph
            .edges(uuid)
            .map(|(_, parent_uuid, edge_data)| {
                (
                    parent_uuid,
                    Parent {
                        hrid: edge_data.parent_hrid.clone(),
                        fingerprint: edge_data.fingerprint.clone(),
                    },
                )
            })
            .collect()
    }

    /// Collect the children of a node from its incoming graph edges.
    fn child_uuids(&self, uuid: Uuid) -> Vec<Uuid> {
        self.graph
            .edges_directed(uuid, petgraph::Direction::Incoming)
            .map(|(child_uuid, _, _)| child_uuid)
            .collect()
    }

    /// Finds a requirement by its human-readable identifier.
    #[must_use]
    pub fn find_by_hrid(&self, hrid: &Hrid) -> Option<RequirementView<'_>> {
        let uuid = self.hrid_to_uuid.get(hrid)?;
        self.requirement(*uuid)
    }

    /// Remove a requirement from the tree.
    ///
    /// This removes the requirement node and all its edges (both incoming and
    /// outgoing). Parent requirements will have this requirement removed
    /// from their children lists. Child requirements will have this
    /// requirement removed from their parent lists.
    ///
    /// # Errors
    ///
    /// Returns an error if the requirement doesn't exist.
    pub fn remove_requirement(&mut self, uuid: Uuid) -> anyhow::Result<()> {
        // Check if requirement exists
        if !self.requirements.contains_key(&uuid) {
            anyhow::bail!("Requirement with UUID {uuid} not found");
        }

        // Get HRID before removing
        let Some(hrid) = self.hrids.get(&uuid).cloned() else {
            anyhow::bail!("Requirement UUID {uuid} has no HRID mapping");
        };

        // Remove all edges connected to this node
        self.graph.remove_node(uuid);

        // Remove from requirements map
        self.requirements.remove(&uuid);

        // Remove from HRID map
        self.hrid_to_uuid.remove(&hrid);
        self.hrids.remove(&uuid);

        Ok(())
    }

    /// Get all children of a requirement.
    #[must_use]
    pub fn children(&self, uuid: Uuid) -> Vec<Uuid> {
        if !self.graph.contains_node(uuid) {
            return Vec::new();
        }

        // Incoming edges are from children
        self.graph
            .neighbors_directed(uuid, petgraph::Direction::Incoming)
            .collect()
    }

    /// Get all parents of a requirement with their fingerprints.
    #[must_use]
    pub fn parents(&self, uuid: Uuid) -> Vec<(Uuid, String)> {
        if !self.graph.contains_node(uuid) {
            return Vec::new();
        }

        // Outgoing edges are to parents
        self.graph
            .edges(uuid)
            .map(|(_, parent_uuid, edge_data)| (parent_uuid, edge_data.fingerprint.clone()))
            .collect()
    }

    /// Return all ancestors (transitive parents) of a requirement as UUIDs.
    ///
    /// The result is deduplicated; order is deterministic.
    #[must_use]
    pub fn ancestors_of(&self, uuid: Uuid) -> Vec<Uuid> {
        if !self.graph.contains_node(uuid) {
            return Vec::new();
        }

        let mut visited: BTreeSet<Uuid> = BTreeSet::new();
        let mut queue: VecDeque<Uuid> = self
            .graph
            .edges(uuid)
            .map(|(_, parent_uuid, _)| parent_uuid)
            .collect();

        while let Some(next) = queue.pop_front() {
            if !visited.insert(next) {
                continue;
            }

            for (_, parent_uuid, _) in self.graph.edges(next) {
                queue.push_back(parent_uuid);
            }
        }

        visited.into_iter().collect()
    }

    /// Return all descendants (transitive children) of a requirement as UUIDs.
    ///
    /// The result is deduplicated; order is deterministic.
    #[must_use]
    pub fn descendants_of(&self, uuid: Uuid) -> Vec<Uuid> {
        if !self.graph.contains_node(uuid) {
            return Vec::new();
        }

        let mut visited: BTreeSet<Uuid> = BTreeSet::new();
        let mut queue: VecDeque<Uuid> = self
            .graph
            .neighbors_directed(uuid, petgraph::Direction::Incoming)
            .collect();

        while let Some(next) = queue.pop_front() {
            if !visited.insert(next) {
                continue;
            }

            for child_uuid in self
                .graph
                .neighbors_directed(next, petgraph::Direction::Incoming)
            {
                queue.push_back(child_uuid);
            }
        }

        visited.into_iter().collect()
    }
}
