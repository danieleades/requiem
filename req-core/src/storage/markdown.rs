use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, BufRead, BufReader, Write},
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
    fn write<W: Write>(&self, writer: &mut W, digits: usize) -> io::Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Construct the heading with HRID and title
        let heading = format!("# {} {}", self.hrid.display(digits), self.title);

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

        self.save_to_path(&file_path, config.digits())
    }

    /// Writes the requirement to a specific file path.
    ///
    /// Parent directories are created automatically if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn save_to_path(&self, file_path: &Path, digits: usize) -> io::Result<()> {
        let dir = file_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir)?;

        // Write to a temporary file in the same directory, then rename it over
        // the destination. The rename is atomic on both Unix and Windows, so a
        // crash mid-write can never leave a truncated requirement behind. We
        // deliberately skip fsync: the threat model is crash-truncation, not
        // power loss, and these are plain-text files typically tracked in git.
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        self.write(&mut tmp, digits)?;

        // The temp file is created owner-only on Unix and persist replaces
        // the destination inode, so carry over the destination's existing
        // permissions (or the conventional 0o644 for new files) to avoid
        // silently making shared-readable requirement files private.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::metadata(file_path).map_or_else(
                |_| std::fs::Permissions::from_mode(0o644),
                |metadata| metadata.permissions(),
            );
            tmp.as_file().set_permissions(permissions)?;
        }

        tmp.persist(file_path).map_err(|e| e.error)?;
        Ok(())
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

