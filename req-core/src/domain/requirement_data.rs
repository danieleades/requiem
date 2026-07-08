//! Decomposed requirement data: the content-and-timestamp payload stored
//! per UUID by the [`Tree`](crate::domain::Tree).

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};

use crate::Requirement;

/// The core data of a requirement, excluding identity and relationship
/// information.
///
/// This struct contains only the mutable content of a requirement.
/// Identity (UUID, HRID) and relationships (parents) are stored separately
/// in the Tree structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementData {
    /// The title of the requirement (without HRID or markdown heading markers).
    pub title: String,
    /// The body content of the requirement (markdown text after the heading).
    pub body: String,
    /// Tags associated with the requirement.
    pub tags: BTreeSet<String>,
    /// When the requirement was created.
    pub created: DateTime<Utc>,
}

impl From<Requirement> for RequirementData {
    fn from(req: Requirement) -> Self {
        Self {
            title: req.content.title,
            body: req.content.body,
            tags: req.content.tags,
            created: req.metadata.created,
        }
    }
}
