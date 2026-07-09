//! Editing tools: create, update, link, delete, and review requirements.

use std::{collections::BTreeSet, fs};

use requiem_core::{Directory, Hrid, LinkRequirementError};
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, ErrorData as McpError};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::discovery::RequirementDetails;
use crate::server::ReqMcpServer;

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequirementKindParams {
    /// New kind identifier, e.g. "USR".
    pub kind: String,
    /// Optional human-readable description of the kind.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequirementKindResponse {
    /// Kind identifier that was added or updated.
    pub kind: String,
    /// Optional description that was applied.
    #[serde(default)]
    pub description: Option<String>,
    /// Whether the kind was newly added (false if it already existed).
    pub added: bool,
    /// Updated list of allowed kinds.
    pub allowed_kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequirementParams {
    /// Optional namespace segments (upper-case) preceding the kind.
    #[serde(default)]
    pub namespace: Vec<String>,
    /// Requirement kind, e.g. "USR".
    pub kind: String,
    /// Title for the new requirement.
    pub title: String,
    /// Markdown body for the new requirement.
    pub body: String,
    /// Optional parent HRIDs to link to.
    #[serde(default)]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequirementParams {
    /// HRID of the requirement to update.
    pub hrid: String,
    /// New title (omit to leave unchanged).
    #[serde(default)]
    pub title: Option<String>,
    /// New markdown body (omit to leave unchanged).
    #[serde(default)]
    pub body: Option<String>,
    /// Replacement set of tags (omit to leave unchanged).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequirementResponse {
    /// Whether any field actually changed.
    pub changed: bool,
    /// The requirement after the update.
    pub requirement: RequirementDetails,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequirementParams {
    /// Child HRID.
    pub child: String,
    /// Parent HRID to link the child to.
    pub parent: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequirementResponse {
    /// Child HRID.
    pub child: String,
    /// Parent HRID.
    pub parent: String,
    /// Whether the link already existed. Relinking an existing pair refreshes
    /// the stored parent fingerprint (equivalent to accepting a suspect link).
    pub already_linked: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnlinkRequirementParams {
    /// Child HRID.
    pub child: String,
    /// Parent HRID to unlink the child from.
    pub parent: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnlinkRequirementResponse {
    /// Child HRID.
    pub child: String,
    /// Parent HRID the child was unlinked from.
    pub parent: String,
}

/// How `delete_requirement` handles children of the deleted requirement.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum DeleteMode {
    /// Fail if the requirement has children (default).
    #[default]
    Refuse,
    /// Delete the requirement and unlink it from its children; the children
    /// are kept.
    Orphan,
    /// Delete the requirement together with all descendants that would be
    /// left without any parent.
    Cascade,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequirementParams {
    /// HRID of the requirement to delete.
    pub hrid: String,
    /// How to handle children: "refuse" (default), "orphan", or "cascade".
    #[serde(default)]
    pub mode: DeleteMode,
    /// Preview what would be deleted without changing anything.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequirementResponse {
    /// Mode that was applied.
    pub mode: DeleteMode,
    /// Whether this was a dry run (nothing was changed).
    pub dry_run: bool,
    /// HRIDs deleted (or that would be deleted for a dry run).
    pub deleted: Vec<String>,
    /// Children that were unlinked from a deleted requirement but kept.
    pub unlinked_children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequirementParams {
    /// Child HRID whose link is being reviewed.
    pub child: String,
    /// Parent HRID the child references.
    pub parent: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequirementResponse {
    /// Child HRID reviewed.
    pub child: String,
    /// Parent HRID reviewed.
    pub parent: String,
    /// Outcome of the review.
    pub status: String,
}

/// Build the full details view for a requirement that is known to exist.
fn requirement_details(
    directory: &Directory,
    hrid: &requiem_core::Hrid,
    digits: usize,
) -> Result<RequirementDetails, McpError> {
    let Some(view) = directory.find_by_hrid(hrid) else {
        return Err(McpError::internal_error(
            "requirement could not be reloaded",
            Some(json!({ "hrid": ReqMcpServer::format_hrid(hrid, digits) })),
        ));
    };

    let parents = view
        .parents
        .iter()
        .map(|(_, parent)| ReqMcpServer::format_hrid(&parent.hrid, digits))
        .collect();

    let children = directory
        .children_of(hrid)
        .iter()
        .map(|child| ReqMcpServer::format_hrid(child, digits))
        .collect();

    Ok(RequirementDetails {
        hrid: ReqMcpServer::format_hrid(hrid, digits),
        title: view.title.to_string(),
        body: view.body.to_string(),
        tags: view.tags.iter().cloned().collect(),
        parents,
        children,
    })
}

/// Persist pending changes, mapping failures to an MCP error.
fn flush(directory: &mut Directory) -> Result<(), McpError> {
    directory.flush().map(|_| ()).map_err(|error| {
        McpError::internal_error(
            "failed to persist changes",
            Some(json!({ "reason": error.to_string() })),
        )
    })
}

pub(super) async fn create_requirement_kind(
    server: &ReqMcpServer,
    params: Parameters<CreateRequirementKindParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;

    let raw_kind = params.kind.trim();
    if raw_kind.is_empty() {
        return Err(McpError::invalid_params(
            "`kind` is required",
            Some(json!({ "field": "kind" })),
        ));
    }

    let kind = raw_kind.to_uppercase();
    let description_input = params.description.as_ref().map(|d| d.trim());

    let mut directory = server.state.directory.write().await;
    let root = directory.root().to_path_buf();
    let mut config = directory.config().clone();

    let added = config.add_kind(&kind);
    match description_input {
        Some("") => config.set_kind_description(&kind, None),
        Some(desc) => config.set_kind_description(&kind, Some(desc.to_string())),
        None => {}
    }

    let config_dir = root.join(".req");
    fs::create_dir_all(&config_dir).map_err(|error| {
        McpError::internal_error(
            "failed to ensure .req directory",
            Some(json!({ "path": config_dir, "reason": error.to_string() })),
        )
    })?;

    let config_path = config_dir.join("config.toml");
    config.save(&config_path).map_err(|reason| {
        McpError::internal_error(
            "failed to write config.toml",
            Some(json!({ "path": config_path, "reason": reason })),
        )
    })?;

    let reloaded = Directory::new(root).map_err(|error| {
        McpError::internal_error(
            "failed to reload requirements after config update",
            Some(json!({ "reason": error.to_string() })),
        )
    })?;
    *directory = reloaded;

    let description = directory
        .config()
        .metadata_for_kind(&kind)
        .and_then(|meta| meta.description.clone());

    let response = CreateRequirementKindResponse {
        kind: kind.clone(),
        description,
        added,
        allowed_kinds: directory.config().allowed_kinds().to_vec(),
    };

    let summary = if added {
        format!("Kind {kind} added")
    } else {
        format!("Kind {kind} updated")
    };

    drop(directory);

    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "create_requirement_kind response")?,
    ))
}

#[allow(clippy::too_many_lines)]
pub(super) async fn create_requirement(
    server: &ReqMcpServer,
    params: Parameters<CreateRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;

    if params.kind.trim().is_empty() {
        return Err(McpError::invalid_params(
            "`kind` is required",
            Some(json!({ "field": "kind" })),
        ));
    }

    let namespace: Vec<String> = params
        .namespace
        .into_iter()
        .map(|segment| segment.trim().to_string())
        .filter(|segment| !segment.is_empty())
        .collect();
    let kind = params.kind.trim().to_uppercase();

    let content = match (
        params.title.trim().is_empty(),
        params.body.trim().is_empty(),
    ) {
        (false, false) => format!("# {}\n\n{}", params.title.trim(), params.body),
        (false, true) => format!("# {}", params.title.trim()),
        (true, false) => params.body,
        (true, true) => String::new(),
    };

    let parent_hrids: Vec<Hrid> = params
        .parents
        .iter()
        .map(|parent| ReqMcpServer::parse_hrid(parent))
        .collect::<Result<_, _>>()?;

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    // Validate all parent HRIDs before creating anything to avoid partial inserts.
    for parent in &parent_hrids {
        if directory.find_by_hrid(parent).is_none() {
            return Err(McpError::resource_not_found(
                "parent requirement not found",
                Some(json!({ "parent": ReqMcpServer::format_hrid(parent, digits) })),
            ));
        }
    }

    let requirement = if namespace.is_empty() {
        directory
            .add_requirement(&kind, content)
            .map_err(|error| match error {
                requiem_core::storage::directory::AddRequirementError::Hrid(reason) => {
                    McpError::invalid_params(
                        "invalid kind or namespace",
                        Some(json!({ "kind": kind, "reason": reason.to_string() })),
                    )
                }
                requiem_core::storage::directory::AddRequirementError::Duplicate(reason) => {
                    McpError::invalid_params(
                        "requirement already exists",
                        Some(json!({ "reason": reason.to_string() })),
                    )
                }
                requiem_core::storage::directory::AddRequirementError::DisallowedKind {
                    kind,
                    allowed_kinds,
                } => McpError::invalid_params(
                    "kind is not allowed by configuration",
                    Some(json!({ "kind": kind, "allowed_kinds": allowed_kinds })),
                ),
            })?
    } else {
        directory
            .add_requirement_with_namespace(namespace, &kind, content)
            .map_err(|error| match error {
                requiem_core::storage::directory::AddRequirementError::Hrid(reason) => {
                    McpError::invalid_params(
                        "invalid kind or namespace",
                        Some(json!({ "kind": kind, "reason": reason.to_string() })),
                    )
                }
                requiem_core::storage::directory::AddRequirementError::Duplicate(reason) => {
                    McpError::invalid_params(
                        "requirement already exists",
                        Some(json!({ "reason": reason.to_string() })),
                    )
                }
                requiem_core::storage::directory::AddRequirementError::DisallowedKind {
                    kind,
                    allowed_kinds,
                } => McpError::invalid_params(
                    "kind is not allowed by configuration",
                    Some(json!({ "kind": kind, "allowed_kinds": allowed_kinds })),
                ),
            })?
    };

    for parent in parent_hrids {
        directory
            .link_requirement(requirement.hrid(), &parent)
            .map_err(|error| match error {
                LinkRequirementError::ParentNotFound(_) => McpError::resource_not_found(
                    "parent requirement not found",
                    Some(json!({ "parent": ReqMcpServer::format_hrid(&parent, digits) })),
                ),
                LinkRequirementError::ChildNotFound(_) => McpError::internal_error(
                    "child requirement missing after creation",
                    Some(json!({ "child": ReqMcpServer::format_hrid(requirement.hrid(), digits) })),
                ),
                LinkRequirementError::WouldCreateCycle(message) => McpError::invalid_params(
                    "cannot create link: would form a cycle",
                    Some(json!({ "reason": message })),
                ),
            })?;
    }

    flush(&mut directory)?;

    let hrid = requirement.hrid().clone();
    let response = requirement_details(&directory, &hrid, digits)?;

    drop(directory);

    let summary = format!("Created requirement {}", response.hrid);
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "create_requirement response")?,
    ))
}

pub(super) async fn update_requirement(
    server: &ReqMcpServer,
    params: Parameters<UpdateRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;

    if params.title.is_none() && params.body.is_none() && params.tags.is_none() {
        return Err(McpError::invalid_params(
            "at least one of `title`, `body`, or `tags` is required",
            Some(json!({ "fields": ["title", "body", "tags"] })),
        ));
    }

    let hrid = ReqMcpServer::parse_hrid(&params.hrid)?;
    let tags: Option<BTreeSet<String>> = params
        .tags
        .map(|tags| tags.into_iter().map(|tag| tag.trim().to_string()).collect());

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    if directory.find_by_hrid(&hrid).is_none() {
        return Err(McpError::resource_not_found(
            "requirement not found",
            Some(json!({ "hrid": params.hrid })),
        ));
    }

    let changed = directory
        .update_requirement(&hrid, params.title, params.body, tags)
        .map_err(|error| {
            McpError::internal_error(
                "failed to update requirement",
                Some(json!({ "reason": error.to_string() })),
            )
        })?;

    if changed {
        flush(&mut directory)?;
    }

    let response = UpdateRequirementResponse {
        changed,
        requirement: requirement_details(&directory, &hrid, digits)?,
    };

    drop(directory);

    let summary = if changed {
        format!(
            "Updated requirement {}; links from its children may now be suspect (see review)",
            response.requirement.hrid
        )
    } else {
        format!("No changes applied to {}", response.requirement.hrid)
    };

    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "update_requirement response")?,
    ))
}

