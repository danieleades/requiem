use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    title: String,
    body: String,
}

impl MarkdownRequirement {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter).expect("this must never fail");

        // Construct the heading with HRID and title
        let heading = format!("# {} {}", self.hrid, self.title);

        // Combine frontmatter, heading, and body
        let result = if self.body.is_empty() {
            format!("---\n{frontmatter}---\n{heading}\n")
        } else {
            format!("---\n{frontmatter}---\n{heading}\n\n{}\n", self.body)
        };

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

        // Extract HRID, title, and body from content
        let (hrid, title, body) = parse_content(&content)?;

        Ok(Self {
            frontmatter: front,
            hrid,
            title,
            body,
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

/// Parses markdown content into HRID, title, and body.
///
/// The HRID must be the first token in the first heading (after the `#`
/// markers), followed by the title. The body is everything after the first
/// heading.
///
/// # Errors
///
/// Returns an error if no heading is found or if the HRID cannot be parsed.
fn parse_content(content: &str) -> Result<(Hrid, String, String), LoadError> {
    // Find the first non-empty line that starts with '#'
    let (heading_line_idx, line) = content
        .lines()
        .enumerate()
        .find(|(_, line)| line.trim().starts_with('#'))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "No heading found in content - HRID must be in the first heading",
            )
        })?;

    let trimmed = line.trim();
    // Remove leading '#' characters and whitespace
    let after_hashes = trimmed.trim_start_matches('#').trim();

    // Extract the first token (should be the HRID)
    let first_token = after_hashes
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No HRID found in title"))?;

    // Parse the HRID
    let hrid = first_token.parse::<Hrid>().map_err(LoadError::from)?;

    // The rest after the HRID is the title
    let title = after_hashes
        .strip_prefix(first_token)
        .unwrap_or("")
        .trim()
        .to_string();

    // The body is everything after the heading line
    let body = content
        .lines()
        .skip(heading_line_idx + 1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    Ok((hrid, title, body))
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(from = "FrontMatterVersion")]
#[serde(into = "FrontMatterVersion")]
struct FrontMatter {
    uuid: Uuid,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "_version")]
enum FrontMatterVersion {
    #[serde(rename = "1")]
    V1 {
        uuid: Uuid,
        created: DateTime<Utc>,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        tags: BTreeSet<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parents: Vec<Parent>,
    },
}

impl From<FrontMatterVersion> for FrontMatter {
    fn from(version: FrontMatterVersion) -> Self {
        match version {
            FrontMatterVersion::V1 {
                uuid,
                created,
                tags,
                parents,
            } => Self {
                uuid,
                created,
                tags,
                parents,
            },
        }
    }
}

impl From<FrontMatter> for FrontMatterVersion {
    fn from(front_matter: FrontMatter) -> Self {
        let FrontMatter {
            uuid,
            created,
            tags,
            parents,
        } = front_matter;
        Self::V1 {
            uuid,
            created,
            tags,
            parents,
        }
    }
}

impl From<Requirement> for MarkdownRequirement {
    fn from(req: Requirement) -> Self {
        let Requirement {
            content: Content { title, body, tags },
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
            title,
            body,
        }
    }
}

impl TryFrom<MarkdownRequirement> for Requirement {
    type Error = HridError;

    fn try_from(req: MarkdownRequirement) -> Result<Self, Self::Error> {
        let MarkdownRequirement {
            hrid,
            frontmatter:
                FrontMatter {
                    uuid,
                    created,
                    tags,
                    parents,
                },
            title,
            body,
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
            content: Content { title, body, tags },
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
    use std::{io::Cursor, num::NonZeroUsize};

    use chrono::TimeZone;
    use tempfile::TempDir;

    use super::{Parent, *};
    use crate::domain::hrid::KindString;

    fn req_hrid() -> Hrid {
        Hrid::new(
            KindString::new("REQ".to_string()).unwrap(),
            NonZeroUsize::new(1).unwrap(),
        )
    }

    fn create_test_frontmatter() -> FrontMatter {
        let uuid = Uuid::parse_str("12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53").unwrap();
        let created = Utc.with_ymd_and_hms(2025, 7, 14, 7, 15, 0).unwrap();
        let tags = BTreeSet::from(["tag1".to_string(), "tag2".to_string()]);
        let parents = vec![Parent {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            fingerprint: "fingerprint1".to_string(),
            hrid: "REQ-PARENT-001".parse().unwrap(),
        }];
        FrontMatter {
            uuid,
            created,
            tags,
            parents,
        }
    }

    #[test]
    fn markdown_round_trip() {
        let input = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
tags:
- tag1
- tag2
parents:
- uuid: 550e8400-e29b-41d4-a716-446655440000
  fingerprint: fingerprint1
  hrid: REQ-PARENT-001
---
# REQ-001 The Title

This is a paragraph.
";

        let mut reader = Cursor::new(input);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, req_hrid());

        let mut bytes: Vec<u8> = vec![];
        requirement.write(&mut bytes).unwrap();

        let actual = String::from_utf8(bytes).unwrap();
        assert_eq!(input, &actual);
    }

    #[test]
    fn markdown_minimal_content() {
        let hrid = req_hrid();
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
---
# REQ-001 Just content
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, hrid);
        assert_eq!(requirement.title, "Just content");
        assert_eq!(requirement.body, "");
        assert!(requirement.frontmatter.tags.is_empty());
        assert!(requirement.frontmatter.parents.is_empty());
    }

