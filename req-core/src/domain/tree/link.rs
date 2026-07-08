//! Creating, removing, and renaming parent-child links between requirements.

use uuid::Uuid;

use super::{EdgeData, LinkError, LinkRequirementError, Tree};
use crate::domain::Hrid;

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

impl Tree {
    /// Link two requirements identified by their HRIDs.
    ///
    /// # Errors
    ///
    /// Returns an error when either HRID does not exist in the tree, when
    /// the parent/child UUIDs cannot be linked, or when the link would create a
    /// cycle.
    pub fn link_requirement(
        &mut self,
        child: &Hrid,
        parent: &Hrid,
    ) -> Result<LinkOutcome, LinkRequirementError> {
        let (child_uuid, child_hrid) = {
            let view = self
                .find_by_hrid(child)
                .ok_or_else(|| LinkRequirementError::ChildNotFound(child.clone()))?;
            (*view.uuid, view.hrid.clone())
        };

        let (parent_uuid, parent_hrid, parent_fingerprint) = {
            let view = self
                .find_by_hrid(parent)
                .ok_or_else(|| LinkRequirementError::ParentNotFound(parent.clone()))?;
            (*view.uuid, view.hrid.clone(), view.fingerprint())
        };

        // Check if this link would create a cycle
        self.check_would_create_cycle(child_uuid, parent_uuid)
            .map_err(|e| LinkRequirementError::WouldCreateCycle(e.to_string()))?;

        let already_linked = self.graph.contains_edge(child_uuid, parent_uuid);

        self.upsert_parent_link(child_uuid, parent_uuid, parent_fingerprint)
            .map_err(|error| match error {
                LinkError::ChildNotFound(_) => {
                    LinkRequirementError::ChildNotFound(child_hrid.clone())
                }
                LinkError::ParentNotFound(_) => {
                    LinkRequirementError::ParentNotFound(parent_hrid.clone())
                }
            })?;

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

    /// Insert or update a parent link for the given child UUID.
    ///
    /// Returns `Ok(true)` if an existing link was replaced, or `Ok(false)` if a
    /// new link was created.
    ///
    /// # Errors
    ///
    /// Returns an error if either the child or parent UUID does not exist in
    /// the tree.
    pub fn upsert_parent_link(
        &mut self,
        child_uuid: Uuid,
        parent_uuid: Uuid,
        fingerprint: String,
    ) -> Result<bool, LinkError> {
        // Validate both UUIDs exist in requirements first
        if !self.requirements.contains_key(&child_uuid) {
            return Err(LinkError::ChildNotFound(child_uuid));
        }
        if !self.requirements.contains_key(&parent_uuid) {
            return Err(LinkError::ParentNotFound(parent_uuid));
        }

        // Ensure both nodes exist in the graph; GraphMap::add_node is idempotent.
        self.graph.add_node(child_uuid);
        self.graph.add_node(parent_uuid);

        let parent_hrid = self
            .hrids
            .get(&parent_uuid)
            .ok_or(LinkError::ParentNotFound(parent_uuid))?;

        let edge = EdgeData {
            parent_hrid: parent_hrid.clone(),
            fingerprint,
        };

        Ok(self.graph.add_edge(child_uuid, parent_uuid, edge).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Requirement;

    #[test]
    fn test_upsert_parent_link_errors_on_missing_parent() {
        // Test that upsert_parent_link returns an error (not panic) when
        // trying to link to a parent that doesn't exist
        let mut tree = Tree::default();

        // Create and insert a child requirement
        let child_req = Requirement::new(
            "USR-001".parse().unwrap(),
            "Child requirement".to_string(),
            String::new(),
        );
        let child_uuid = child_req.uuid();
        tree.insert(child_req).unwrap();

        // Try to link to a parent UUID that doesn't exist in the tree
        let missing_parent_uuid = uuid::Uuid::new_v4();

        // This should return an error, not panic
        let result =
            tree.upsert_parent_link(child_uuid, missing_parent_uuid, "fingerprint".to_string());

        // Verify we get a ParentNotFound error
        assert!(result.is_err(), "Expected error for missing parent");
        match result {
            Err(LinkError::ParentNotFound(uuid)) => {
                assert_eq!(
                    uuid, missing_parent_uuid,
                    "Error should contain the missing parent UUID"
                );
            }
            _ => panic!("Expected ParentNotFound error, got: {result:?}"),
        }
    }

    #[test]
    fn test_upsert_parent_link_errors_on_missing_child() {
        // Test that upsert_parent_link returns an error when child doesn't exist
        let mut tree = Tree::default();

        // Create and insert a parent requirement
        let parent_req = Requirement::new(
            "SYS-001".parse().unwrap(),
            "Parent requirement".to_string(),
            String::new(),
        );
        let parent_uuid = parent_req.uuid();
        tree.insert(parent_req).unwrap();

        // Try to link a child UUID that doesn't exist
        let missing_child_uuid = uuid::Uuid::new_v4();

        // This should return an error, not panic
        let result =
            tree.upsert_parent_link(missing_child_uuid, parent_uuid, "fingerprint".to_string());

        // Verify we get a ChildNotFound error
        assert!(result.is_err(), "Expected error for missing child");
        match result {
            Err(LinkError::ChildNotFound(uuid)) => {
                assert_eq!(
                    uuid, missing_child_uuid,
                    "Error should contain the missing child UUID"
                );
            }
            _ => panic!("Expected ChildNotFound error, got: {result:?}"),
        }
    }

    #[test]
    fn test_upsert_parent_link_succeeds_when_both_exist() {
        // Test the happy path - both parent and child exist
        let mut tree = Tree::default();

        // Create and insert parent and child requirements
        let parent_req = Requirement::new(
            "SYS-001".parse().unwrap(),
            "Parent requirement".to_string(),
            String::new(),
        );
        let parent_uuid = parent_req.uuid();
        tree.insert(parent_req).unwrap();

        let child_req = Requirement::new(
            "USR-001".parse().unwrap(),
            "Child requirement".to_string(),
            String::new(),
        );
        let child_uuid = child_req.uuid();
        tree.insert(child_req).unwrap();

        // This should succeed
        let result =
            tree.upsert_parent_link(child_uuid, parent_uuid, "test-fingerprint".to_string());

        assert!(
            result.is_ok(),
            "Expected success when both requirements exist"
        );
        assert!(
            !result.unwrap(),
            "Should return false for new link (not a replacement)"
        );

        // Verify the link was created
        let parents = tree.parents(child_uuid);
        assert_eq!(parents.len(), 1, "Child should have one parent");
        assert_eq!(parents[0].0, parent_uuid, "Parent UUID should match");
    }
}