pub(super) async fn link_requirement(
    server: &ReqMcpServer,
    params: Parameters<LinkRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    let child = ReqMcpServer::parse_hrid(&params.child)?;
    let parent = ReqMcpServer::parse_hrid(&params.parent)?;

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    let already_linked = directory.children_of(&parent).contains(&child);

    directory
        .link_requirement(&child, &parent)
        .map_err(|error| match error {
            LinkRequirementError::ChildNotFound(_) => McpError::resource_not_found(
                "child requirement not found",
                Some(json!({ "child": ReqMcpServer::format_hrid(&child, digits) })),
            ),
            LinkRequirementError::ParentNotFound(_) => McpError::resource_not_found(
                "parent requirement not found",
                Some(json!({ "parent": ReqMcpServer::format_hrid(&parent, digits) })),
            ),
            LinkRequirementError::WouldCreateCycle(message) => McpError::invalid_params(
                "cannot create link: would form a cycle",
                Some(json!({ "reason": message })),
            ),
        })?;

    flush(&mut directory)?;

    let response = LinkRequirementResponse {
        child: ReqMcpServer::format_hrid(&child, digits),
        parent: ReqMcpServer::format_hrid(&parent, digits),
        already_linked,
    };

    drop(directory);

    let summary = if already_linked {
        format!(
            "{} was already linked to {}; refreshed the stored parent fingerprint",
            response.child, response.parent
        )
    } else {
        format!("Linked {} to parent {}", response.child, response.parent)
    };

    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "link_requirement response")?,
    ))
}

