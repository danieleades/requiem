use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

/// Configuration for requirements management.
///
/// This struct holds settings that control how requirements are managed,
/// including HRID formatting, directory structure modes, and validation rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Versions", into = "Versions")]
pub struct Config {
    /// The kinds of requirements that are allowed.
    ///
    /// This is the first component of the HRID.
    /// For example, 'USR' or 'SYS'.
    ///
    /// If this is empty, all kinds are allowed.
    allowed_kinds: Vec<String>,

    /// Optional metadata describing requirement kinds.
    ///
    /// Keyed by the KIND identifier (e.g., "USR").
    kind_metadata: HashMap<String, KindMetadata>,

    /// The number of digits in the HRID.
    ///
    /// Digits are padded to this width with leading zeros.
    ///
    /// This is the second component of the HRID.
    /// For example, '001' (3 digits) or '0001' (4 digits).
    digits: usize,

    /// Whether to allow the requirements directory to contain markdown files
    /// with names that are not valid HRIDs
    pub allow_unrecognised: bool,

    /// Whether subfolder paths contribute to the namespace of requirements.
    ///
    /// When `false` (default): The full HRID is encoded in the filename.
    ///   Example: `system/auth/REQ-001.md` -> HRID is parsed as `REQ-001`
    ///   Example: `custom/system-auth-REQ-001.md` -> HRID is
    /// `system-auth-REQ-001`
    ///
    /// When `true`: Subfolders encode the namespace, filename contains KIND-ID.
    ///   Example: `system/auth/REQ-001.md` -> HRID is `system-auth-REQ-001`
    ///   Example: `system/auth/USR/001.md` -> HRID is `system-auth-USR-001`
    ///   (The format is inferred: numeric filename means KIND in parent folder)
    pub subfolders_are_namespaces: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allowed_kinds: Vec::new(),
            kind_metadata: HashMap::new(),
            digits: default_digits(),
            allow_unrecognised: false,
            subfolders_are_namespaces: false,
        }
    }
}

