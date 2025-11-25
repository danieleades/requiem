use std::{fmt, num::NonZeroUsize, ops::Deref, str::FromStr};

use non_empty_string::NonEmptyString;

/// A validated string containing only uppercase alphabetic characters ([A-Z]+).
///
/// Used for HRID kind and namespace segments to ensure they conform to the
/// required format.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KindString(NonEmptyString);

impl KindString {
    /// Creates a new `KindString` from a string.
    ///
    /// # Errors
    ///
    /// Returns `InvalidKindError` if the string is empty or contains
    /// characters other than uppercase letters (A-Z).
    pub fn new(s: String) -> Result<Self, InvalidKindError> {
        // Check non-empty
        let non_empty = NonEmptyString::new(s.clone()).map_err(|_| InvalidKindError(s.clone()))?;

        // Check all characters are uppercase ASCII letters
        if !s.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(InvalidKindError(s));
        }

        Ok(Self(non_empty))
    }

    /// Returns the string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for KindString {
    type Error = InvalidKindError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for KindString {
    type Error = InvalidKindError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}

impl AsRef<str> for KindString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for KindString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for KindString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for KindString {
    type Err = InvalidKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

/// Error returned when a string doesn't match the required pattern [A-Z]+.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("Invalid kind string '{0}': must be non-empty and contain only uppercase letters (A-Z)")]
pub struct InvalidKindError(String);

/// A human-readable identifier (HRID) for a requirement.
///
/// Format:
/// `{NAMESPACE*}-{KIND}-{ID}`, where:
/// - `NAMESPACE` is an optional sequence of uppercase alphabetic segments (e.g.
///   `COMPONENT-SUBCOMPONENT`)
/// - `KIND` is an uppercase alphabetic category string (e.g. `URS`, `SYS`)
/// - `ID` is a positive non-zero integer (e.g. `001`, `123`)
///
/// Examples: `URS-001`, `SYS-099`, `COMPONENT-SUBCOMPONENT-SYS-005`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hrid {
    namespace: Vec<KindString>,
    kind: KindString,
    id: NonZeroUsize,
}

impl Hrid {
    /// Create an HRID with no namespace.
    ///
    /// This is an infallible constructor that takes pre-validated types.
    #[must_use]
    pub const fn new(kind: KindString, id: NonZeroUsize) -> Self {
        Self::new_with_namespace(Vec::new(), kind, id)
    }

    /// Create an HRID with the given namespace.
    ///
    /// This is an infallible constructor that takes pre-validated types.
    #[must_use]
    pub const fn new_with_namespace(
        namespace: Vec<KindString>,
        kind: KindString,
        id: NonZeroUsize,
    ) -> Self {
        Self {
            namespace,
            kind,
            id,
        }
    }

    /// Returns the namespace segments as strings.
    pub fn namespace(&self) -> Vec<&str> {
        self.namespace.iter().map(KindString::as_str).collect()
    }

    /// Returns the kind component as a string.
    #[must_use]
    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }

    /// Returns the numeric ID component.
    #[must_use]
    pub const fn id(&self) -> NonZeroUsize {
        self.id
    }

    /// Returns the prefix (namespace + kind) without the numeric ID.
    ///
    /// For example:
    /// - "USR" for a requirement with no namespace and kind "USR"
    /// - "AUTH-USR" for a requirement with namespace `["AUTH"]` and kind "USR"
    #[must_use]
    pub fn prefix(&self) -> String {
        if self.namespace.is_empty() {
            self.kind.to_string()
        } else {
            let namespace_str = self
                .namespace
                .iter()
                .map(KindString::as_str)
                .collect::<Vec<_>>()
                .join("-");
            format!("{}-{}", namespace_str, self.kind)
        }
    }

    /// Returns a displayable representation with the specified digit width.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use requiem::{domain::hrid::KindString, Hrid};
    ///
    /// let kind = KindString::new("USR".to_string()).unwrap();
    /// let id = NonZeroUsize::new(42).unwrap();
    /// let hrid = Hrid::new(kind, id);
    ///
    /// assert_eq!(hrid.display(3).to_string(), "USR-042");
    /// assert_eq!(hrid.display(4).to_string(), "USR-0042");
    /// assert_eq!(hrid.display(2).to_string(), "USR-42");
    /// ```
    #[must_use]
    pub const fn display(&self, digits: usize) -> FormattedHrid<'_> {
        FormattedHrid { hrid: self, digits }
    }
}

/// A wrapper type that formats an HRID with a specified digit width.
///
/// This type is returned by [`Hrid::display`] and implements [`fmt::Display`]
/// to format the HRID with the configured number of digits.
#[derive(Debug, Clone, Copy)]
pub struct FormattedHrid<'a> {
    hrid: &'a Hrid,
    digits: usize,
}

