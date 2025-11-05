use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_evolve::Versioned;
use uuid::Uuid;

use crate::{
    domain::{
        requirement::{Content, Metadata, Parent as DomainParent},
        Hrid, HridError,
    },
    Requirement,
};

/// A requirement serialized in markdown format with YAML frontmatter.
#[derive(Debug, Clone)]
pub struct MarkdownRequirement {
    frontmatter: FrontMatter,
    hrid: Hrid,
    content: String,
}

impl MarkdownRequirement {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter).expect("this must never fail");
        let result = format!("---\n{frontmatter}---\n{}\n", self.content);
        writer.write_all(result.as_bytes())
    }

    pub(crate) fn read<R: BufRead>(reader: &mut R) -> Result<Self, LoadError> {
        let mut lines = reader.lines();

        // Ensure frontmatter starts correctly
        let first_line = lines
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "Empty input"))?
            .map_err(LoadError::from)?;

        if first_line.trim() != "---" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected frontmatter starting with '---'",
            )
            .into());
        }

        // Collect lines until next '---'
        let frontmatter = lines
            .by_ref()
            .map_while(|line| match line {
                Ok(content) if content.trim() == "---" => None,
                Ok(content) => Some(Ok(content)),
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        // The rest of the lines are Markdown content
        let content = lines.collect::<Result<Vec<_>, _>>()?.join("\n");

        let front: FrontMatter = serde_yaml::from_str(&frontmatter)?;
        let hrid = front.hrid.clone();

        Ok(Self {
            frontmatter: front,
            hrid,
            content,
        })
    }

    /// Writes the requirement to a file path constructed using the given
    /// config.
    ///
    /// The path construction respects the `subfolders_are_namespaces` setting:
    /// - If `false`: file is saved as `root/FULL-HRID.md`
    /// - If `true`: file is saved as `root/namespace/folders/KIND-ID.md`
    ///
    /// Parent directories are created automatically if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn save(&self, root: &Path, config: &crate::domain::Config) -> io::Result<()> {
        use crate::storage::construct_path_from_hrid;

        let file_path = construct_path_from_hrid(
            root,
            &self.hrid,
            config.subfolders_are_namespaces,
            config.digits(),
        );

        self.save_to_path(&file_path)
    }

    /// Writes the requirement to a specific file path.
    ///
    /// Parent directories are created automatically if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn save_to_path(&self, file_path: &Path) -> io::Result<()> {
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        self.write(&mut writer)
    }

    /// Reads a requirement using the given configuration.
    ///
    /// The path construction respects the `subfolders_are_namespaces` setting:
    /// - If `false`: loads from `root/FULL-HRID.md`
    /// - If `true`: loads from `root/namespace/folders/KIND-ID.md`
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(
        root: &Path,
        hrid: &Hrid,
        config: &crate::domain::Config,
    ) -> Result<Self, LoadError> {
        use crate::storage::construct_path_from_hrid;

        let file_path = construct_path_from_hrid(
            root,
            hrid,
            config.subfolders_are_namespaces,
            config.digits(),
        );

        let file = File::open(&file_path).map_err(|io_error| match io_error.kind() {
            io::ErrorKind::NotFound => LoadError::NotFound,
            _ => LoadError::Io(io_error),
        })?;

        let mut reader = BufReader::new(file);
        Self::read(&mut reader)
    }
}

/// Errors that can occur when loading a requirement from markdown.
#[derive(Debug, thiserror::Error)]
#[error("failed to read from markdown")]
pub enum LoadError {
    /// The requirement file was not found.
    NotFound,
    /// An I/O error occurred.
    Io(#[from] io::Error),
    /// The YAML frontmatter could not be parsed.
    Yaml(#[from] serde_yaml::Error),
    /// The HRID could not be parsed.
    Hrid(#[from] HridError),
}

/// YAML frontmatter for markdown requirements.
#[derive(Debug, Clone, PartialEq, Versioned)]
#[versioned(mode = "infallible", chain(FrontMatterV1), transparent = true)]
struct FrontMatter {
    uuid: Uuid,
    hrid: Hrid,
    created: DateTime<Utc>,
    tags: BTreeSet<String>,
    parents: Vec<Parent>,
}

/// A parent requirement reference in the serialized format.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Parent {
    uuid: Uuid,
    fingerprint: String,
    #[serde(
        serialize_with = "hrid_as_string",
        deserialize_with = "hrid_from_string"
    )]
    hrid: Hrid,
}