pub(super) async fn unlink_requirement(
    server: &ReqMcpServer,
    params: Parameters<UnlinkRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    let child = ReqMcpServer::parse_hrid(&params.child)?;
    let parent = ReqMcpServer::parse_hrid(&params.parent)?;

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    if directory.find_by_hrid(&child).is_none() {
        return Err(McpError::resource_not_found(
            "child requirement not found",
            Some(json!({ "child": ReqMcpServer::format_hrid(&child, digits) })),
        ));
    }
    if directory.find_by_hrid(&parent).is_none() {
        return Err(McpError::resource_not_found(
            "parent requirement not found",
            Some(json!({ "parent": ReqMcpServer::format_hrid(&parent, digits) })),
        ));
    }

    directory
        .unlink_requirement(&child, &parent)
        .map_err(|error| {
            McpError::invalid_params(
                "link between child and parent does not exist",
                Some(json!({
                    "child": ReqMcpServer::format_hrid(&child, digits),
                    "parent": ReqMcpServer::format_hrid(&parent, digits),
                    "reason": error.to_string()
                })),
            )
        })?;

    flush(&mut directory)?;

    let response = UnlinkRequirementResponse {
        child: ReqMcpServer::format_hrid(&child, digits),
        parent: ReqMcpServer::format_hrid(&parent, digits),
    };

    drop(directory);

    let summary = format!(
        "Unlinked {} from parent {}",
        response.child, response.parent
    );
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "unlink_requirement response")?,
    ))
}

