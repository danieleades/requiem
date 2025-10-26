use std::path::Path;

use serde::{Deserialize, Serialize};

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

    /// Whether to allow markdown files with names that are valid HRIDs that are
    /// not correctly formatted
    pub allow_invalid: bool,

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
            digits: default_digits(),
            allow_unrecognised: false,
            allow_invalid: false,
            subfolders_are_namespaces: false,
        }
    }
}

impl Config {
    /// Loads the configuration from a TOML file at the given path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))
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
}

const fn default_digits() -> usize {
    3
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
        allowed_kinds: Vec<String>,

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

        #[serde(default)]
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
                allow_invalid,
                subfolders_are_namespaces,
            } => Self {
                allowed_kinds,
                digits,
                allow_unrecognised,
                allow_invalid,
                subfolders_are_namespaces,
            },
        }
    }
}

impl From<super::Config> for Versions {
    fn from(config: super::Config) -> Self {
        Self::V1 {
            allowed_kinds: config.allowed_kinds,
            digits: config.digits,
            allow_unrecognised: config.allow_unrecognised,
            allow_invalid: config.allow_invalid,
            subfolders_are_namespaces: config.subfolders_are_namespaces,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_reads_valid_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            file,
            "{content}",
            content = "_version = \"1\"\nallowed_kinds = [\"USR\", \"SYS\"]\ndigits = 4\nallow_unrecognised = true\nallow_invalid = true\nsubfolders_are_namespaces = true"
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();

        assert_eq!(
            config.allowed_kinds(),
            &["USR".to_string(), "SYS".to_string()]
        );
        assert_eq!(config.digits(), 4);
        assert!(config.allow_unrecognised);
        assert!(config.allow_invalid);
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
        writeln!(
            file,
            "{content}",
            content = "_version = \"1\"\ndigits = \"three\""
        )
        .unwrap();

        let error = Config::load(file.path()).unwrap_err();
        assert!(error.starts_with("Failed to parse config file:"));
    }

    #[test]
    fn empty_file_returns_default() {
        // Tests that deserialising an empty file returns the default configuration.
        let expected = Config::default();
        let actual: Config = toml::from_str(r#"_version = "1""#).unwrap();
        assert_eq!(actual, expected);
    }
}
