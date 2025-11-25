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

/// Error type for tree insertion operations.
#[derive(Debug)]
pub enum TreeInsertError {
    /// Attempted to insert a requirement with a UUID that already exists in the
    /// tree.
    DuplicateUuid {
        /// The duplicate UUID
        uuid: Uuid,
    },
    /// Attempted to insert a requirement with an HRID that already exists in
    /// the tree.
    DuplicateHrid {
        /// The duplicate HRID
        hrid: Hrid,
        /// The UUID of the requirement being inserted
        new_uuid: Uuid,
        /// The UUID of the existing requirement with this HRID
        existing_uuid: Uuid,
    },
}

impl std::fmt::Display for TreeInsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateUuid { uuid } => {
                write!(f, "Duplicate requirement UUID: {uuid}")
            }
            Self::DuplicateHrid {
                hrid,
                new_uuid,
                existing_uuid,
            } => {
                write!(
                    f,
                    "Duplicate requirement HRID: {} (attempting to insert UUID {}, but HRID \
                     already maps to UUID {})",
                    hrid.display(3),
                    new_uuid,
                    existing_uuid
                )
            }
        }
    }
}

impl std::error::Error for TreeInsertError {}

/// Error type for accepting suspect links.
#[derive(Debug, thiserror::Error)]
pub enum AcceptLinkError {
    /// The child requirement was not found in the tree.
    #[error("Child requirement {0} not found")]
    ChildNotFound(Uuid),

    /// The parent requirement was not found in the tree.
    #[error("Parent requirement {0} not found (may have failed to load or been deleted)")]
    ParentNotFound(Uuid),

