//! New in-memory tree structure for requirements with decomposed storage
//!
//! The [`Tree`] knows nothing about the filesystem or the directory structure.
//! It stores requirements in a decomposed format for better maintainability and
//! performance.

use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroUsize,
};

use petgraph::graphmap::DiGraphMap;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    domain::{
        hrid::KindString,
        requirement::{LoadError, Parent},
        requirement_data::RequirementData,
        requirement_view::RequirementView,
        Hrid,
    },
    Requirement,
};

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

/// Result of linking two requirements together.
#[derive(Debug)]
pub struct LinkOutcome {
    /// UUID of the child requirement.
    pub child_uuid: Uuid,
    /// HRID of the child requirement.
    pub child_hrid: Hrid,
    /// UUID of the parent requirement.
    pub parent_uuid: Uuid,
    /// HRID of the parent requirement.
    pub parent_hrid: Hrid,
    /// Whether the relationship already existed prior to linking.
    pub already_linked: bool,
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
    /// # Panics
    ///
    /// Panics if a requirement with the same UUID already exists.
    pub fn insert(&mut self, requirement: Requirement) {
        let uuid = requirement.metadata.uuid;
        assert!(
            !self.requirements.contains_key(&uuid),
            "Duplicate requirement UUID: {uuid}"
        );

        let hrid = requirement.metadata.hrid.clone();

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
        use std::collections::HashMap;

        let data = self.requirements.get(&uuid)?;
        let hrid = self.hrids.get(&uuid)?;

        // Reconstruct parents from graph edges
        let parents: HashMap<Uuid, Parent> = self
            .graph
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
            .collect();

        Some(Requirement {
            content: crate::domain::requirement::Content {
                content: data.content.clone(),
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

    /// Retrieves a requirement by UUID as a borrowed view.
    ///
    /// Note: Since UUID is passed by value, we need to find a way to get a
    /// reference to it. The UUID is stored as a key in the requirements
    /// `HashMap`.
    #[must_use]
    pub fn requirement(&self, uuid: Uuid) -> Option<RequirementView<'_>> {
        let data = self.requirements.get(&uuid)?;
        let hrid = self.hrids.get(&uuid)?;

        // Reconstruct parent data from graph edges
        // Since RequirementView owns the parent data, we can collect directly into Vec
        let parents: Vec<(Uuid, Parent)> = self
            .graph
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
            .collect();

        // Get a reference to the UUID from the requirements HashMap key
        // This is safe because we know it exists (we just got data from it)
        let uuid_ref = self.requirements.get_key_value(&uuid)?.0;

        Some(RequirementView {
            uuid: uuid_ref,
            hrid,
            created: &data.created,
            content: &data.content,
            tags: &data.tags,
            parents,
        })
    }

    /// Returns the next available index for a requirement of the given kind.
    ///
    /// This method uses a range query on the `hrid_to_uuid` `BTreeMap` to find
    /// the maximum ID for the given kind. Time complexity is O(log n) where n
    /// is the total number of requirements.
    ///
    /// The input `kind` will be normalized to uppercase.
    ///
    /// # Panics
    ///
    /// Panics if the provided kind is invalid (empty or contains non-alphabetic
    /// characters).
    #[must_use]
    pub fn next_index(&self, kind: &KindString) -> NonZeroUsize {
        // Construct range bounds for this kind
        // Start: kind with ID 1 (MIN), End: kind with ID MAX
        let start =
            crate::domain::Hrid::new_with_namespace(Vec::new(), kind.clone(), NonZeroUsize::MIN);
        let end =
            crate::domain::Hrid::new_with_namespace(Vec::new(), kind.clone(), NonZeroUsize::MAX);

        // Use range query to find all HRIDs of this kind, then get the last one
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

            let parents: Vec<(Uuid, Parent)> = self
                .graph
                .edges(*uuid)
                .map(|(_, parent_uuid, edge_data)| {
                    (
                        parent_uuid,
                        Parent {
                            hrid: edge_data.parent_hrid.clone(),
                            fingerprint: edge_data.fingerprint.clone(),
                        },
                    )
                })
                .collect();

            Some(RequirementView {
                uuid,
                hrid,
                created: &data.created,
                content: &data.content,
                tags: &data.tags,
                parents,
            })
        })
    }

    /// Finds a requirement by its human-readable identifier.
    #[must_use]
    pub fn find_by_hrid(&self, hrid: &Hrid) -> Option<RequirementView<'_>> {
        let uuid = self.hrid_to_uuid.get(hrid)?;
        self.requirement(*uuid)
    }

