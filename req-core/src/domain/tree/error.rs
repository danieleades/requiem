//! Error types for tree operations.

use uuid::Uuid;

use crate::domain::Hrid;

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

/// Error type for linking requirements.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    /// The child requirement was not found in the tree.
    #[error("Child UUID {0} not found in tree")]
    ChildNotFound(Uuid),

    /// The parent requirement was not found in the tree.
    #[error("Parent UUID {0} not found in tree")]
    ParentNotFound(Uuid),
}

/// Error type for linking requirements.
#[derive(Debug, thiserror::Error)]
pub enum LinkRequirementError {
    /// The child HRID was not found in the tree.
    #[error("Child requirement {0:?} not found")]
    ChildNotFound(Hrid),

    /// The parent HRID was not found in the tree.
    #[error("Parent requirement {0:?} not found")]
    ParentNotFound(Hrid),

    /// The link would create a cycle in the requirement graph.
    #[error("{0}")]
    WouldCreateCycle(String),
}