impl Config {
    /// Loads the configuration from a TOML file at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if the TOML content is
    /// invalid.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))
    }

    /// Saves the configuration to a TOML file at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be serialized to TOML or if
    /// the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write config file: {e}"))
    }

    /// Returns the number of digits for padding HRID IDs.
    #[must_use]
    pub const fn digits(&self) -> usize {
        self.digits
    }

    /// Returns the allowed kinds, if configured.
    #[must_use]
    pub fn allowed_kinds(&self) -> &[String] {
        &self.allowed_kinds
    }

    /// Returns the metadata for all configured kinds.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn kind_metadata(&self) -> &HashMap<String, KindMetadata> {
        &self.kind_metadata
    }

    /// Returns metadata for a specific kind, if present.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn metadata_for_kind(&self, kind: &str) -> Option<&KindMetadata> {
        self.kind_metadata.get(kind)
    }

    /// Checks if a kind is allowed by the configuration.
    ///
    /// If `allowed_kinds` is empty, all kinds are allowed.
    /// Otherwise, the kind must be in the allowed list.
    #[must_use]
    pub fn is_kind_allowed(&self, kind: &str) -> bool {
        self.allowed_kinds.is_empty() || self.allowed_kinds.iter().any(|k| k == kind)
    }

    /// Sets the `subfolders_are_namespaces` configuration option.
    pub const fn set_subfolders_are_namespaces(&mut self, value: bool) {
        self.subfolders_are_namespaces = value;
    }

    /// Adds a kind to the allowed kinds list.
    ///
    /// If the kind already exists, it is not added again.
    /// Kinds are normalized to uppercase before adding.
    ///
    /// Returns `true` if the kind was added, `false` if it already existed.
    pub fn add_kind(&mut self, kind: &str) -> bool {
        let kind = kind.to_uppercase();
        if self.allowed_kinds.contains(&kind) {
            false
        } else {
            self.allowed_kinds.push(kind);
            true
        }
    }

    /// Removes a kind from the allowed kinds list.
    ///
    /// Kinds are normalized to uppercase before removal.
    ///
    /// Returns `true` if the kind was removed, `false` if it didn't exist.
    pub fn remove_kind(&mut self, kind: &str) -> bool {
        let kind = kind.to_uppercase();
        if let Some(pos) = self.allowed_kinds.iter().position(|k| k == &kind) {
            self.allowed_kinds.remove(pos);
            self.kind_metadata.remove(&kind);
            true
        } else {
            false
        }
    }

    /// Sets or clears a description for a kind (stored uppercase).
    ///
    /// An empty or `None` description removes existing metadata.
    pub fn set_kind_description(&mut self, kind: &str, description: Option<String>) {
        let key = kind.to_uppercase();
        let description = description
            .and_then(|d| {
                let trimmed = d.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .map(Some);

        match description {
            Some(description) => {
                self.kind_metadata.insert(key, KindMetadata { description });
            }
            None => {
                self.kind_metadata.remove(&key);
            }
        }
    }
}

const fn default_digits() -> usize {
    3
}

/// Metadata describing a requirement kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KindMetadata {
    /// Human-readable description of the kind's purpose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// The serialized versions of the configuration.
/// This allows for future changes to the configuration format and to the domain
/// type without breaking compatibility.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "_version")]
enum Versions {
    #[serde(rename = "1")]
    V1 {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed_kinds: Vec<AllowedKindEntry>,

        /// The number of digits in the HRID.
        ///
        /// Digits are padded to this width with leading zeros.
        ///
        /// This is the second component of the HRID.
        /// For example, '001' (3 digits) or '0001' (4 digits).
        #[serde(default = "default_digits")]
        digits: usize,

        #[serde(default)]
        allow_unrecognised: bool,

        /// Deprecated: This field is no longer used but kept for backward
        /// compatibility
        #[serde(default, skip_serializing)]
        allow_invalid: bool,

        #[serde(default)]
        subfolders_are_namespaces: bool,
    },
}

impl From<Versions> for super::Config {
    fn from(versions: Versions) -> Self {
        match versions {
            Versions::V1 {
                allowed_kinds,
                digits,
                allow_unrecognised,
                allow_invalid: _, // Ignored for backward compatibility
                subfolders_are_namespaces,
            } => Self {
                allowed_kinds: allowed_kinds
                    .iter()
                    .map(AllowedKindEntry::kind)
                    .map(ToString::to_string)
                    .collect(),
                kind_metadata: allowed_kinds
                    .into_iter()
                    .filter_map(AllowedKindEntry::into_metadata)
                    .collect(),
                digits,
                allow_unrecognised,
                subfolders_are_namespaces,
            },
        }
    }
}

impl From<super::Config> for Versions {
    fn from(config: super::Config) -> Self {
        let super::Config {
            allowed_kinds,
            mut kind_metadata,
            digits,
            allow_unrecognised,
            subfolders_are_namespaces,
        } = config;

        let mut serialized_kinds: Vec<AllowedKindEntry> = allowed_kinds
            .iter()
            .map(|kind| {
                kind_metadata.remove(kind).map_or_else(
                    || AllowedKindEntry::Simple(kind.clone()),
                    |meta| AllowedKindEntry::Detailed {
                        kind: kind.clone(),
                        description: meta.description,
                    },
                )
            })
            .collect();

        let mut remaining: Vec<_> = kind_metadata.into_iter().collect();
        remaining.sort_by(|a, b| a.0.cmp(&b.0));
        serialized_kinds.extend(remaining.into_iter().map(|(kind, meta)| {
            AllowedKindEntry::Detailed {
                kind,
                description: meta.description,
            }
        }));

        Self::V1 {
            allowed_kinds: serialized_kinds,
            digits,
            allow_unrecognised,
            allow_invalid: false, // No longer used
            subfolders_are_namespaces,
        }
    }
}

/// Serialization helper for allowed kinds that supports either bare strings or
/// inline tables with metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum AllowedKindEntry {
    /// A bare kind identifier, e.g. "USR".
    Simple(String),
    /// A kind identifier with optional metadata fields.
    Detailed {
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

impl AllowedKindEntry {
    fn kind(&self) -> &str {
        match self {
            Self::Simple(kind) | Self::Detailed { kind, .. } => kind,
        }
    }

    fn into_metadata(self) -> Option<(String, KindMetadata)> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { kind, description } => Some((kind, KindMetadata { description })),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn load_reads_valid_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            b"_version = \"1\"\nallowed_kinds = [\"USR\", \"SYS\"]\ndigits = 4\nallow_unrecognised = true\nsubfolders_are_namespaces = true\n",
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();

        assert_eq!(
            config.allowed_kinds(),
            &["USR".to_string(), "SYS".to_string()]
        );
        assert_eq!(config.digits(), 4);
        assert!(config.allow_unrecognised);
        assert!(config.subfolders_are_namespaces);
    }

    #[test]
    fn load_missing_file_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("missing.toml");

        let error = Config::load(&missing).unwrap_err();
        assert!(error.starts_with("Failed to read config file:"));
    }

    #[test]
    fn load_invalid_toml_returns_error() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"_version = \"1\"\ndigits = \"three\"\n")
            .unwrap();

        let error = Config::load(file.path()).unwrap_err();
        assert!(error.starts_with("Failed to parse config file:"));
    }

    #[test]
    fn load_kinds_with_metadata() {
        let config: Config = toml::from_str(
            r#"_version = "1"
allowed_kinds = [
  { kind = "USR", description = "User-facing change" },
  "SYS"
]
"#,
        )
        .unwrap();

        assert_eq!(
            config.allowed_kinds(),
            &["USR".to_string(), "SYS".to_string()]
        );

        let usr = config.kind_metadata().get("USR").unwrap();
        assert_eq!(usr.description.as_deref(), Some("User-facing change"));

        assert!(config.kind_metadata().get("SYS").is_none());
    }

    #[test]
    fn set_kind_description_adds_and_removes_metadata() {
        let mut config = Config::default();
        config.add_kind("usr");

        config.set_kind_description("usr", Some("User stories".into()));
        assert_eq!(
            config
                .metadata_for_kind("USR")
                .and_then(|m| m.description.as_deref()),
            Some("User stories")
        );

        config.set_kind_description("USR", Some("Refined".into()));
        assert_eq!(
            config
                .metadata_for_kind("USR")
                .and_then(|m| m.description.as_deref()),
            Some("Refined")
        );

        config.set_kind_description("usr", Some("   ".into()));
        assert!(config.metadata_for_kind("USR").is_none());
    }

    #[test]
    fn empty_file_returns_default() {
        // Tests that deserialising an empty file returns the default configuration.
        let expected = Config::default();
        let actual: Config = toml::from_str(r#"_version = "1""#).unwrap();
        assert_eq!(actual, expected);
    }
}
