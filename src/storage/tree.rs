//! An in-memory tree structure for requirements
//!
//! The [`Tree`] knows nothing about the filesystem or the directory structure.
//! It is a simple in-memory representation of the requirements and their
//! relationships.

use std::{cmp::Ordering, collections::HashMap};

use tracing::instrument;
use uuid::Uuid;

use crate::{domain::Hrid, Requirement};

/// An in-memory representation of the set of requirements
#[derive(Debug, Default, PartialEq)]
pub struct Tree {
    /// The requirements, stored contiguously.
    requirements: Vec<Requirement>,

    /// An index from UUID to position in `requirements`.
    index: HashMap<Uuid, usize>,

    /// A map from requirement kind to the next available index for that kind.
    next_indices: HashMap<String, usize>,
}

impl Tree {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            requirements: Vec::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
            next_indices: HashMap::new(),
        }
    }

    /// Inserts a requirement into the tree.
    ///
    /// # Panics
    ///
    /// Panics if a requirement with the same UUID already exists.
    pub fn insert(&mut self, requirement: Requirement) {
        let uuid = requirement.uuid();
        assert!(
            !self.index.contains_key(&uuid),
            "Duplicate requirement UUID: {uuid}"
        );
        let index = self.requirements.len();

        // Update the current index for the requirement's kind to the larger of its
        // current value or the index of the incoming requirement.
        let hrid = requirement.hrid();
        let kind = hrid.kind();
        let suffix = hrid.id();

        self.next_indices
            .entry(kind.to_string())
            .and_modify(|i| *i = (*i).max(suffix + 1))
            .or_insert(suffix + 1);

        self.requirements.push(requirement);
        self.index.insert(uuid, index);
    }

    /// Retrieves a requirement by UUID.
    #[must_use]
    pub fn requirement(&self, uuid: Uuid) -> Option<&Requirement> {
        self.index
            .get(&uuid)
            .and_then(|&idx| self.requirements.get(idx))
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
        (0..self.requirements.len()).filter_map(|i| {
            let (left, right) = self.requirements.split_at_mut(i);
            let (req, right) = right.split_first_mut()?;
            let uuid = req.uuid();

            let updated: Vec<bool> = req
                .parents_mut()
                .map(|(parent_id, parent)| {
                    let &parent_idx = self
                        .index
                        .get(&parent_id)
                        .unwrap_or_else(|| panic!("Parent requirement {parent_id} not found!"));

                    let actual_hrid = match parent_idx.cmp(&i) {
                        Ordering::Less => left[parent_idx].hrid(),
                        Ordering::Greater => right[parent_idx - i - 1].hrid(),
                        Ordering::Equal => {
                            unreachable!("Requirement {parent_id} is its own parent!")
                        }
                    };

                    if parent.hrid == *actual_hrid {
                        false
                    } else {
                        parent.hrid = actual_hrid.clone();
                        true
                    }
                })
                // Collect here to ensure all parents are updated (no short-circuiting).
                .collect();

            // If any parent was updated, return the UUID of the requirement.
            updated
                .iter()
                .any(|was_updated| *was_updated)
                .then_some(uuid)
        })
    }

    /// Returns the next available index for a requirement of the given kind.
    ///
    /// This is one greater than the highest index currently used for that kind.
    /// No attempt is made to 'recycle' indices if there are gaps in the
    /// sequence.
    #[must_use]
    pub fn next_index(&self, kind: &str) -> usize {
        self.next_indices.get(kind).copied().unwrap_or(1)
    }

    /// Find all suspect links in the requirement graph.
    ///
    /// A link is suspect when the fingerprint stored in a child requirement
    /// does not match the current fingerprint of the parent requirement.
    /// This indicates the parent has been modified and the child should be
    /// reviewed.
    ///
    /// Returns a vector of (`child_uuid`, `child_hrid`, `parent_uuid`,
    /// `parent_hrid`, `stored_fingerprint`, `current_fingerprint`)
    #[must_use]
    pub fn suspect_links(&self) -> Vec<SuspectLink> {
        let mut suspect = Vec::new();

        for req in &self.requirements {
            let child_uuid = req.uuid();
            let child_hrid = req.hrid().clone();

            for (parent_uuid, parent_ref) in req.parents() {
                let Some(parent) = self.requirement(parent_uuid) else {
                    // Parent not found - this is a different error, skip for now
                    continue;
                };

                let current_fingerprint = parent.fingerprint();
                if parent_ref.fingerprint != current_fingerprint {
                    suspect.push(SuspectLink {
                        child_uuid,
                        child_hrid: child_hrid.clone(),
                        parent_uuid,
                        parent_hrid: parent_ref.hrid.clone(),
                        stored_fingerprint: parent_ref.fingerprint.clone(),
                        current_fingerprint,
                    });
                }
            }
        }

        suspect
    }

    /// Update the fingerprint for a specific parent link in a child
    /// requirement.
    ///
    /// This signals that the child has been reviewed and is still valid despite
    /// the parent's changes.
    ///
    /// Returns `true` if the fingerprint was updated, `false` if the link was
    /// not suspect (fingerprints already matched).
    ///
    /// # Panics
    ///
    /// Panics if the child or parent requirement is not found in the tree.
    pub fn accept_suspect_link(&mut self, child_uuid: Uuid, parent_uuid: Uuid) -> bool {
        let parent_idx = *self
            .index
            .get(&parent_uuid)
            .unwrap_or_else(|| panic!("Parent requirement {parent_uuid} not found!"));

        let current_fingerprint = self.requirements[parent_idx].fingerprint();

        let child_idx = *self
            .index
            .get(&child_uuid)
            .unwrap_or_else(|| panic!("Child requirement {child_uuid} not found!"));

        let child = &mut self.requirements[child_idx];

        // Find the parent reference and update its fingerprint
        for (pid, parent_ref) in child.parents_mut() {
            if pid == parent_uuid {
                if parent_ref.fingerprint == current_fingerprint {
                    // Already up to date
                    return false;
                }
                parent_ref.fingerprint = current_fingerprint;
                return true;
            }
        }

        panic!("Parent link {parent_uuid} not found in child {child_uuid}");
    }

    /// Update all suspect fingerprints in the tree.
    ///
    /// Returns a vector of (`child_uuid`, `parent_uuid`) pairs that were
    /// updated.
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