pub(super) async fn delete_requirement(
    server: &ReqMcpServer,
    params: Parameters<DeleteRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    let hrid = ReqMcpServer::parse_hrid(&params.hrid)?;

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    if directory.find_by_hrid(&hrid).is_none() {
        return Err(McpError::resource_not_found(
            "requirement not found",
            Some(json!({ "hrid": params.hrid })),
        ));
    }

    let children = directory.children_of(&hrid);

    if params.mode == DeleteMode::Refuse && !children.is_empty() {
        return Err(McpError::invalid_params(
            "requirement has children; use mode `orphan` to keep them or `cascade` to also \
             delete orphaned descendants",
            Some(json!({
                "hrid": ReqMcpServer::format_hrid(&hrid, digits),
                "children": children
                    .iter()
                    .map(|child| ReqMcpServer::format_hrid(child, digits))
                    .collect::<Vec<_>>()
            })),
        ));
    }

    // What the delete will remove: just this requirement, or (for cascade)
    // this requirement plus every descendant left without a parent.
    let deleted = if params.mode == DeleteMode::Cascade {
        directory.find_orphaned_descendants(&hrid)
    } else {
        vec![hrid.clone()]
    };

    // Children that survive the delete but lose a parent link.
    let unlinked_children: Vec<String> = match params.mode {
        DeleteMode::Refuse => Vec::new(),
        DeleteMode::Orphan => children
            .iter()
            .map(|child| ReqMcpServer::format_hrid(child, digits))
            .collect(),
        DeleteMode::Cascade => {
            let deleted_set: BTreeSet<&requiem_core::Hrid> = deleted.iter().collect();
            let survivors: BTreeSet<String> = deleted
                .iter()
                .flat_map(|hrid| directory.children_of(hrid))
                .filter(|child| !deleted_set.contains(child))
                .map(|child| ReqMcpServer::format_hrid(&child, digits))
                .collect();
            survivors.into_iter().collect()
        }
    };

    if !params.dry_run {
        let result = match params.mode {
            DeleteMode::Refuse => directory.delete_requirement(&hrid),
            DeleteMode::Orphan => directory.delete_and_orphan(&hrid),
            // Deleting via `delete_and_orphan` is order-independent: edges to
            // already-removed members are gone, and surviving children are
            // unlinked as their deleted parents go.
            DeleteMode::Cascade => deleted
                .iter()
                .try_for_each(|hrid| directory.delete_and_orphan(hrid)),
        };
        result.map_err(|error| {
            McpError::internal_error(
                "failed to delete requirement",
                Some(json!({ "reason": error.to_string() })),
            )
        })?;

        flush(&mut directory)?;
    }

    let response = DeleteRequirementResponse {
        mode: params.mode,
        dry_run: params.dry_run,
        deleted: deleted
            .iter()
            .map(|hrid| ReqMcpServer::format_hrid(hrid, digits))
            .collect(),
        unlinked_children,
    };

    drop(directory);

    let summary = if params.dry_run {
        format!("Would delete {} requirement(s)", response.deleted.len())
    } else {
        format!("Deleted {} requirement(s)", response.deleted.len())
    };

    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "delete_requirement response")?,
    ))
}

