use std::{
    collections::{BTreeSet, HashMap},
    io,
    path::Path,
};

use borsh::BorshSerialize;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[doc(hidden)]
pub use crate::storage::markdown::LoadError;
use crate::{domain::Hrid, storage::markdown::MarkdownRequirement};

/// A requirement is a document used to describe a system.
///
/// It can represent a user requirement, a specification, etc.
/// Requirements can have dependencies between them, such that one requirement
/// satisfies, fulfils, verifies (etc.) another requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement's content (markdown text and tags).
    pub(crate) content: Content,
    /// The requirement's metadata (UUID, HRID, creation time, parents).
    pub(crate) metadata: Metadata,
}

/// The semantically important content of the requirement.
///
/// This contributes to the 'fingerprint' of the requirement
#[derive(Debug, BorshSerialize, Clone, PartialEq, Eq)]
pub(crate) struct Content {
    /// Markdown content of the requirement.
    pub(crate) content: String,
    /// Set of tags associated with the requirement.
    pub(crate) tags: BTreeSet<String>,
}

impl Content {
    /// Creates a borrowed reference to this content.
    ///
    /// This is useful for computing fingerprints without cloning data.
    #[must_use]
    fn as_ref(&self) -> ContentRef<'_> {
        ContentRef {
            content: &self.content,
            tags: &self.tags,
        }
    }

    fn fingerprint(&self) -> String {
        self.as_ref().fingerprint()
    }
}

/// A borrowed reference to requirement content.
///
/// This type represents the semantically important content of a requirement
/// using borrowed data. It is used for computing fingerprints without cloning.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContentRef<'a> {
    /// The markdown content of the requirement.
    pub(crate) content: &'a str,
    /// Tags associated with the requirement.
    pub(crate) tags: &'a BTreeSet<String>,
}

impl ContentRef<'_> {
    /// Calculate the fingerprint of this content.
    ///
    /// The fingerprint is a SHA256 hash of the Borsh-serialized content and
    /// tags. This is used to detect when requirement content has changed.
    ///
    /// # Panics
    ///
    /// Panics if borsh serialization fails (which should never happen for this
    /// data structure).
    #[must_use]
    pub fn fingerprint(&self) -> String {
        #[derive(BorshSerialize)]
        struct FingerprintData<'a> {
            content: &'a str,
            tags: &'a BTreeSet<String>,
        }

        let data = FingerprintData {
            content: self.content,
            tags: self.tags,
        };

        // encode using [borsh](https://borsh.io/)
        let encoded = borsh::to_vec(&data).expect("this should never fail");

        // generate a SHA256 hash
        let hash = Sha256::digest(encoded);

        // Convert to hex string
        format!("{hash:x}")
    }
}

/// Requirement metadata.
///
/// Does not contribute to the requirement fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Metadata {
    /// Globally unique, perpetually stable identifier
    pub(crate) uuid: Uuid,

    /// Globally unique, human readable identifier.
    ///
    /// This should in general change, however it is possible to
    /// change it if needed.
    pub(crate) hrid: Hrid,
    /// Timestamp recording when the requirement was created.
    pub(crate) created: DateTime<Utc>,
    /// Parent requirements keyed by UUID.
    pub(crate) parents: HashMap<Uuid, Parent>,
}

/// Parent requirement metadata stored alongside a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parent {
    /// Human-readable identifier of the parent requirement.
    pub hrid: Hrid,
    /// Fingerprint snapshot of the parent requirement.
    pub fingerprint: String,
}

impl Requirement {
    /// Construct a new [`Requirement`] from a human-readable ID and its
    /// content.
    ///
    /// A new UUID is automatically generated.
    #[must_use]
    pub(crate) fn new(hrid: Hrid, content: String) -> Self {
        Self::new_with_uuid(hrid, content, Uuid::new_v4())
    }

    pub(crate) fn new_with_uuid(hrid: Hrid, content: String, uuid: Uuid) -> Self {
        let content = Content {
            content,
            tags: BTreeSet::default(),
        };

        let metadata = Metadata {
            uuid,
            hrid,
            created: Utc::now(),
            parents: HashMap::new(),
        };

        Self { content, metadata }
    }