    /// Link two requirements identified by their HRIDs.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::NotFound`] when either HRID does not exist in the
    /// tree.
    pub fn link_requirement(
        &mut self,
        child: &Hrid,
        parent: &Hrid,
    ) -> Result<LinkOutcome, LoadError> {
        let (child_uuid, child_hrid) = {
            let view = self.find_by_hrid(child).ok_or(LoadError::NotFound)?;
            (*view.uuid, view.hrid.clone())
        };

        let (parent_uuid, parent_hrid, parent_fingerprint) = {
            let view = self.find_by_hrid(parent).ok_or(LoadError::NotFound)?;
            (*view.uuid, view.hrid.clone(), view.fingerprint())
        };

        let already_linked = self
            .parents(child_uuid)
            .into_iter()
            .any(|(uuid, _)| uuid == parent_uuid);

        self.upsert_parent_link(child_uuid, parent_uuid, parent_fingerprint);

        Ok(LinkOutcome {
            child_uuid,
            child_hrid,
            parent_uuid,
            parent_hrid,
            already_linked,
        })
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

    /// Insert or update a parent link for the given child UUID.
    ///
    /// Returns `true` if an existing link was replaced, or `false` if a new
    /// link was created.
    ///
    /// # Panics
    ///
    /// Panics if either the child or parent UUID does not exist in the tree.
    pub fn upsert_parent_link(
        &mut self,
        child_uuid: Uuid,
        parent_uuid: Uuid,
        fingerprint: String,
    ) -> bool {
        assert!(
            self.requirements.contains_key(&child_uuid),
            "Child requirement {child_uuid} not found in tree"
        );
        assert!(
            self.requirements.contains_key(&parent_uuid),
            "Parent requirement {parent_uuid} not found in tree"
        );

        // Ensure both nodes exist in the graph; GraphMap::add_node is idempotent.
        self.graph.add_node(child_uuid);
        self.graph.add_node(parent_uuid);

        let parent_hrid = self
            .hrids
            .get(&parent_uuid)
            .unwrap_or_else(|| panic!("Parent HRID for {parent_uuid} not found"));

        let edge = EdgeData {
            parent_hrid: parent_hrid.clone(),
            fingerprint,
        };

        self.graph.add_edge(child_uuid, parent_uuid, edge).is_some()
    }

    /// Read all the requirements and update any incorrect parent HRIDs.
    /// Returns an iterator of UUIDs whose parents were updated.
    ///
    /// # Panics
    ///
    /// Panics if a requirement references a parent UUID that doesn't exist in
    /// the tree, or if a requirement is its own parent.
    #[instrument(skip(self))]
    pub fn update_hrids(&mut self) -> impl Iterator<Item = Uuid> + '_ {
        use std::collections::HashSet;

        let mut updated_uuids = HashSet::new();

        // Collect all edges that need updating (store edge identifiers, not cloned
        // HRIDs)
        let mut edges_to_update = Vec::new();

        for child_uuid in self.graph.nodes() {
            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Get the current HRID of the parent
                let current_parent_hrid = self
                    .hrids
                    .get(&parent_uuid)
                    .unwrap_or_else(|| panic!("Parent requirement {parent_uuid} not found!"));

                // Check if the stored HRID is outdated
                if &edge_data.parent_hrid != current_parent_hrid {
                    edges_to_update.push((child_uuid, parent_uuid));
                    updated_uuids.insert(child_uuid);
                }
            }
        }

