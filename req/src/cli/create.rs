use std::path::PathBuf;

use requiem_core::{Directory, Hrid};
use tracing::instrument;

use crate::cli::parse_hrid;

#[derive(Debug, clap::Parser)]
pub struct Command {
    /// The kind of requirement to create, optionally with namespace.
    ///
    /// Accepts a dash-separated list where the last token is the kind
    /// and any preceding tokens form the namespace.
    ///
    /// Examples:
    /// - 'USR' creates a requirement with kind USR and no namespace
    /// - 'SYSTEM-AUTH-USR' creates a requirement with namespace [SYSTEM, AUTH]
    ///   and kind USR
    kind: String,

    /// The human-readable IDs of the parent requirements.
    #[clap(long, short, value_delimiter = ',', value_parser = parse_hrid)]
    parent: Vec<Hrid>,

    /// The title of the requirement (will be formatted as a markdown heading).
    #[clap(long, short)]
    title: Option<String>,

    /// The body text of the requirement.
    #[clap(long, short)]
    body: Option<String>,
}

impl Command {
    #[instrument]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let mut directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Construct content from title and body
        let content = match (&self.title, &self.body) {
            (Some(title), Some(body)) => format!("# {title}\n\n{body}"),
            (Some(title), None) => format!("# {title}"),
            (None, Some(body)) => body.clone(),
            (None, None) => String::new(),
        };

        // Parse kind string as dash-separated tokens (CLI boundary - normalize to
        // uppercase)
        let tokens: nonempty::NonEmpty<String> = {
            let parts: Vec<String> = self
                .kind
                .split('-')
                .map(|s| s.trim().to_uppercase())
                .collect();
            nonempty::NonEmpty::from_vec(parts)
                .ok_or_else(|| anyhow::anyhow!("kind must contain at least one token"))?
        };

        // Last token is the kind, everything before is the namespace
        let (namespace, kind) = {
            let mut parts: Vec<String> = tokens.into();
            let kind = parts.pop().expect("nonempty has at least one element");
            (parts, kind)
        };

        let requirement =
            directory.add_requirement_with_parents(namespace, &kind, content, self.parent)?;
        directory.flush()?;

        println!("Added requirement {}", requirement.hrid().display(digits));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use requiem_core::{Directory, RequirementView};
    use tempfile::tempdir;

    use super::Command;

    fn find_with_kind<'a>(directory: &'a Directory, kind: &'a str) -> RequirementView<'a> {
        directory
            .requirements()
            .find(|req| req.hrid.kind() == kind)
            .expect("expected requirement for kind")
    }

    #[test]
    fn create_run_creates_namespaced_requirement() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let create = Command {
            kind: "SYSTEM-AUTH-USR".to_string(),
            parent: Vec::new(),
            title: Some("Namespaced Requirement".to_string()),
            body: Some("test body".to_string()),
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");

        // Find the requirement by kind USR with namespace SYSTEM-AUTH
        let requirements: Vec<_> = directory
            .requirements()
            .filter(|r| r.hrid.kind() == "USR" && r.hrid.namespace() == ["SYSTEM", "AUTH"])
            .map(|view| view.to_requirement())
            .collect();

        assert_eq!(requirements.len(), 1);
        let req = &requirements[0];

        // Verify namespace and kind
        assert_eq!(req.hrid().namespace(), &["SYSTEM", "AUTH"]);
        assert_eq!(req.hrid().kind(), "USR");
        assert_eq!(req.title(), "Namespaced Requirement");
    }

    #[test]
    fn create_run_uses_template_when_no_content_provided() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let template_dir = root.join(".req").join("templates");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("USR.md"), "## Template body").unwrap();

        let create = Command {
            kind: "USR".to_string(),
            parent: Vec::new(),
            title: None,
            body: None,
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let child = find_with_kind(&directory, "USR");
        assert_eq!(child.body, "## Template body");
    }

    #[test]
    fn create_run_creates_requirement_and_links_parents() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let mut directory = Directory::new(root.clone()).expect("failed to load directory");
        let parent = directory
            .add_requirement("SYS", "# Parent".to_string())
            .unwrap();
        directory
            .flush()
            .expect("failed to flush parent requirement");

        let create = Command {
            kind: "USR".to_string(),
            parent: vec![parent.hrid().clone()],
            title: Some("Child".to_string()),
            body: Some("body text".to_string()),
        };

        create
            .run(root.clone())
            .expect("create command should succeed");

        let directory = Directory::new(root).expect("failed to load directory");
        let child = find_with_kind(&directory, "USR");

        assert!(child
            .parents
            .iter()
            .any(|(_uuid, info)| info.hrid == *parent.hrid()));
        assert_eq!(child.title, "Child");
        assert_eq!(child.body, "body text");
    }
}