impl fmt::Display for FormattedHrid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id_str = format!("{:0width$}", self.hrid.id, width = self.digits);
        if self.hrid.namespace.is_empty() {
            write!(f, "{}-{}", self.hrid.kind, id_str)
        } else {
            let namespace_str = self
                .hrid
                .namespace
                .iter()
                .map(KindString::as_str)
                .collect::<Vec<_>>()
                .join("-");
            write!(f, "{}-{}-{}", namespace_str, self.hrid.kind, id_str)
        }
    }
}

/// Errors that can occur during HRID parsing or construction.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// Invalid HRID format (malformed structure).
    #[error("Invalid HRID format: {0}")]
    Syntax(String),

    /// Invalid ID value in HRID (non-numeric or zero).
    #[error("Invalid ID in HRID '{0}': expected a non-zero integer, got {1}")]
    Id(String, String),

    /// ID cannot be zero.
    #[error("Invalid ID: cannot be zero")]
    ZeroId,

    /// Invalid kind string (not uppercase alphabetic).
    #[error(transparent)]
    Kind(InvalidKindError),
}

impl From<InvalidKindError> for Error {
    fn from(err: InvalidKindError) -> Self {
        Self::Kind(err)
    }
}

impl FromStr for Hrid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Early validation: check for empty string or malformed structure
        if s.is_empty()
            || s.starts_with('-')
            || s.ends_with('-')
            || s.contains("--")
            || !s.contains('-')
        {
            return Err(Error::Syntax(s.to_string()));
        }

        let parts: Vec<&str> = s.split('-').collect();

        // Must have at least KIND-ID (2 parts)
        if parts.len() < 2 {
            return Err(Error::Syntax(s.to_string()));
        }

        // Parse ID from the last part
        let id_str = parts[parts.len() - 1];
        let id_usize = id_str
            .parse::<usize>()
            .map_err(|_| Error::Id(s.to_string(), id_str.to_string()))?;
        let id = NonZeroUsize::new(id_usize)
            .ok_or_else(|| Error::Id(s.to_string(), id_str.to_string()))?;

        // Parse KIND from the second-to-last part
        let kind_str = parts[parts.len() - 2];
        let kind = KindString::new(kind_str.to_string())?;

        // Parse namespace from all remaining parts
        let namespace = if parts.len() > 2 {
            parts[..parts.len() - 2]
                .iter()
                .map(|&segment| KindString::new(segment.to_string()))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        Ok(Self::new_with_namespace(namespace, kind, id))
    }
}