        // Apply the updates to EdgeData only
        for (child_uuid, parent_uuid) in edges_to_update {
            // Look up the HRID again (HashMap lookup is O(1) and cheaper than cloning
            // earlier)
            let current_parent_hrid = self.hrids.get(&parent_uuid).unwrap();
            if let Some(edge_data) = self.graph.edge_weight_mut(child_uuid, parent_uuid) {
                // Update EdgeData (the sole source of truth)
                edge_data.parent_hrid = current_parent_hrid.clone();
            }
        }

        updated_uuids.into_iter()
    }

    /// Find all suspect links in the requirement graph.
    ///
    /// A link is suspect when the fingerprint stored in the edge data
    /// does not match the current fingerprint of the parent requirement.
    ///
    /// # Panics
    ///
    /// Panics if a child UUID in the graph doesn't have a corresponding HRID.
    #[must_use]
    pub fn suspect_links(&self) -> Vec<SuspectLink> {
        use crate::domain::requirement::ContentRef;

        let mut suspect = Vec::new();

        for child_uuid in self.graph.nodes() {
            let child_hrid = self.hrids.get(&child_uuid).unwrap();

            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Access RequirementData directly to avoid full RequirementView construction
                let Some(parent_data) = self.requirements.get(&parent_uuid) else {
                    continue;
                };

                // Calculate fingerprint directly from RequirementData
                let current_fingerprint = ContentRef {
                    content: &parent_data.content,
                    tags: &parent_data.tags,
                }
                .fingerprint();

                if edge_data.fingerprint != current_fingerprint {
                    suspect.push(SuspectLink {
                        child_uuid,
                        child_hrid: child_hrid.clone(),
                        parent_uuid,
                        parent_hrid: edge_data.parent_hrid.clone(),
                        stored_fingerprint: edge_data.fingerprint.clone(),
                        current_fingerprint,
                    });
                }
            }
        }

        suspect
    }

    /// Update the fingerprint for a specific parent link.
    ///
    /// Returns `true` if the fingerprint was updated.
    ///
    /// # Panics
    ///
    /// Panics if the child or parent requirement is not found.
    pub fn accept_suspect_link(&mut self, child_uuid: Uuid, parent_uuid: Uuid) -> bool {
        let parent = self
            .requirement(parent_uuid)
            .unwrap_or_else(|| panic!("Parent requirement {parent_uuid} not found!"));
        let current_fingerprint = parent.fingerprint();

        // Check if child and parent exist in graph
        assert!(
            self.graph.contains_node(child_uuid),
            "Child requirement {child_uuid} not found!"
        );
        assert!(
            self.graph.contains_node(parent_uuid),
            "Parent requirement {parent_uuid} not found!"
        );

        // Find and update the edge
        if let Some(edge_data) = self.graph.edge_weight_mut(child_uuid, parent_uuid) {
            if edge_data.fingerprint == current_fingerprint {
                return false; // Already up to date
            }

            // Update EdgeData (the sole source of truth)
            edge_data.fingerprint.clone_from(&current_fingerprint);

            true
        } else {
            panic!("Parent link {parent_uuid} not found in child {child_uuid}");
        }
    }

    /// Update all suspect fingerprints in the tree.
    pub fn accept_all_suspect_links(&mut self) -> Vec<(Uuid, Uuid)> {
        let suspect = self.suspect_links();
        let mut updated = Vec::new();

        for link in suspect {
            if self.accept_suspect_link(link.child_uuid, link.parent_uuid) {
                updated.push((link.child_uuid, link.parent_uuid));
            }
        }

        updated
    }
}

/// A suspect link in the requirement graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspectLink {
    /// The UUID of the child requirement.
    pub child_uuid: Uuid,
    /// The HRID of the child requirement.
    pub child_hrid: Hrid,
    /// The UUID of the parent requirement.
    pub parent_uuid: Uuid,
    /// The HRID of the parent requirement.
    pub parent_hrid: Hrid,
    /// The fingerprint stored in the child's parent reference.
    pub stored_fingerprint: String,
    /// The current fingerprint of the parent requirement.
    pub current_fingerprint: String,
}