    /// The link between child and parent does not exist.
    #[error("No link exists between child {child} and parent {parent}")]
    LinkNotFound {
        /// The child UUID
        child: Uuid,
        /// The parent UUID
        parent: Uuid,
    },
}

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

        // Reconstruct children by finding incoming edges (edges point child→parent)
        let children: Vec<Uuid> = self
            .graph
            .edges_directed(uuid, petgraph::Direction::Incoming)
            .map(|(child_uuid, _, _)| child_uuid)
            .collect();

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
    /// Panics if the provided kind is invalid (empty or contains non-alphabetic
    /// characters).
    #[must_use]
    pub fn next_index(&self, namespace: &[KindString], kind: &KindString) -> NonZeroUsize {
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

            let children: Vec<Uuid> = self
                .graph
                .edges_directed(*uuid, petgraph::Direction::Incoming)
                .map(|(child_uuid, _, _)| child_uuid)
                .collect();

            Some(RequirementView {
                uuid,
                hrid,
                created: &data.created,
                title: &data.title,
                body: &data.body,
                tags: &data.tags,
                parents,
                children,
            })
        })
    }

    /// Finds a requirement by its human-readable identifier.
    #[must_use]
    pub fn find_by_hrid(&self, hrid: &Hrid) -> Option<RequirementView<'_>> {
        let uuid = self.hrid_to_uuid.get(hrid)?;
        self.requirement(*uuid)
    }

    /// Finds a requirement by its UUID.
    #[must_use]
    pub fn find_by_uuid(&self, uuid: Uuid) -> Option<RequirementView<'_>> {
        self.requirement(uuid)
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

    /// Unlink two requirements identified by their HRIDs.
    ///
    /// Removes the parent-child relationship between the two requirements.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - Either HRID does not exist in the tree
    /// - The link between child and parent does not exist
    pub fn unlink_requirement(&mut self, child: &Hrid, parent: &Hrid) -> anyhow::Result<Uuid> {
        let child_uuid =
            self.hrid_to_uuid.get(child).copied().ok_or_else(|| {
                anyhow::anyhow!("Child requirement {} not found", child.display(3))
            })?;

        let parent_uuid =
            self.hrid_to_uuid.get(parent).copied().ok_or_else(|| {
                anyhow::anyhow!("Parent requirement {} not found", parent.display(3))
            })?;

        // Check if the edge exists
        if !self.graph.contains_edge(child_uuid, parent_uuid) {
            anyhow::bail!(
                "No link exists between {} and {}",
                child.display(3),
                parent.display(3)
            );
        }

        // Remove the edge
        self.graph.remove_edge(child_uuid, parent_uuid);

        // Return child UUID so Directory can mark it dirty
        Ok(child_uuid)
    }

    /// Renames a requirement by changing its HRID.
    ///
    /// This updates:
    /// - The HRID mapping for the requirement
    /// - All parent edges that reference this requirement (so children's parent
    ///   HRIDs are updated)
    ///
    /// Returns the UUID of the renamed requirement and a list of children UUIDs
    /// (which need to be marked dirty since their parent HRID changed).
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - The old HRID does not exist
    /// - The new HRID already exists
    /// - The new HRID is invalid for the current configuration
    pub fn rename_requirement(
        &mut self,
        old_hrid: &Hrid,
        new_hrid: &Hrid,
    ) -> anyhow::Result<(Uuid, Vec<Uuid>)> {
        // Check old HRID exists
        let uuid = self
            .hrid_to_uuid
            .get(old_hrid)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Requirement {} not found", old_hrid.display(3)))?;

        // Check new HRID doesn't exist
        if self.hrid_to_uuid.contains_key(new_hrid) {
            anyhow::bail!(
                "Cannot rename to {}: HRID already exists",
                new_hrid.display(3)
            );
        }

        // Update HRID mappings
        self.hrids.insert(uuid, new_hrid.clone());
        self.hrid_to_uuid.remove(old_hrid);
        self.hrid_to_uuid.insert(new_hrid.clone(), uuid);

        // Find all children (incoming edges where this requirement is the parent)
        let children: Vec<Uuid> = self
            .graph
            .edges_directed(uuid, petgraph::Direction::Incoming)
            .map(|(child_uuid, _, _)| child_uuid)
            .collect();

        // Update all edges where this requirement is referenced as a parent
        // We need to update the EdgeData to have the new parent HRID
        for child_uuid in &children {
            if let Some(edge_data) = self.graph.edge_weight_mut(*child_uuid, uuid) {
                edge_data.parent_hrid = new_hrid.clone();
            }
        }

        Ok((uuid, children))
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

    /// Check which requirements have stale parent HRIDs without modifying them.
    ///
    /// Returns an iterator of child UUIDs that have at least one parent link
    /// with an outdated HRID.
    #[instrument(skip(self))]
    pub fn check_hrid_drift(&self) -> impl Iterator<Item = Uuid> + '_ {
        use std::collections::HashSet;

        let mut drifted_uuids = HashSet::new();

        for child_uuid in self.graph.nodes() {
            for (_, parent_uuid, edge_data) in self.graph.edges(child_uuid) {
                // Get the current HRID of the parent
                let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                    continue;
                };

                // Check if the stored HRID is outdated
                if &edge_data.parent_hrid != current_parent_hrid {
                    drifted_uuids.insert(child_uuid);
                }
            }
        }

        drifted_uuids.into_iter()
    }

    /// Update parent HRIDs in all requirements.
    ///
    /// When requirements are renamed or moved, the stored parent HRIDs in child
    /// requirements can become stale. This method updates all outdated parent
    /// HRIDs to match their current values.
    ///
    /// Returns an iterator of child UUIDs that were updated.
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
                // Skip edges with missing parents (e.g., failed to load)
                let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                    tracing::warn!(
                        "Skipping edge with missing parent: child={child_uuid}, \
                         parent={parent_uuid}"
                    );
                    continue;
                };

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
            // earlier). Should always succeed since we already checked above.
            let Some(current_parent_hrid) = self.hrids.get(&parent_uuid) else {
                // Parent disappeared between collection and update - skip
                tracing::warn!(
                    "Parent {parent_uuid} disappeared during HRID update for child {child_uuid}"
                );
                continue;
            };
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
                let parent_data = self.requirements.get(&parent_uuid);

                // Calculate current fingerprint, or use empty string if parent is missing
                // Empty string indicates a dangling/broken reference (parent failed to load or
                // was deleted)
                let current_fingerprint = parent_data.map_or_else(String::new, |data| {
                    ContentRef {
                        title: &data.title,
                        body: &data.body,
                        tags: &data.tags,
                    }
                    .fingerprint()
                });

                // Report as suspect if fingerprints don't match, OR if parent is missing
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
    /// Returns `Ok(true)` if the fingerprint was updated, `Ok(false)` if
    /// already up to date.
    ///
    /// # Errors
    ///
    /// Returns an error if the child or parent requirement is not found, or if
    /// the link doesn't exist.
    pub fn accept_suspect_link(
        &mut self,
        child_uuid: Uuid,
        parent_uuid: Uuid,
    ) -> Result<bool, AcceptLinkError> {
        // Check if child exists in graph
        if !self.graph.contains_node(child_uuid) {
            return Err(AcceptLinkError::ChildNotFound(child_uuid));
        }

        // Check if parent exists and get its fingerprint
        let parent = self
            .requirement(parent_uuid)
            .ok_or(AcceptLinkError::ParentNotFound(parent_uuid))?;
        let current_fingerprint = parent.fingerprint();

        // Check if parent exists in graph
        if !self.graph.contains_node(parent_uuid) {
            return Err(AcceptLinkError::ParentNotFound(parent_uuid));
        }

        // Find and update the edge
        if let Some(edge_data) = self.graph.edge_weight_mut(child_uuid, parent_uuid) {
            if edge_data.fingerprint == current_fingerprint {
                return Ok(false); // Already up to date
            }

            // Update EdgeData (the sole source of truth)
            edge_data.fingerprint.clone_from(&current_fingerprint);

            Ok(true)
        } else {
            Err(AcceptLinkError::LinkNotFound {
                child: child_uuid,
                parent: parent_uuid,
            })
        }
    }

    /// Update all suspect fingerprints in the tree.
    ///
    /// Skips links where the parent is missing (logs a warning instead of
    /// failing).
    pub fn accept_all_suspect_links(&mut self) -> Vec<(Uuid, Uuid)> {
        let suspect = self.suspect_links();
        let mut updated = Vec::new();

        for link in suspect {
            match self.accept_suspect_link(link.child_uuid, link.parent_uuid) {
                Ok(true) => {
                    updated.push((link.child_uuid, link.parent_uuid));
                }
                Ok(false) => {
                    // Already up to date, skip
                }
                Err(AcceptLinkError::ParentNotFound(parent_uuid)) => {
                    // Parent missing - log but don't fail the entire operation
                    tracing::warn!(
                        "Cannot accept suspect link: parent {parent_uuid} not found (child: {})",
                        link.child_uuid
                    );
                }
                Err(e) => {
                    // Other errors are unexpected but also shouldn't stop the batch
                    tracing::error!("Failed to accept suspect link: {e}");
                }
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
    ///
    /// If empty, indicates the parent requirement is missing (failed to load or
    /// was deleted).
    pub current_fingerprint: String,
}
