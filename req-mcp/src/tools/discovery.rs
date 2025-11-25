use std::collections::BTreeSet;

use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, ErrorData as McpError};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::server::ReqMcpServer;

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListRequirementsParams {
    /// Required kind filter, e.g. "USR".
    pub kind: String,
    /// Optional substring search applied to title or body (case-insensitive).
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequirementSummary {
    /// Human-readable identifier.
    pub hrid: String,
    /// Requirement title.
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListRequirementsResponse {
    /// Normalized kind filter that was applied.
    pub kind: String,
    /// Optional query string that was applied.
    #[serde(default)]
    pub query: Option<String>,
    /// Matching requirements.
    pub results: Vec<RequirementSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListRequirementKindsResponse {
    /// All known requirement kinds found in the repository.
    pub kinds: Vec<RequirementKind>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequirementKind {
    /// Requirement kind identifier, e.g. "USR".
    pub kind: String,
    /// Human-readable description of the kind's purpose.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetRequirementParams {
    /// Human-readable identifier to look up.
    pub hrid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequirementDetails {
    /// Human-readable identifier.
    pub hrid: String,
    /// Title of the requirement.
    pub title: String,
    /// Markdown body content.
    pub body: String,
    /// Tags on the requirement.
    pub tags: Vec<String>,
    /// Direct parent HRIDs.
    pub parents: Vec<String>,
    /// Direct child HRIDs.
    pub children: Vec<String>,
}

pub(super) async fn list_requirement_kinds(
    server: &ReqMcpServer,
) -> Result<CallToolResult, McpError> {
    let response: ListRequirementKindsResponse = {
        let directory = server.state.directory.read().await;
        let mut kinds: BTreeSet<String> = BTreeSet::new();
        let metadata = directory.config().kind_metadata().clone();

        // Include configured kinds (if any), those present in metadata, and those
        // observed in requirements.
        kinds.extend(directory.config().allowed_kinds().iter().cloned());
        kinds.extend(metadata.keys().cloned());
        for requirement in directory.requirements() {
            kinds.insert(requirement.hrid.kind().to_string());
        }
        drop(directory);

        let kinds = kinds
            .into_iter()
            .map(|kind| {
                let meta = metadata.get(&kind);
                RequirementKind {
                    kind,
                    description: meta.and_then(|m| m.description.clone()),
                }
            })
            .collect();

        ListRequirementKindsResponse { kinds }
    };

    let summary = format!("Found {} requirement kinds", response.kinds.len());
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "list_requirement_kinds response")?,
    ))
}

pub(super) async fn list_requirements(
    server: &ReqMcpServer,
    params: Parameters<ListRequirementsParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    if params.kind.trim().is_empty() {
        return Err(McpError::invalid_params(
            "`kind` is required",
            Some(json!({ "field": "kind" })),
        ));
    }

    let filter_kind = params.kind.to_uppercase();
    let query = params.query.clone().map(|value| value.to_lowercase());

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();
        let results = directory
            .requirements()
            .filter(|view| view.hrid.kind() == filter_kind)
            .filter(|view| {
                query.as_ref().is_none_or(|query| {
                    view.title.to_lowercase().contains(query)
                        || view.body.to_lowercase().contains(query)
                })
            })
            .map(|view| RequirementSummary {
                hrid: ReqMcpServer::format_hrid(view.hrid, digits),
                title: view.title.to_string(),
            })
            .collect();

        drop(directory);

        ListRequirementsResponse {
            kind: filter_kind.clone(),
            query: params.query,
            results,
        }
    };

    let summary = format!(
        "Found {} requirements of kind {}",
        response.results.len(),
        response.kind
    );

    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "list_requirements response")?,
    ))
}

pub(super) async fn get_requirement(
    server: &ReqMcpServer,
    params: Parameters<GetRequirementParams>,
) -> Result<CallToolResult, McpError> {
    let hrid = ReqMcpServer::parse_hrid(&params.0.hrid)?;

    let directory = server.state.directory.read().await;
    let digits = directory.config().digits();
    let Some(view) = directory.find_by_hrid(&hrid) else {
        return Err(McpError::resource_not_found(
            "requirement not found",
            Some(json!({ "hrid": params.0.hrid })),
        ));
    };

    let view_title = view.title.to_string();
    let view_body = view.body.to_string();
    let view_tags: Vec<String> = view.tags.iter().cloned().collect();
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

    drop(directory);

    let response = RequirementDetails {
        hrid: ReqMcpServer::format_hrid(&hrid, digits),
        title: view_title,
        body: view_body,
        tags: view_tags,
        parents,
        children,
    };

    let summary = format!("Fetched requirement {}", response.hrid);
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "get_requirement response")?,
    ))
}