    /// The body of the requirement.
    ///
    /// This should be a markdown document.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content.content
    }

    /// The tags on the requirement
    #[must_use]
    pub const fn tags(&self) -> &BTreeSet<String> {
        &self.content.tags
    }

    /// Set the tags on the requirement.
    ///
    /// this replaces any existing tags.
    pub fn set_tags(&mut self, tags: BTreeSet<String>) {
        self.content.tags = tags;
    }

    /// Add a tag to the requirement.
    ///
    /// returns 'true' if a new tag was inserted, or 'false' if it was already
    /// present.
    pub fn add_tag(&mut self, tag: String) -> bool {
        self.content.tags.insert(tag)
    }

    /// The human-readable identifier for this requirement.
    ///
    /// In normal usage these should be stable
    #[must_use]
    pub const fn hrid(&self) -> &Hrid {
        &self.metadata.hrid
    }

    /// The unique, stable identifier of this requirement
    #[must_use]
    pub const fn uuid(&self) -> Uuid {
        self.metadata.uuid
    }

    /// When the requirement was first created
    #[must_use]
    pub const fn created(&self) -> DateTime<Utc> {
        self.metadata.created
    }

    /// Returns a value generated by hashing the content of the Requirement.
    ///
    /// Any change to the requirement will change the fingerprint. This is used
    /// to determine when links are 'suspect'. Meaning that because a
    /// requirement has been modified, related or dependent requirements
    /// also need to be reviewed to ensure consistency.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.content.fingerprint()
    }

    /// Add a parent to the requirement, keyed by UUID.
    pub fn add_parent(&mut self, parent_id: Uuid, parent_info: Parent) -> Option<Parent> {
        self.metadata.parents.insert(parent_id, parent_info)
    }

    /// Return an iterator over the requirement's 'parents'
    pub fn parents(&self) -> impl Iterator<Item = (Uuid, &Parent)> {
        self.metadata
            .parents
            .iter()
            .map(|(&id, parent)| (id, parent))
    }

    /// Return a mutable iterator over the requirement's 'parents'
    pub fn parents_mut(&mut self) -> impl Iterator<Item = (Uuid, &mut Parent)> {
        self.metadata
            .parents
            .iter_mut()
            .map(|(&id, parent)| (id, parent))
    }

    /// Reads a requirement using the given configuration.
    ///
    /// The path construction respects the `subfolders_are_namespaces` setting:
    /// - If `false`: loads from `root/FULL-HRID.md`
    /// - If `true`: loads from `root/namespace/folders/KIND-ID.md`
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, cannot be read from, or has
    /// malformed YAML frontmatter.
    #[doc(hidden)]
    pub fn load(
        root: &Path,
        hrid: &Hrid,
        config: &crate::domain::Config,
    ) -> Result<Self, LoadError> {
        Ok(MarkdownRequirement::load(root, hrid, config)?.try_into()?)
    }

    /// Writes the requirement using the given configuration.
    ///
    /// The path construction respects the `subfolders_are_namespaces` setting:
    /// - If `false`: file is saved as `root/FULL-HRID.md`
    /// - If `true`: file is saved as `root/namespace/folders/KIND-ID.md`
    ///
    /// Parent directories are created automatically if they don't exist.
    ///
    /// # Errors
    ///
    /// This method returns an error if the path cannot be written to.
    #[doc(hidden)]
    pub fn save(&self, root: &Path, config: &crate::domain::Config) -> io::Result<()> {
        MarkdownRequirement::from(self.clone()).save(root, config)
    }

    /// Save this requirement to a specific file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    #[doc(hidden)]
    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        MarkdownRequirement::from(self.clone()).save_to_path(path)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::Content;

    #[test]
    fn fingerprint_does_not_panic() {
        let content = Content {
            content: "Some string".to_string(),
            tags: ["tag1".to_string(), "tag2".to_string()].into(),
        };
        content.fingerprint();
    }

    #[test]
    fn fingerprint_is_stable_with_tag_order() {
        let content1 = Content {
            content: "Some string".to_string(),
            tags: ["tag1".to_string(), "tag2".to_string()].into(),
        };
        let content2 = Content {
            content: "Some string".to_string(),
            tags: ["tag2".to_string(), "tag1".to_string()].into(),
        };
        assert_eq!(content1.fingerprint(), content2.fingerprint());
    }

    #[test]
    fn tags_affect_fingerprint() {
        let content1 = Content {
            content: "Some string".to_string(),
            tags: ["tag1".to_string()].into(),
        };
        let content2 = Content {
            content: "Some string".to_string(),
            tags: ["tag1".to_string(), "tag2".to_string()].into(),
        };
        assert_ne!(content1.fingerprint(), content2.fingerprint());
    }

    #[test]
    fn content_affects_fingerprint() {
        let content1 = Content {
            content: "Some string".to_string(),
            tags: BTreeSet::default(),
        };
        let content2 = Content {
            content: "Other string".to_string(),
            tags: BTreeSet::default(),
        };
        assert_ne!(content1.fingerprint(), content2.fingerprint());
    }
}