/// Trim empty lines from the start and end of a string, preserving indentation.
///
/// Unlike `.trim()`, this function only removes completely empty lines from
/// the beginning and end, keeping any leading/trailing whitespace on non-empty
/// lines. This is crucial for preserving markdown structures like code blocks,
/// lists, and blockquotes which rely on indentation.
pub(crate) fn trim_empty_lines(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();

    // Find first and last non-empty lines
    let first_non_empty = lines.iter().position(|line| !line.trim().is_empty());
    let last_non_empty = lines.iter().rposition(|line| !line.trim().is_empty());

    match (first_non_empty, last_non_empty) {
        (Some(start), Some(end)) => lines[start..=end].join("\n"),
        _ => String::new(),
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
    // Preserve leading indentation but trim empty lines from start/end
    let body_content: String = content
        .lines()
        .skip(heading_line_idx + 1)
        .collect::<Vec<_>>()
        .join("\n");
    let body = trim_empty_lines(&body_content);

    Ok((hrid, title, body))
}

/// Errors that can occur when loading a requirement from markdown.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The requirement file was not found.
    #[error("requirement file not found")]
    NotFound,

    /// An I/O error occurred.
    #[error("failed to read file: {0}")]
    Io(#[from] io::Error),

    /// The YAML frontmatter could not be parsed.
    #[error("invalid YAML frontmatter: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// The HRID could not be parsed.
    #[error("invalid HRID in title: {0}")]
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
    // Use default 3-digit formatting for frontmatter serialization
    serializer.serialize_str(&hrid.display(3).to_string())
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
        requirement.write(&mut bytes, 3).unwrap();

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
        let result = requirement.write(&mut buffer, 3);

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
    fn save_to_path_overwrite_leaves_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("REQ-001.md");

        let mut requirement = MarkdownRequirement {
            frontmatter: create_test_frontmatter(),
            hrid: req_hrid(),
            title: "First".to_string(),
            body: "first body".to_string(),
        };
        requirement.save_to_path(&path, 3).unwrap();

        requirement.body = "second body".to_string();
        requirement.save_to_path(&path, 3).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("second body"));

        // The atomic write must not leave temporary files behind.
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("REQ-001.md")]);
    }

    #[cfg(unix)]
    #[test]
    fn save_to_path_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("REQ-001.md");
        let requirement = MarkdownRequirement {
            frontmatter: create_test_frontmatter(),
            hrid: req_hrid(),
            title: "Title".to_string(),
            body: String::new(),
        };

        // New files get the conventional world-readable mode, not the
        // temp file's owner-only mode.
        requirement.save_to_path(&path, 3).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o644);

        // Overwriting keeps the destination's existing mode.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
        requirement.save_to_path(&path, 3).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o664);
    }

    #[cfg(unix)]
    #[test]
    fn failed_save_preserves_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("REQ-001.md");

        let mut requirement = MarkdownRequirement {
            frontmatter: create_test_frontmatter(),
            hrid: req_hrid(),
            title: "First".to_string(),
            body: "original body".to_string(),
        };
        requirement.save_to_path(&path, 3).unwrap();

        // Make the directory read-only so the temp file cannot be created.
        let original_perms = std::fs::metadata(temp_dir.path()).unwrap().permissions();
        std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        requirement.body = "replacement body".to_string();
        let result = requirement.save_to_path(&path, 3);

        std::fs::set_permissions(temp_dir.path(), original_perms).unwrap();

        assert!(result.is_err());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("original body"));
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

    #[test]
    fn trim_empty_lines_removes_only_empty_lines() {
        assert_eq!(trim_empty_lines(""), "");
        assert_eq!(trim_empty_lines("\n\n"), "");
        assert_eq!(trim_empty_lines("content"), "content");
        assert_eq!(trim_empty_lines("\n\ncontent\n\n"), "content");
    }

    #[test]
    fn trim_empty_lines_preserves_leading_indentation() {
        assert_eq!(trim_empty_lines("    indented"), "    indented");
        assert_eq!(trim_empty_lines("\n    indented\n"), "    indented");
        assert_eq!(
            trim_empty_lines("    code block\n    more code"),
            "    code block\n    more code"
        );
    }

    #[test]
    fn trim_empty_lines_preserves_internal_empty_lines() {
        assert_eq!(trim_empty_lines("line1\n\nline2"), "line1\n\nline2");
        assert_eq!(trim_empty_lines("\nline1\n\nline2\n"), "line1\n\nline2");
    }

    #[test]
    fn trim_empty_lines_handles_markdown_structures() {
        // Code block
        let code = "    fn main() {\n        println!(\"hello\");\n    }";
        assert_eq!(trim_empty_lines(code), code);

        // List with indentation
        let list = "- Item 1\n  - Sub item\n- Item 2";
        assert_eq!(trim_empty_lines(list), list);

        // Blockquote
        let quote = "> This is a quote\n> with multiple lines";
        assert_eq!(trim_empty_lines(quote), quote);
    }

    #[test]
    fn trim_empty_lines_with_trailing_whitespace_lines() {
        // Lines with only spaces should be treated as empty
        assert_eq!(trim_empty_lines("   \n\ncontent\n   \n"), "content");
    }

    #[test]
    fn parse_content_preserves_body_indentation() {
        let content = "# REQ-001 Title\n\n    code block\n    more code";
        let (hrid, title, body) = parse_content(content).unwrap();

        assert_eq!(hrid.display(3).to_string(), "REQ-001");
        assert_eq!(title, "Title");
        assert_eq!(body, "    code block\n    more code");
    }

    #[test]
    fn parse_content_trims_empty_lines_around_body() {
        let content = "# REQ-001 Title\n\n\n\ncontent\n\n\n";
        let (_hrid, _title, body) = parse_content(content).unwrap();

        assert_eq!(body, "content");
    }

    #[test]
    fn round_trip_preserves_indentation() {
        let temp_dir = TempDir::new().unwrap();
        let frontmatter = create_test_frontmatter();
        let hrid = req_hrid();
        let title = "Code Example".to_string();
        let body = "Here's a code block:\n\n    fn main() {\n        println!(\"hello\");\n    \
                    }\n\nAnd a list:\n\n- Item 1\n  - Sub item\n- Item 2"
            .to_string();

        let requirement = MarkdownRequirement {
            frontmatter,
            hrid: hrid.clone(),
            title,
            body: body.clone(),
        };

        // Save and reload
        let config = crate::domain::Config::default();
        requirement.save(temp_dir.path(), &config).unwrap();
        let loaded = MarkdownRequirement::load(temp_dir.path(), &hrid, &config).unwrap();

        // Verify indentation is preserved
        assert_eq!(loaded.body, body);
    }
}