/// Information about a suspect link
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspectLink {
    pub child_uuid: Uuid,
    pub child_hrid: Hrid,
    pub parent_uuid: Uuid,
    pub parent_hrid: Hrid,
    pub stored_fingerprint: String,
    pub current_fingerprint: String,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{domain::Hrid, storage::Tree, Requirement};

    fn make_requirement(uuid: Uuid, hrid: Hrid, parents: Vec<(Uuid, Hrid)>) -> Requirement {
        let mut req = Requirement::new_with_uuid(hrid, String::new(), uuid);
        for (parent_uuid, parent_hrid) in parents {
            req.add_parent(
                parent_uuid,
                crate::domain::requirement::Parent {
                    hrid: parent_hrid,
                    fingerprint: String::new(),
                },
            );
        }
        req
    }

    #[test]
    fn insert_and_lookup() {
        let mut tree = Tree::default();
        let uuid = Uuid::new_v4();
        let hrid = Hrid::try_from("R-001").unwrap();
        let req = make_requirement(uuid, hrid.clone(), vec![]);
        tree.insert(req);

        let retrieved = tree.requirement(uuid).unwrap();
        assert_eq!(retrieved.uuid(), uuid);
        assert_eq!(retrieved.hrid(), &hrid);
    }

    #[test]
    #[should_panic(expected = "Duplicate requirement UUID")]
    fn test_insert_duplicate_uuid_panics() {
        let mut tree = Tree::default();
        let uuid = Uuid::new_v4();
        let req1 = make_requirement(uuid, Hrid::try_from("R-001").unwrap(), vec![]);
        let req2 = make_requirement(uuid, Hrid::try_from("R-002").unwrap(), vec![]);
        tree.insert(req1);
        tree.insert(req2); // should panic
    }

    #[test]
    fn update_hrids_corrects_parent_hrids() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();

        let parent = make_requirement(parent_uuid, Hrid::try_from("P-001").unwrap(), vec![]);
        let child = make_requirement(
            child_uuid,
            Hrid::try_from("C-001").unwrap(),
            vec![(parent_uuid, Hrid::try_from("WRONG-001").unwrap())],
        );

        tree.insert(parent);
        tree.insert(child);

        let updated: Vec<_> = tree.update_hrids().collect();
        assert_eq!(updated, vec![child_uuid]);

        let updated_child = tree.requirement(child_uuid).unwrap();
        let (_, actual_parent) = updated_child.parents().next().unwrap();
        assert_eq!(actual_parent.hrid, Hrid::try_from("P-001").unwrap());
    }

    #[test]
    fn update_hrids_no_change() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();

        let parent = make_requirement(parent_uuid, Hrid::try_from("P-001").unwrap(), vec![]);
        let child = make_requirement(
            child_uuid,
            Hrid::try_from("C-001").unwrap(),
            vec![(parent_uuid, Hrid::try_from("P-001").unwrap())],
        );

        tree.insert(parent);
        tree.insert(child);

        let updated = tree.update_hrids();
        assert!(updated.count() == 0);
    }

    #[test]
    #[should_panic(expected = "Parent requirement")]
    fn test_update_hrids_missing_parent_panics() {
        let mut tree = Tree::default();
        let missing_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();
        let child = make_requirement(
            child_uuid,
            Hrid::try_from("C-001").unwrap(),
            vec![(missing_uuid, Hrid::try_from("UNKNOWN-001").unwrap())],
        );

        tree.insert(child);
        let _ = tree.update_hrids().collect::<Vec<_>>();
    }

    #[test]
    #[should_panic(expected = "is its own parent")]
    fn test_update_hrids_self_parent_panics() {
        let mut tree = Tree::default();
        let uuid = Uuid::new_v4();
        let req = make_requirement(
            uuid,
            Hrid::try_from("SELF-001").unwrap(),
            vec![(uuid, Hrid::try_from("SELF-001").unwrap())],
        );

        tree.insert(req);
        let _ = tree.update_hrids().collect::<Vec<_>>();
    }

    #[test]
    fn test_suspect_links_detects_mismatched_fingerprints() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();

        // Create parent with some content
        let parent = Requirement::new_with_uuid(
            Hrid::try_from("P-001").unwrap(),
            "original content".to_string(),
            parent_uuid,
        );
        let original_fingerprint = parent.fingerprint();

        // Create child linking to parent with correct fingerprint
        let mut child =
            Requirement::new_with_uuid(Hrid::try_from("C-001").unwrap(), String::new(), child_uuid);
        child.add_parent(
            parent_uuid,
            crate::domain::requirement::Parent {
                hrid: Hrid::try_from("P-001").unwrap(),
                fingerprint: original_fingerprint.clone(),
            },
        );

        tree.insert(parent);
        tree.insert(child);

        // No suspect links initially
        assert!(tree.suspect_links().is_empty());

        // Modify parent content (creating new parent with same UUID)
        let modified_parent = Requirement::new_with_uuid(
            Hrid::try_from("P-001").unwrap(),
            "modified content".to_string(),
            parent_uuid,
        );

        // Replace parent in tree
        let parent_idx = *tree.index.get(&parent_uuid).unwrap();
        tree.requirements[parent_idx] = modified_parent;

        // Now should detect suspect link
        let suspect = tree.suspect_links();
        assert_eq!(suspect.len(), 1);
        assert_eq!(suspect[0].child_uuid, child_uuid);
        assert_eq!(suspect[0].parent_uuid, parent_uuid);
        assert_eq!(suspect[0].stored_fingerprint, original_fingerprint);
        assert_ne!(
            suspect[0].stored_fingerprint,
            suspect[0].current_fingerprint
        );
    }

    #[test]
    fn test_accept_suspect_link_updates_fingerprint() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();

        let parent = Requirement::new_with_uuid(
            Hrid::try_from("P-001").unwrap(),
            "content".to_string(),
            parent_uuid,
        );

        let mut child =
            Requirement::new_with_uuid(Hrid::try_from("C-001").unwrap(), String::new(), child_uuid);
        child.add_parent(
            parent_uuid,
            crate::domain::requirement::Parent {
                hrid: Hrid::try_from("P-001").unwrap(),
                fingerprint: "old_fingerprint".to_string(),
            },
        );

        tree.insert(parent);
        tree.insert(child);

        // Should have suspect link
        assert_eq!(tree.suspect_links().len(), 1);

        // Accept the suspect link
        let was_updated = tree.accept_suspect_link(child_uuid, parent_uuid);
        assert!(was_updated);

        // No more suspect links
        assert!(tree.suspect_links().is_empty());

        // Accepting again should return false (already up to date)
        let was_updated = tree.accept_suspect_link(child_uuid, parent_uuid);
        assert!(!was_updated);
    }

    #[test]
    fn test_accept_all_suspect_links() {
        let mut tree = Tree::default();
        let parent1_uuid = Uuid::new_v4();
        let parent2_uuid = Uuid::new_v4();
        let child1_uuid = Uuid::new_v4();
        let child2_uuid = Uuid::new_v4();

        let parent1 = Requirement::new_with_uuid(
            Hrid::try_from("P-001").unwrap(),
            "content1".to_string(),
            parent1_uuid,
        );
        let parent2 = Requirement::new_with_uuid(
            Hrid::try_from("P-002").unwrap(),
            "content2".to_string(),
            parent2_uuid,
        );

        let mut child1 = Requirement::new_with_uuid(
            Hrid::try_from("C-001").unwrap(),
            String::new(),
            child1_uuid,
        );
        child1.add_parent(
            parent1_uuid,
            crate::domain::requirement::Parent {
                hrid: Hrid::try_from("P-001").unwrap(),
                fingerprint: "old1".to_string(),
            },
        );

        let mut child2 = Requirement::new_with_uuid(
            Hrid::try_from("C-002").unwrap(),
            String::new(),
            child2_uuid,
        );
        child2.add_parent(
            parent2_uuid,
            crate::domain::requirement::Parent {
                hrid: Hrid::try_from("P-002").unwrap(),
                fingerprint: "old2".to_string(),
            },
        );

        tree.insert(parent1);
        tree.insert(parent2);
        tree.insert(child1);
        tree.insert(child2);

        // Should have 2 suspect links
        assert_eq!(tree.suspect_links().len(), 2);

        // Accept all
        let updated = tree.accept_all_suspect_links();
        assert_eq!(updated.len(), 2);

        // No more suspect links
        assert!(tree.suspect_links().is_empty());
    }

    #[test]
    #[should_panic(expected = "Parent requirement")]
    fn test_accept_suspect_link_missing_parent_panics() {
        let mut tree = Tree::default();
        let child_uuid = Uuid::new_v4();
        let missing_parent_uuid = Uuid::new_v4();

        let child = make_requirement(child_uuid, Hrid::try_from("C-001").unwrap(), vec![]);
        tree.insert(child);

        tree.accept_suspect_link(child_uuid, missing_parent_uuid);
    }

    #[test]
    #[should_panic(expected = "Child requirement")]
    fn test_accept_suspect_link_missing_child_panics() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let missing_child_uuid = Uuid::new_v4();

        let parent = make_requirement(parent_uuid, Hrid::try_from("P-001").unwrap(), vec![]);
        tree.insert(parent);

        tree.accept_suspect_link(missing_child_uuid, parent_uuid);
    }

    #[test]
    #[should_panic(expected = "Parent link")]
    fn test_accept_suspect_link_missing_link_panics() {
        let mut tree = Tree::default();
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();

        let parent = make_requirement(parent_uuid, Hrid::try_from("P-001").unwrap(), vec![]);
        let child = make_requirement(child_uuid, Hrid::try_from("C-001").unwrap(), vec![]);

        tree.insert(parent);
        tree.insert(child);

        // Child has no parent link, should panic
        tree.accept_suspect_link(child_uuid, parent_uuid);
    }
}
