// Storage layer data structures for requirements.
//
// This module contains the decomposed data structures used by the Tree for storage.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};

use crate::Requirement;

/// The core data of a requirement, excluding identity and relationship information.
///
/// This struct contains only the mutable content of a requirement.
/// Identity (UUID, HRID) and relationships (parents) are stored separately
/// in the Tree structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementData {
    /// The markdown content of the requirement.
    pub content: String,
    /// Tags associated with the requirement.
    pub tags: BTreeSet<String>,
    /// When the requirement was created.
    pub created: DateTime<Utc>,
}

impl From<Requirement> for RequirementData {
    fn from(req: Requirement) -> Self {
        Self {
            content: req.content.content,
            tags: req.content.tags,
            created: req.metadata.created,
        }
    }
}