/// Serialize an HRID as a string.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn hrid_as_string<S>(hrid: &Hrid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hrid.to_string())
}

/// Deserialize an HRID from a string.
///
/// # Errors
///
/// Returns an error if the string cannot be parsed as a valid HRID.
pub fn hrid_from_string<'de, D>(deserializer: D) -> Result<Hrid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Hrid::try_from(s.as_str()).map_err(serde::de::Error::custom)
}

/// Version 1 of the serialized frontmatter format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatterV1 {
    uuid: Uuid,
    #[serde(
        serialize_with = "hrid_as_string",
        deserialize_with = "hrid_from_string"
    )]
    hrid: Hrid,
    created: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    tags: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    parents: Vec<Parent>,
}

impl From<FrontMatterV1> for FrontMatter {
    fn from(v1: FrontMatterV1) -> Self {
        Self {
            uuid: v1.uuid,
            hrid: v1.hrid,
            created: v1.created,
            tags: v1.tags,
            parents: v1.parents,
        }
    }
}

impl From<&FrontMatter> for FrontMatterV1 {
    fn from(front_matter: &FrontMatter) -> Self {
        Self {
            uuid: front_matter.uuid,
            hrid: front_matter.hrid.clone(),
            created: front_matter.created,
            tags: front_matter.tags.clone(),
            parents: front_matter.parents.clone(),
        }
    }
}

impl From<Requirement> for MarkdownRequirement {
    fn from(req: Requirement) -> Self {
        let Requirement {
            content: Content { content, tags },
            metadata:
                Metadata {
                    uuid,
                    hrid,
                    created,
                    parents,
                },
        } = req;

        let frontmatter = FrontMatter {
            uuid,
            hrid: hrid.clone(),
            created,
            tags,
            parents: parents
                .into_iter()
                .map(|(uuid, DomainParent { hrid, fingerprint })| Parent {
                    uuid,
                    fingerprint,
                    hrid,
                })
                .collect(),
        };

        Self {
            frontmatter,
            hrid,
            content,
        }
    }
}

impl TryFrom<MarkdownRequirement> for Requirement {
    type Error = HridError;

    fn try_from(req: MarkdownRequirement) -> Result<Self, Self::Error> {
        let MarkdownRequirement {
            hrid: _,
            frontmatter:
                FrontMatter {
                    uuid,
                    hrid,
                    created,
                    tags,
                    parents,
                },
            content,
        } = req;

        let parent_map = parents
            .into_iter()
            .map(|parent| {
                let Parent {
                    uuid,
                    fingerprint,
                    hrid: parent_hrid,
                } = parent;
                Ok((
                    uuid,
                    DomainParent {
                        hrid: parent_hrid,
                        fingerprint,
                    },
                ))
            })
            .collect::<Result<_, Self::Error>>()?;

        Ok(Self {
            content: Content { content, tags },
            metadata: Metadata {
                uuid,
                hrid,
                created,
                parents: parent_map,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, io::Cursor, num::NonZeroUsize};

    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::{FrontMatter, FrontMatterV1, Hrid, LoadError, MarkdownRequirement, Parent};
    use crate::domain::hrid::KindString;

    fn req_hrid() -> Hrid {
        Hrid::new(
            KindString::new("REQ".to_string()).unwrap(),
            NonZeroUsize::new(1).unwrap(),
        )
    }

    fn create_test_frontmatter() -> FrontMatter {
        let uuid = Uuid::parse_str("12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53").unwrap();
        let hrid = req_hrid();
        let created = Utc.with_ymd_and_hms(2025, 7, 14, 7, 15, 0).unwrap();
        let tags = BTreeSet::from(["tag1".to_string(), "tag2".to_string()]);
        let parents = vec![Parent {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            fingerprint: "fingerprint1".to_string(),
            hrid: "REQ-PARENT-001".parse().unwrap(),
        }];
        FrontMatter {
            uuid,
            hrid,
            created,
            tags,
            parents,
        }
    }

    #[test]
    fn markdown_round_trip() {
        let expected = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
tags:
- tag1
- tag2
parents:
- uuid: 550e8400-e29b-41d4-a716-446655440000
  fingerprint: fingerprint1
  hrid: REQ-PARENT-001
---

# The Title

This is a paragraph.
";

        let mut reader = Cursor::new(expected);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        let mut bytes: Vec<u8> = vec![];
        requirement.write(&mut bytes).unwrap();

        let actual = String::from_utf8(bytes).unwrap();
        assert_eq!(expected, &actual);
    }

    #[test]
    fn markdown_minimal_content() {
        let hrid = req_hrid();
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
---
Just content
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, hrid);
        assert_eq!(requirement.content, "Just content");
        assert!(requirement.frontmatter.tags.is_empty());
        assert!(requirement.frontmatter.parents.is_empty());
    }

    #[test]
    fn empty_content() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
---
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.content, "");
    }

