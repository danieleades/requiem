//! Parsing of the markdown body of a requirement file.

use std::io;

use super::LoadError;
use crate::domain::Hrid;

/// Trim empty lines from the start and end of a string, preserving indentation.
///
/// Unlike `.trim()`, this function only removes completely empty lines from
/// the beginning and end, keeping any leading/trailing whitespace on non-empty
/// lines. This is crucial for preserving markdown structures like code blocks,
/// lists, and blockquotes which rely on indentation.
pub fn trim_empty_lines(s: &str) -> String {
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
pub(super) fn parse_content(content: &str) -> Result<(Hrid, String, String), LoadError> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
