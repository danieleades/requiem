use std::fs;

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
    /// Updated title (optional).
    #[serde(default)]
    pub title: Option<String>,
    /// Updated body (optional).
    #[serde(default)]
    pub body: Option<String>,
    /// Replace parent HRIDs (optional).
    #[serde(default)]
    pub parents: Option<Vec<String>>,
    /// Replace tags (optional).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
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
        .map(|segment| segment.trim().to_uppercase())
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

    directory.flush().map_err(|error| {
        McpError::internal_error(
            "failed to persist requirement",
            Some(json!({ "reason": error.to_string() })),
        )
    })?;

    let hrid = requirement.hrid().clone();
    let Some(view) = directory.find_by_hrid(&hrid) else {
        return Err(McpError::internal_error(
            "created requirement could not be reloaded",
            Some(json!({ "hrid": ReqMcpServer::format_hrid(&hrid, digits) })),
        ));
    };

    let parents: Vec<String> = view
        .parents
        .iter()
        .map(|(_, parent)| ReqMcpServer::format_hrid(&parent.hrid, digits))
        .collect();

    let children: Vec<String> = directory
        .children_of(&hrid)
        .iter()
        .map(|child| ReqMcpServer::format_hrid(child, digits))
        .collect();

    let response = RequirementDetails {
        hrid: ReqMcpServer::format_hrid(&hrid, digits),
        title: view.title.to_string(),
        body: view.body.to_string(),
        tags: view.tags.iter().cloned().collect(),
        parents,
        children,
    };

    drop(directory);

    let summary = format!("Created requirement {}", response.hrid);
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "create_requirement response")?,
    ))
}

#[allow(clippy::unused_async)]
pub(super) async fn update_requirement(
    _server: &ReqMcpServer,
    params: Parameters<UpdateRequirementParams>,
) -> Result<CallToolResult, McpError> {
    Ok(ReqMcpServer::stub(
        "update_requirement",
        Some(ReqMcpServer::serialize(
            &params.0,
            "update_requirement params",
        )?),
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

    directory.flush().map_err(|error| {
        McpError::internal_error(
            "failed to persist review update",
            Some(json!({ "reason": error.to_string() })),
        )
    })?;

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
