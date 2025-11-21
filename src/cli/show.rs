use std::{path::PathBuf, process};

use clap::Parser;
use requiem::{Directory, Hrid};
use tracing::instrument;

use super::terminal::Colorize;

#[derive(Debug, Parser)]
#[command(about = "Display detailed information about a requirement")]
pub struct Show {
    /// The human-readable ID of the requirement to display
    #[clap(value_parser = super::parse_hrid)]
    hrid: Hrid,

    /// Output format
    #[arg(long, value_name = "FORMAT", default_value = "pretty")]
    output: OutputFormat,

    /// Include full markdown content in output
    #[arg(long)]
    with_content: bool,

    /// Open requirement in EDITOR
    #[arg(long)]
    edit: bool,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Markdown,
    Raw,
}

impl Show {
    #[instrument(level = "debug", skip(self))]
    pub fn run(self, root: PathBuf) -> anyhow::Result<()> {
        let directory = Directory::new(root)?;
        let digits = directory.config().digits();

        // Find the requirement
        let Some(req) = directory.find_by_hrid(&self.hrid) else {
            eprintln!(
                "Requirement {} not found",
                self.hrid.display(digits)
            );
            std::process::exit(1);
        };

        // Handle --edit flag
        if self.edit {
            return self.edit_requirement(&directory, &self.hrid);
        }

        // Display based on output format
        match self.output {
            OutputFormat::Pretty => self.output_pretty(&directory, req, digits),
            OutputFormat::Json => self.output_json(&directory, req, digits)?,
            OutputFormat::Markdown => self.output_markdown(&directory, req),
            OutputFormat::Raw => self.output_raw(&directory, &self.hrid)?,
        }

        Ok(())
    }

    fn output_pretty(&self, directory: &Directory, req: requiem::RequirementView, digits: usize) {
        // Header
        println!("# {}", req.hrid.display(digits));
        println!("{}\n", req.title);

        // Metadata
        println!("{}", "Metadata".dim());
        println!("  Kind:      {}", req.hrid.kind());
        let ns = req.hrid.namespace();
        if !ns.is_empty() {
            println!("  Namespace: {}", ns.join("-"));
        }
        println!("  UUID:      {}", req.uuid);
        println!("  Created:   {}", req.created);

        // File path
        if let Some(path) = directory.path_for(req.hrid) {
            println!("  Path:      {}", path.display());
        }

        // Tags
        if !req.tags.is_empty() {
            println!("\n{}", "Tags".dim());
            for tag in req.tags {
                println!("  • {}", tag);
            }
        }

        // Parents
        if !req.parents.is_empty() {
            println!("\n{}", "Parents".dim());
            let suspect_links = directory.suspect_links();
            for (parent_uuid, parent_info) in &req.parents {
                // Check if this link is suspect
                let is_suspect = suspect_links.iter().any(|link| {
                    link.child_hrid == *req.hrid && link.parent_hrid == parent_info.hrid
                });

                let indicator = if is_suspect { " ⚠️" } else { "" };
                println!(
                    "  • {} ({}){indicator}",
                    parent_info.hrid.display(digits),
                    parent_uuid
                );
            }
        }

        // Children
        if !req.children.is_empty() {
            println!("\n{}", "Children".dim());
            for child_uuid in &req.children {
                // Find child by UUID
                if let Some(child) = directory.requirements().find(|r| r.uuid == child_uuid) {
                    println!("  • {} ({})", child.hrid.display(digits), child_uuid);
                }
            }
        }

        // Content
        if self.with_content && !req.body.is_empty() {
            println!("\n{}", "Content".dim());
            println!("{}", req.body);
        }
    }

    fn output_json(
        &self,
        directory: &Directory,
        req: requiem::RequirementView,
        digits: usize,
    ) -> anyhow::Result<()> {
        use serde_json::json;

        let parents: Vec<_> = req
            .parents
            .iter()
            .map(|(uuid, info)| {
                json!({
                    "uuid": uuid.to_string(),
                    "hrid": info.hrid.display(digits).to_string(),
                    "fingerprint": info.fingerprint
                })
            })
            .collect();

        let children: Vec<_> = req
            .children
            .iter()
            .map(|uuid| {
                let hrid = directory
                    .requirements()
                    .find(|r| r.uuid == uuid)
                    .map(|r| r.hrid.display(digits).to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                json!({
                    "uuid": uuid.to_string(),
                    "hrid": hrid
                })
            })
            .collect();

        let path = directory
            .path_for(req.hrid)
            .map(|p| p.to_string_lossy().to_string());

        let ns = req.hrid.namespace();
        let mut output = json!({
            "hrid": req.hrid.display(digits).to_string(),
            "kind": req.hrid.kind(),
            "uuid": req.uuid.to_string(),
            "created": req.created.to_rfc3339(),
            "title": req.title,
            "tags": req.tags,
            "parents": parents,
            "children": children,
            "path": path,
        });

        if !ns.is_empty() {
            output["namespace"] = json!(ns);
        }

        if self.with_content {
            output["body"] = json!(req.body);
        }

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn output_markdown(&self, directory: &Directory, req: requiem::RequirementView) {
        // Output as markdown that could be used in documentation
        println!("# {} {}\n", req.hrid.display(directory.config().digits()), req.title);

        // Metadata table
        println!("| Property | Value |");
        println!("| --- | --- |");
        println!("| Kind | {} |", req.hrid.kind());
        let ns = req.hrid.namespace();
        if !ns.is_empty() {
            println!("| Namespace | {} |", ns.join("-"));
        }
        println!("| UUID | `{}` |", req.uuid);
        println!("| Created | {} |", req.created);

        if !req.parents.is_empty() {
            println!("\n## Parents\n");
            for (_uuid, info) in &req.parents {
                println!(
                    "- [{}]({})",
                    info.hrid.display(directory.config().digits()),
                    format!("{}.md", info.hrid.display(directory.config().digits()))
                );
            }
        }

        if !req.children.is_empty() {
            println!("\n## Children\n");
            for child_uuid in &req.children {
                if let Some(child) = directory.requirements().find(|r| r.uuid == child_uuid) {
                    println!(
                        "- [{}]({})",
                        child.hrid.display(directory.config().digits()),
                        format!("{}.md", child.hrid.display(directory.config().digits()))
                    );
                }
            }
        }

        if self.with_content && !req.body.is_empty() {
            println!("\n## Content\n");
            println!("{}", req.body);
        }
    }

    fn output_raw(&self, directory: &Directory, hrid: &Hrid) -> anyhow::Result<()> {
        // Output the raw markdown file content
        let Some(path) = directory.path_for(hrid) else {
            anyhow::bail!("Path not found for requirement");
        };

        let content = std::fs::read_to_string(path)?;
        print!("{}", content);
        Ok(())
    }

    fn edit_requirement(&self, directory: &Directory, hrid: &Hrid) -> anyhow::Result<()> {
        let Some(path) = directory.path_for(hrid) else {
            anyhow::bail!("Path not found for requirement");
        };

        // Get editor from environment
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        // Launch editor
        let status = process::Command::new(&editor)
            .arg(path)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to launch editor '{}': {}", editor, e))?;

        if !status.success() {
            anyhow::bail!("Editor exited with non-zero status");
        }

        Ok(())
    }
}
