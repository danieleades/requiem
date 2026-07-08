//! Filtering of list entries.

use anyhow::Context;
use regex::Regex;

use super::{entry::Entry, List};

/// Compiled filters derived from the command-line arguments.
#[derive(Debug, Clone)]
pub(super) struct Filters {
    pub(super) kinds: Vec<String>,
    pub(super) namespaces: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) orphans: bool,
    pub(super) leaves: bool,
    pub(super) contains: Option<String>,
    pub(super) regex: Option<Regex>,
}

impl Filters {
    pub(super) fn new(cmd: &List) -> anyhow::Result<Self> {
        let regex = if let Some(pattern) = &cmd.regex {
            Some(Regex::new(pattern).with_context(|| format!("invalid regex: {pattern}"))?)
        } else {
            None
        };

        Ok(Self {
            kinds: cmd
                .kind
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            namespaces: cmd
                .namespace
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            tags: cmd
                .tag
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
            orphans: cmd.orphans,
            leaves: cmd.leaves,
            contains: cmd.contains.as_deref().map(str::to_ascii_lowercase),
            regex,
        })
    }

    pub(super) fn any(&self) -> bool {
        !self.kinds.is_empty()
            || !self.namespaces.is_empty()
            || !self.tags.is_empty()
            || self.orphans
            || self.leaves
            || self.contains.is_some()
            || self.regex.is_some()
    }

    /// Human-readable summary of the active filters for the listing header.
    pub(super) fn describe(&self) -> String {
        if !self.any() {
            return "none".to_string();
        }

        let mut parts = Vec::new();
        if !self.kinds.is_empty() {
            parts.push(format!("kind: {}", self.kinds.join(", ")));
        }
        if !self.namespaces.is_empty() {
            parts.push(format!("namespace: {}", self.namespaces.join(", ")));
        }
        if !self.tags.is_empty() {
            parts.push(format!("tag: {}", self.tags.join(", ")));
        }
        if self.orphans {
            parts.push("orphans".to_string());
        }
        if self.leaves {
            parts.push("leaves".to_string());
        }
        if self.contains.is_some() {
            parts.push("text-match".to_string());
        }
        if self.regex.is_some() {
            parts.push("regex-match".to_string());
        }
        parts.join(", ")
    }

    pub(super) fn matches(&self, entry: &Entry) -> bool {
        if !self.kinds.is_empty() {
            let kind = entry.hrid.kind().to_ascii_lowercase();
            if !self.kinds.iter().any(|k| k == &kind) {
                return false;
            }
        }

        if !self.namespaces.is_empty() {
            let namespace: Vec<String> = entry
                .hrid
                .namespace()
                .into_iter()
                .map(str::to_ascii_lowercase)
                .collect();
            if !self
                .namespaces
                .iter()
                .any(|ns| namespace.iter().any(|segment| segment == ns))
            {
                return false;
            }
        }

        if !self.tags.is_empty() {
            let tag_set: Vec<String> = entry
                .tags
                .iter()
                .map(String::as_str)
                .map(str::to_ascii_lowercase)
                .collect();
            if !self
                .tags
                .iter()
                .any(|tag| tag_set.iter().any(|entry_tag| entry_tag == tag))
            {
                return false;
            }
        }

        if self.orphans && !entry.parents.is_empty() {
            return false;
        }

        if self.leaves && !entry.children.is_empty() {
            return false;
        }

        if let Some(search) = &self.contains {
            let title = entry
                .title
                .as_deref()
                .map_or_else(String::new, str::to_ascii_lowercase);
            if title.contains(search) {
                // already matched
            } else if !entry.content.to_ascii_lowercase().contains(search) {
                return false;
            }
        }

        if let Some(regex) = &self.regex {
            let haystack = entry.title.as_deref().map_or_else(
                || entry.content.clone(),
                |title| format!("{title}\n{}", entry.content),
            );
            if !regex.is_match(&haystack) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::{super::fixtures::SampleEntries, Filters};

    #[test]
    fn filters_match_combined_conditions() {
        let fixtures = SampleEntries::new();
        let leaf = fixtures.leaf();

        let build_filters = || Filters {
            kinds: vec!["usr".to_string()],
            namespaces: vec!["auth".to_string()],
            tags: vec!["security".to_string()],
            orphans: false,
            leaves: false,
            contains: Some("login".to_string()),
            regex: Some(Regex::new(r"(?i)login").unwrap()),
        };

        let filters = build_filters();
        assert!(filters.matches(leaf));

        let mut filters_with_orphans = build_filters();
        filters_with_orphans.orphans = true;
        assert!(!filters_with_orphans.matches(leaf));

        let mut filters_with_leaves = build_filters();
        filters_with_leaves.leaves = true;
        assert!(filters_with_leaves.matches(leaf));

        let mut filters_with_tags = build_filters();
        filters_with_tags.tags = vec!["missing".to_string()];
        assert!(!filters_with_tags.matches(leaf));
    }
}
