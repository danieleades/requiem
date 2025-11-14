// RequirementView - a borrowed view of a requirement for serialization.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        requirement::{ContentRef, Parent},
        Hrid,
    },
    Requirement,
};

/// A borrowed view of a requirement, suitable for serialization.
///
/// This struct holds references to requirement data from the Tree's decomposed
/// storage. It's used primarily for serialization to avoid cloning data.
#[derive(Debug, Clone)]
pub struct RequirementView<'a> {
    /// The requirement's UUID.
    pub uuid: &'a Uuid,
    /// The requirement's HRID.
    pub hrid: &'a Hrid,
    /// When the requirement was created.
    pub created: &'a DateTime<Utc>,
    /// The requirement's title.
    pub title: &'a str,
    /// The requirement's body content.
    pub body: &'a str,
    /// The requirement's tags.
    pub tags: &'a BTreeSet<String>,
    /// Parent requirements (UUID → Parent info).
    ///
    /// Note: This is constructed on-demand from the graph, so it owns the data.
    pub parents: Vec<(Uuid, Parent)>,
}

impl RequirementView<'_> {
    /// Calculate the fingerprint of this requirement.
    ///
    /// The fingerprint is based on the body and tags only (not metadata like
    /// UUID, HRID, title, or parents).
    #[must_use]
    pub fn fingerprint(&self) -> String {
        ContentRef {
            title: self.title,
            body: self.body,
            tags: self.tags,
        }
        .fingerprint()
    }

    /// Convert this view to an owned `Requirement`.
    ///
    /// This clones all the data to create a fully owned requirement.
    #[must_use]
    pub fn to_requirement(&self) -> Requirement {
        use std::collections::HashMap;

        Requirement {
            content: crate::domain::requirement::Content {
                title: self.title.to_string(),
                body: self.body.to_string(),
                tags: self.tags.clone(),
            },
            metadata: crate::domain::requirement::Metadata {
                uuid: *self.uuid,
                hrid: self.hrid.clone(),
                created: *self.created,
                parents: self
                    .parents
                    .iter()
                    .map(|(uuid, parent)| (*uuid, parent.clone()))
                    .collect::<HashMap<_, _>>(),
            },
        }
    }
}