#[allow(clippy::unused_async)]
pub(super) async fn review_requirement(
    server: &ReqMcpServer,
    params: Parameters<ReviewRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    let child = ReqMcpServer::parse_hrid(&params.child)?;
    let parent = ReqMcpServer::parse_hrid(&params.parent)?;

    let mut directory = server.state.directory.write().await;
    let digits = directory.config().digits();

    let result = directory
        .accept_suspect_link(child.clone(), parent.clone())
        .map_err(|error| match error {
            requiem_core::storage::directory::AcceptSuspectLinkError::ChildNotFound(hrid) => {
                McpError::resource_not_found(
                    "child requirement not found",
                    Some(json!({ "child": ReqMcpServer::format_hrid(&hrid, digits) })),
                )
            }
            requiem_core::storage::directory::AcceptSuspectLinkError::ParentNotFound(_) => {
                McpError::resource_not_found(
                    "parent requirement not found",
                    Some(json!({ "parent": ReqMcpServer::format_hrid(&parent, digits) })),
                )
            }
            requiem_core::storage::directory::AcceptSuspectLinkError::LinkNotFound {
                child,
                parent,
            } => McpError::invalid_params(
                "link between child and parent does not exist",
                Some(json!({
                    "child": ReqMcpServer::format_hrid(&child, digits),
                    "parent": ReqMcpServer::format_hrid(&parent, digits)
                })),
            ),
        })?;

    flush(&mut directory)?;

    let status = match result {
        requiem_core::storage::directory::AcceptResult::Updated => "updated",
        requiem_core::storage::directory::AcceptResult::AlreadyUpToDate => "already_up_to_date",
    }
    .to_string();

    let response = ReviewRequirementResponse {
        child: ReqMcpServer::format_hrid(&child, digits),
        parent: ReqMcpServer::format_hrid(&parent, digits),
        status,
    };

    drop(directory);

    Ok(ReqMcpServer::success(
        "Suspect link marked as reviewed",
        ReqMcpServer::serialize(response, "review_requirement response")?,
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::state::ServerState;

    fn server_with_root() -> (tempfile::TempDir, ReqMcpServer) {
        let tmp = tempfile::tempdir().unwrap();
        let state = ServerState::new(tmp.path()).unwrap();
        (tmp, ReqMcpServer::new(state))
    }

    fn structured(result: &CallToolResult) -> Value {
        result
            .structured_content
            .clone()
            .expect("tool result should carry structured content")
    }

    async fn create(
        server: &ReqMcpServer,
        kind: &str,
        title: &str,
        parents: Vec<String>,
    ) -> String {
        let result = create_requirement(
            server,
            Parameters(CreateRequirementParams {
                namespace: vec![],
                kind: kind.to_string(),
                title: title.to_string(),
                body: "Body".to_string(),
                parents,
            }),
        )
        .await
        .expect("create_requirement should succeed");
        structured(&result)["hrid"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn update_requirement_edits_content_in_place() {
        let (tmp, server) = server_with_root();
        let hrid = create(&server, "REQ", "Old title", vec![]).await;

        let result = update_requirement(
            &server,
            Parameters(UpdateRequirementParams {
                hrid: hrid.clone(),
                title: Some("New title".to_string()),
                body: Some("New body".to_string()),
                tags: Some(vec!["safety".to_string()]),
            }),
        )
        .await
        .expect("update should succeed");

        let data = structured(&result);
        assert_eq!(data["changed"], Value::Bool(true));
        assert_eq!(data["requirement"]["title"], "New title");
        assert_eq!(data["requirement"]["body"], "New body");
        assert_eq!(data["requirement"]["tags"][0], "safety");

        // The change is persisted to disk.
        let on_disk = std::fs::read_to_string(tmp.path().join(format!("{hrid}.md"))).unwrap();
        assert!(on_disk.contains("New body"));

        // Repeating the same update reports no change.
        let result = update_requirement(
            &server,
            Parameters(UpdateRequirementParams {
                hrid,
                title: Some("New title".to_string()),
                body: Some("New body".to_string()),
                tags: Some(vec!["safety".to_string()]),
            }),
        )
        .await
        .expect("no-op update should succeed");
        assert_eq!(structured(&result)["changed"], Value::Bool(false));
    }

    #[tokio::test]
    async fn update_requirement_requires_at_least_one_field() {
        let (_tmp, server) = server_with_root();
        let hrid = create(&server, "REQ", "Title", vec![]).await;

        let result = update_requirement(
            &server,
            Parameters(UpdateRequirementParams {
                hrid,
                title: None,
                body: None,
                tags: None,
            }),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn link_and_unlink_round_trip() {
        let (_tmp, server) = server_with_root();
        let parent = create(&server, "SYS", "Parent", vec![]).await;
        let child = create(&server, "USR", "Child", vec![]).await;

        let result = link_requirement(
            &server,
            Parameters(LinkRequirementParams {
                child: child.clone(),
                parent: parent.clone(),
            }),
        )
        .await
        .expect("link should succeed");
        assert_eq!(structured(&result)["alreadyLinked"], Value::Bool(false));

        // Relinking the same pair is reported, not an error.
        let result = link_requirement(
            &server,
            Parameters(LinkRequirementParams {
                child: child.clone(),
                parent: parent.clone(),
            }),
        )
        .await
        .expect("relink should succeed");
        assert_eq!(structured(&result)["alreadyLinked"], Value::Bool(true));

        // Linking the parent to its own descendant is rejected as a cycle.
        let result = link_requirement(
            &server,
            Parameters(LinkRequirementParams {
                child: parent.clone(),
                parent: child.clone(),
            }),
        )
        .await;
        assert!(result.is_err(), "cycle-forming link must be rejected");

        unlink_requirement(
            &server,
            Parameters(UnlinkRequirementParams {
                child: child.clone(),
                parent: parent.clone(),
            }),
        )
        .await
        .expect("unlink should succeed");

        // The link is gone, so unlinking again fails.
        let result = unlink_requirement(
            &server,
            Parameters(UnlinkRequirementParams { child, parent }),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_refuses_requirement_with_children_by_default() {
        let (_tmp, server) = server_with_root();
        let parent = create(&server, "SYS", "Parent", vec![]).await;
        let _child = create(&server, "USR", "Child", vec![parent.clone()]).await;

        let result = delete_requirement(
            &server,
            Parameters(DeleteRequirementParams {
                hrid: parent,
                mode: DeleteMode::Refuse,
                dry_run: false,
            }),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_cascade_removes_orphaned_descendants() {
        let (tmp, server) = server_with_root();
        let parent = create(&server, "SYS", "Parent", vec![]).await;
        let child = create(&server, "USR", "Child", vec![parent.clone()]).await;

        // Dry run reports both without deleting anything.
        let result = delete_requirement(
            &server,
            Parameters(DeleteRequirementParams {
                hrid: parent.clone(),
                mode: DeleteMode::Cascade,
                dry_run: true,
            }),
        )
        .await
        .expect("dry run should succeed");
        let data = structured(&result);
        assert_eq!(data["dryRun"], Value::Bool(true));
        assert_eq!(data["deleted"].as_array().unwrap().len(), 2);
        assert!(tmp.path().join(format!("{parent}.md")).exists());

        let result = delete_requirement(
            &server,
            Parameters(DeleteRequirementParams {
                hrid: parent.clone(),
                mode: DeleteMode::Cascade,
                dry_run: false,
            }),
        )
        .await
        .expect("cascade delete should succeed");
        let data = structured(&result);
        assert_eq!(data["deleted"].as_array().unwrap().len(), 2);
        assert!(!tmp.path().join(format!("{parent}.md")).exists());
        assert!(!tmp.path().join(format!("{child}.md")).exists());
    }

    #[tokio::test]
    async fn delete_orphan_keeps_children() {
        let (tmp, server) = server_with_root();
        let parent = create(&server, "SYS", "Parent", vec![]).await;
        let child = create(&server, "USR", "Child", vec![parent.clone()]).await;

        let result = delete_requirement(
            &server,
            Parameters(DeleteRequirementParams {
                hrid: parent.clone(),
                mode: DeleteMode::Orphan,
                dry_run: false,
            }),
        )
        .await
        .expect("orphan delete should succeed");
        let data = structured(&result);
        assert_eq!(data["deleted"].as_array().unwrap().len(), 1);
        assert_eq!(data["unlinkedChildren"][0], child.clone());
        assert!(!tmp.path().join(format!("{parent}.md")).exists());
        assert!(tmp.path().join(format!("{child}.md")).exists());
    }
}