    #[test]
    fn hrid_only_title() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
---
# REQ-001
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, req_hrid());
        assert_eq!(requirement.title, "");
        assert_eq!(requirement.body, "");
    }

    #[test]
    fn multiline_content() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
---
# REQ-001 Title

Line 2

Line 4
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, req_hrid());
        assert_eq!(requirement.title, "Title");
        assert_eq!(requirement.body, "Line 2\n\nLine 4");
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
# REQ-001 Content";

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
            title: "Test content".to_string(),
            body: String::new(),
        };

        let mut buffer = Vec::new();
        let result = requirement.write(&mut buffer);

        assert!(result.is_ok());
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("---"));
        assert!(output.contains("# REQ-001 Test content"));
        // The frontmatter should not have an hrid field at the top level
        // (though parent entries still contain hrid fields)
        let lines: Vec<&str> = output.lines().collect();
        let frontmatter_end = lines
            .iter()
            .skip(1)
            .position(|l| l.trim() == "---")
            .unwrap()
            + 1;
        let frontmatter_lines = &lines[1..frontmatter_end];
        let has_top_level_hrid = frontmatter_lines
            .iter()
            .any(|line| line.starts_with("hrid:") && !line.contains("  "));
        assert!(
            !has_top_level_hrid,
            "Frontmatter should not have top-level hrid field"
        );
    }

    #[test]
    fn save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let frontmatter = create_test_frontmatter();
        let hrid = req_hrid();
        let title = "Saved content".to_string();
        let body = "Some body text".to_string();

        let requirement = MarkdownRequirement {
            frontmatter: frontmatter.clone(),
            hrid: hrid.clone(),
            title: title.clone(),
            body: body.clone(),
        };

        // Test save
        let config = crate::domain::Config::default();
        let save_result = requirement.save(temp_dir.path(), &config);
        assert!(save_result.is_ok());

        // Test load
        let loaded_requirement =
            MarkdownRequirement::load(temp_dir.path(), &hrid, &config).unwrap();
        assert_eq!(loaded_requirement.hrid, hrid);
        assert_eq!(loaded_requirement.title, title);
        assert_eq!(loaded_requirement.body, body);
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
            created,
            tags,
            parents,
        };
        let version: FrontMatterVersion = frontmatter.clone().into();
        let back_to_frontmatter: FrontMatter = version.into();

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
created: 2025-07-14T07:15:00Z
---
# REQ-001 Content

This content has --- in it
And more --- here
";

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, req_hrid());
        assert_eq!(requirement.title, "Content");
        assert_eq!(
            requirement.body,
            "This content has --- in it\nAnd more --- here"
        );
    }

    #[test]
    fn frontmatter_with_special_characters() {
        let content = r#"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
tags:
- "tag with spaces"
- "tag-with-dashes"
- "tag_with_underscores"
---
# REQ-001 Content here
"#;

        let mut reader = Cursor::new(content);
        let requirement = MarkdownRequirement::read(&mut reader).unwrap();

        assert_eq!(requirement.hrid, req_hrid());
        assert!(requirement.frontmatter.tags.contains("tag with spaces"));
        assert!(requirement.frontmatter.tags.contains("tag-with-dashes"));
        assert!(requirement
            .frontmatter
            .tags
            .contains("tag_with_underscores"));
    }

    #[test]
    fn missing_hrid_in_title() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
---
# Just a title without HRID
";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(matches!(result, Err(LoadError::Hrid(_))));
    }

    #[test]
    fn no_heading_in_content() {
        let content = r"---
_version: '1'
uuid: 12b3f5c5-b1a8-4aa8-a882-20ff1c2aab53
created: 2025-07-14T07:15:00Z
---
Just plain text without a heading
";

        let mut reader = Cursor::new(content);
        let result = MarkdownRequirement::read(&mut reader);

        assert!(matches!(result, Err(LoadError::Io(_))));
    }
}