impl TryFrom<&str> for Hrid {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hrid_creation_no_namespace() {
        let kind = KindString::new("URS".to_string()).unwrap();
        let id = NonZeroUsize::new(42).unwrap();
        let hrid = Hrid::new(kind, id);
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "URS");
        assert_eq!(hrid.id().get(), 42);
    }

    #[test]
    fn hrid_creation_with_namespace() {
        let namespace = vec![
            KindString::new("COMPONENT".to_string()).unwrap(),
            KindString::new("SUBCOMPONENT".to_string()).unwrap(),
        ];
        let kind = KindString::new("SYS".to_string()).unwrap();
        let id = NonZeroUsize::new(5).unwrap();
        let hrid = Hrid::new_with_namespace(namespace, kind, id);

        assert_eq!(hrid.namespace(), vec!["COMPONENT", "SUBCOMPONENT"]);
        assert_eq!(hrid.kind(), "SYS");
        assert_eq!(hrid.id().get(), 5);
    }

    #[test]
    fn hrid_creation_empty_kind_fails() {
        assert!(KindString::new(String::new()).is_err());
    }

    #[test]
    fn hrid_creation_lowercase_kind_fails() {
        assert!(KindString::new("sys".to_string()).is_err());
    }

    #[test]
    fn hrid_creation_zero_id_fails() {
        assert!(NonZeroUsize::new(0).is_none());
    }

    use test_case::test_case;

    // Test digit width formatting - no namespace
    #[test_case(2, 1, "SYS-01"; "2 digits id 1")]
    #[test_case(2, 42, "SYS-42"; "2 digits id 42")]
    #[test_case(2, 99, "SYS-99"; "2 digits at boundary")]
    #[test_case(2, 100, "SYS-100"; "2 digits expansion")]
    #[test_case(3, 1, "SYS-001"; "3 digits id 1")]
    #[test_case(3, 42, "SYS-042"; "3 digits id 42")]
    #[test_case(3, 999, "SYS-999"; "3 digits at boundary")]
    #[test_case(3, 1000, "SYS-1000"; "3 digits expansion")]
    #[test_case(4, 1, "SYS-0001"; "4 digits id 1")]
    #[test_case(4, 9999, "SYS-9999"; "4 digits at boundary")]
    #[test_case(4, 10000, "SYS-10000"; "4 digits expansion")]
    #[test_case(5, 1, "SYS-00001"; "5 digits id 1")]
    #[test_case(5, 99999, "SYS-99999"; "5 digits at boundary")]
    fn hrid_display_no_namespace(digits: usize, id: usize, expected: &str) {
        let hrid = Hrid::new(
            KindString::new("SYS".to_string()).unwrap(),
            NonZeroUsize::new(id).unwrap(),
        );
        assert_eq!(hrid.display(digits).to_string(), expected);
    }

    // Test digit width formatting - with namespace
    #[test_case(2, 5, "COMPONENT-SYS-05"; "2 digits single namespace")]
    #[test_case(3, 5, "COMPONENT-SYS-005"; "3 digits single namespace")]
    #[test_case(4, 5, "COMPONENT-SYS-0005"; "4 digits single namespace")]
    #[test_case(3, 123, "A-B-C-REQ-123"; "3 digits multi namespace")]
    fn hrid_display_with_namespace(digits: usize, id: usize, expected: &str) {
        let namespace = if expected.starts_with("COMPONENT") {
            vec![KindString::new("COMPONENT".to_string()).unwrap()]
        } else {
            vec![
                KindString::new("A".to_string()).unwrap(),
                KindString::new("B".to_string()).unwrap(),
                KindString::new("C".to_string()).unwrap(),
            ]
        };

        let kind = if expected.contains("SYS") {
            "SYS"
        } else {
            "REQ"
        };

        let hrid = Hrid::new_with_namespace(
            namespace,
            KindString::new(kind.to_string()).unwrap(),
            NonZeroUsize::new(id).unwrap(),
        );
        assert_eq!(hrid.display(digits).to_string(), expected);
    }

    // Test large number expansion
    #[test_case(3, 1000, "BIG-1000"; "3 digits to 4")]
    #[test_case(3, 12345, "BIG-12345"; "3 digits to 5")]
    #[test_case(4, 10000, "BIG-10000"; "4 digits to 5")]
    #[test_case(4, 100_000, "BIG-100000"; "4 digits to 6")]
    fn hrid_display_large_numbers(digits: usize, id: usize, expected: &str) {
        let hrid = Hrid::new(
            KindString::new("BIG".to_string()).unwrap(),
            NonZeroUsize::new(id).unwrap(),
        );
        assert_eq!(hrid.display(digits).to_string(), expected);
    }

    #[test]
    fn try_from_valid_no_namespace() {
        let hrid = Hrid::try_from("URS-001").unwrap();
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "URS");
        assert_eq!(hrid.id().get(), 1);

        let hrid = Hrid::try_from("SYS-042").unwrap();
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "SYS");
        assert_eq!(hrid.id().get(), 42);

        let hrid = Hrid::try_from("TEST-999").unwrap();
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "TEST");
        assert_eq!(hrid.id().get(), 999);
    }

    #[test]
    fn try_from_valid_with_namespace() {
        let hrid = Hrid::try_from("COMPONENT-SYS-005").unwrap();
        assert_eq!(hrid.namespace(), vec!["COMPONENT"]);
        assert_eq!(hrid.kind(), "SYS");
        assert_eq!(hrid.id().get(), 5);

        let hrid = Hrid::try_from("COMPONENT-SUBCOMPONENT-SYS-005").unwrap();
        assert_eq!(hrid.namespace(), vec!["COMPONENT", "SUBCOMPONENT"]);
        assert_eq!(hrid.kind(), "SYS");
        assert_eq!(hrid.id().get(), 5);

        let hrid = Hrid::try_from("A-B-C-REQ-123").unwrap();
        assert_eq!(hrid.namespace(), vec!["A", "B", "C"]);
        assert_eq!(hrid.kind(), "REQ");
        assert_eq!(hrid.id().get(), 123);
    }

    #[test]
    fn try_from_valid_no_leading_zeros() {
        let hrid = Hrid::try_from("URS-1").unwrap();
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "URS");
        assert_eq!(hrid.id().get(), 1);

        let hrid = Hrid::try_from("NS-SYS-42").unwrap();
        assert_eq!(hrid.namespace(), vec!["NS"]);
        assert_eq!(hrid.kind(), "SYS");
        assert_eq!(hrid.id().get(), 42);
    }

    #[test]
    fn try_from_valid_large_numbers() {
        let hrid = Hrid::try_from("BIG-1000").unwrap();
        assert!(hrid.namespace().is_empty());
        assert_eq!(hrid.kind(), "BIG");
        assert_eq!(hrid.id().get(), 1000);

        let hrid = Hrid::try_from("NS-HUGE-12345").unwrap();
        assert_eq!(hrid.namespace(), vec!["NS"]);
        assert_eq!(hrid.kind(), "HUGE");
        assert_eq!(hrid.id().get(), 12345);
    }

    #[test]
    fn try_from_invalid_no_dash() {
        let result = Hrid::try_from("URS001");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_invalid_empty_string() {
        let result = Hrid::try_from("");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_invalid_only_dash() {
        let result = Hrid::try_from("-");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_invalid_single_part() {
        let result = Hrid::try_from("JUSTONEWORD");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_invalid_non_numeric_id() {
        let result = Hrid::try_from("URS-abc");
        assert!(matches!(result, Err(Error::Id(_, _))));

        let result = Hrid::try_from("NS-URS-abc");
        assert!(matches!(result, Err(Error::Id(_, _))));
    }

    #[test]
    fn try_from_invalid_mixed_id() {
        let result = Hrid::try_from("SYS-12abc");
        assert!(matches!(result, Err(Error::Id(_, _))));
    }

    #[test]
    fn try_from_invalid_negative_id() {
        let result = Hrid::try_from("URS--1");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_invalid_zero_id() {
        let result = Hrid::try_from("URS-0");
        assert!(matches!(result, Err(Error::Id(_, _))));
    }

    #[test]
    fn try_from_invalid_lowercase_kind() {
        let result = Hrid::try_from("urs-001");
        assert!(matches!(result, Err(Error::Kind(_))));
    }

    #[test]
    fn try_from_invalid_lowercase_namespace() {
        let result = Hrid::try_from("ns-URS-001");
        assert!(matches!(result, Err(Error::Kind(_))));
    }

    #[test]
    fn try_from_empty_namespace_segment_fails() {
        let result = Hrid::try_from("-NS-SYS-001");
        assert!(matches!(result, Err(Error::Syntax(_))));

        let result = Hrid::try_from("NS--SYS-001");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn try_from_empty_kind_fails() {
        let result = Hrid::try_from("-001");
        assert!(matches!(result, Err(Error::Syntax(_))));
    }

    #[test]
    fn hrid_clone_and_eq() {
        let hrid1 = Hrid::new_with_namespace(
            vec![KindString::new("NS".to_string()).unwrap()],
            KindString::new("URS".to_string()).unwrap(),
            NonZeroUsize::new(42).unwrap(),
        );
        let hrid2 = hrid1.clone();

        assert_eq!(hrid1, hrid2);
        assert_eq!(hrid1.namespace(), hrid2.namespace());
        assert_eq!(hrid1.kind(), hrid2.kind());
        assert_eq!(hrid1.id(), hrid2.id());
    }

    #[test]
    fn hrid_not_eq() {
        let hrid1 = Hrid::new(
            KindString::new("URS".to_string()).unwrap(),
            NonZeroUsize::new(42).unwrap(),
        );
        let hrid2 = Hrid::new(
            KindString::new("SYS".to_string()).unwrap(),
            NonZeroUsize::new(42).unwrap(),
        );
        let hrid3 = Hrid::new(
            KindString::new("URS".to_string()).unwrap(),
            NonZeroUsize::new(43).unwrap(),
        );
        let hrid4 = Hrid::new_with_namespace(
            vec![KindString::new("NS".to_string()).unwrap()],
            KindString::new("URS".to_string()).unwrap(),
            NonZeroUsize::new(42).unwrap(),
        );

        assert_ne!(hrid1, hrid2);
        assert_ne!(hrid1, hrid3);
        assert_ne!(hrid1, hrid4);
    }

    #[test]
    fn roundtrip_conversion_no_namespace() {
        let original = Hrid::new(
            KindString::new("TEST".to_string()).unwrap(),
            NonZeroUsize::new(123).unwrap(),
        );

        let as_string = original.display(3).to_string();
        let parsed = Hrid::try_from(as_string.as_str()).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn roundtrip_conversion_with_namespace() {
        let original = Hrid::new_with_namespace(
            vec![
                KindString::new("COMPONENT".to_string()).unwrap(),
                KindString::new("SUBCOMPONENT".to_string()).unwrap(),
            ],
            KindString::new("SYS".to_string()).unwrap(),
            NonZeroUsize::new(5).unwrap(),
        );

        let as_string = original.display(3).to_string();
        let parsed = Hrid::try_from(as_string.as_str()).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn strict_uppercase_validation() {
        // Domain layer is strict - lowercase should fail
        assert!(KindString::new("sys".to_string()).is_err());

        // FromStr is also strict
        let result = Hrid::from_str("component-sys-001");
        assert!(matches!(result, Err(Error::Kind(_))));
    }

    #[test]
    fn error_display() {
        let syntax_error = Error::Syntax("bad-format".to_string());
        assert_eq!(format!("{syntax_error}"), "Invalid HRID format: bad-format");

        let id_error = Error::Id("URS-bad".to_string(), "bad".to_string());
        assert_eq!(
            format!("{id_error}"),
            "Invalid ID in HRID 'URS-bad': expected a non-zero integer, got bad"
        );
    }
}
