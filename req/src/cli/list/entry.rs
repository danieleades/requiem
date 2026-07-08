//! Requirement snapshots used by the list command.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use requiem_core::{Directory, Hrid, RequirementView};
use uuid::Uuid;

/// Parsed requirement snapshot used for listing.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) uuid: Uuid,
    pub(super) hrid: Hrid,
    pub(super) title: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) created: DateTime<Utc>,
    pub(super) content: String,
    pub(super) parents: Vec<LinkRef>,
    pub(super) children: Vec<LinkRef>,
    pub(super) path: PathBuf,
}

/// A reference to a related requirement (parent or child).
#[derive(Debug, Clone)]
pub(super) struct LinkRef {
    pub(super) uuid: Uuid,
    pub(super) hrid: Hrid,
}

impl LinkRef {
    pub(super) const fn new(uuid: Uuid, hrid: Hrid) -> Self {
        Self { uuid, hrid }
    }
}

pub(super) fn collect_entries(directory: &Directory) -> Vec<Entry> {
    let mut entries = Vec::new();
    let digits = directory.config().digits();

    for requirement in directory.requirements() {
        entries.push(entry_from_requirement(directory, &requirement, digits));
    }

    entries
}

fn entry_from_requirement(
    directory: &Directory,
    requirement: &RequirementView,
    digits: usize,
) -> Entry {
    let parents = requirement
        .parents
        .iter()
        .map(|(uuid, parent)| LinkRef::new(*uuid, parent.hrid.clone()))
        .collect::<Vec<_>>();

    let tags = requirement.tags.iter().cloned().collect::<Vec<_>>();
    let path = directory.path_for(requirement.hrid).map_or_else(
        || directory.canonical_path_for(requirement.hrid),
        std::path::Path::to_path_buf,
    );

    Entry {
        uuid: *requirement.uuid,
        hrid: requirement.hrid.clone(),
        title: Some(requirement.title.to_string()),
        tags,
        created: *requirement.created,
        content: format!(
            "# {} {}\n\n{}",
            requirement.hrid.display(digits),
            requirement.title,
            requirement.body
        ),
        parents,
        children: Vec::new(),
        path,
    }
}