    #[test]
    fn multiline_content() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
---
Line 1
Line 2

Line 4
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.content, "Line 1\nLine 2\n\nLine 4");
    }

    #[test]
    fn invalid_frontmatter_start() {
        let content = "invalid frontmatter";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(result.is_err());
    }

    #[test]
    fn missing_frontmatter_end() {
        let content = r"---
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
This should be content but there's no closing ---";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(result.is_err());
    }

    #[test]
    fn invalid_yaml() {
        let content = r"---
invalid: yaml: structure:
created: not-a-date
---
Content";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(matches!(result, Err(LoadError::Yaml(_))));
    }

    #[test]
    fn empty_input() {
        let content = "";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(result.is_err());
    }

    #[test]
    fn write_success() {
        let frontmatter = create_test_frontmatter();
        let requirement = MarkdownRequirement {
            frontmatter,
            hrid: req_hrid(),
            content: "Test content".to_string(),
        };

        let mut buffer = Vec::new();
        let result = requirement.write(&mut buffer);

        assert!(result.is_ok());
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("---"));
        assert!(output.contains("Test content"));
    }

    #[test]
    fn save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let frontmatter = create_test_frontmatter();
        let hrid = req_hrid();
        let content = "Saved content".to_string();

        let requirement = MarkdownRequirement {
            frontmatter: frontmatter.clone(),
            hrid: hrid.clone(),
            content: content.clone(),
        };

        // Test save
        let config = crate::domain::Config::default();
        let save_result = requirement.save(temp_dir.path(), &config);
        assert!(save_result.is_ok());

        // Test load
        let loaded_requirement =
            MarkdownRequirement::load(temp_dir.path(), &hrid, &config).unwrap();
        assert_eq!(loaded_requirement.hrid, hrid);
        assert_eq!(loaded_requirement.content, content);
        assert_eq!(loaded_requirement.frontmatter, frontmatter);
    }

    #[test]
    fn load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::domain::Config::default();
        let result = MarkdownRequirement::load(temp_dir.path(), &req_hrid(), &config);
        assert!(matches!(result, Err(LoadError::NotFound)));
    }

    #[test]
    fn frontmatter_version_conversion() {
        let uuid = Uuid::parse_str("12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53").unwrap();
        let created = Utc.with_ymd_and_hms(2025, 7, 14, 7, 15, 0).unwrap();
        let tags = BTreeSet::from(["tag1".to_owned()]);
        let parents = vec![Parent {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            fingerprint: "fp1".to_string(),
            hrid: req_hrid(),
        }];

        let frontmatter = FrontMatter {
            uuid,
            hrid: req_hrid(),
            created,
            tags,
            parents,
        };
        let v1: FrontMatterV1 = (&frontmatter).into();
        let back_to_frontmatter: FrontMatter = v1.into();

        assert_eq!(frontmatter, back_to_frontmatter);
    }

    #[test]
    fn parent_creation() {
        let uuid = Uuid::parse_str("12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53").unwrap();
        let fingerprint = "test-fingerprint".to_string();
        let hrid = req_hrid();

        let parent = Parent {
            uuid,
            fingerprint: fingerprint.clone(),
            hrid: hrid.clone(),
        };

        assert_eq!(parent.uuid, uuid);
        assert_eq!(parent.fingerprint, fingerprint);
        assert_eq!(parent.hrid, hrid);
    }

    #[test]
    fn content_with_triple_dashes() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
---
This content has --- in it
And more --- here
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(
            requirement.content,
            "This content has --- in it\nAnd more --- here"
        );
    }

    #[test]
    fn frontmatter_with_special_characters() {
        let content = r#"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
hrid: REQ-001
created: 2025-07-14T07:15:00Z
tags:
- "tag with spaces"
- "tag-with-dashes"
- "tag_with_underscores"
---
Content here
"#;

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert!(requirement.frontmatter.tags.contains("tag with spaces"));
        assert!(requirement.frontmatter.tags.contains("tag-with-dashes"));
        assert!(requirement
            .frontmatter
            .tags
            .contains("tag_with_underscores"));
    }
}
