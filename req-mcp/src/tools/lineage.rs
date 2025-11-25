use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, ErrorData as McpError};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::server::ReqMcpServer;

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetChildrenParams {
    /// Human-readable identifier to retrieve children for.
    pub hrid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetChildrenResponse {
    /// The requested HRID.
    pub hrid: String,
    /// Direct child HRIDs.
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetParentsResponse {
    /// The requested HRID.
    pub hrid: String,
    /// Direct parent HRIDs.
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetAncestorsResponse {
    /// The requested HRID.
    pub hrid: String,
    /// All ancestor HRIDs (deduplicated).
    pub ancestors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetDescendantsResponse {
    /// The requested HRID.
    pub hrid: String,
    /// All descendant HRIDs (deduplicated).
    pub descendants: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetLineageParams {
    /// Human-readable identifier to traverse from.
    pub hrid: String,
}

pub(super) async fn get_children(
    server: &ReqMcpServer,
    params: Parameters<GetChildrenParams>,
) -> Result<CallToolResult, McpError> {
    let hrid = ReqMcpServer::parse_hrid(&params.0.hrid)?;

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();

        if directory.find_by_hrid(&hrid).is_none() {
            return Err(McpError::resource_not_found(
                "requirement not found",
                Some(json!({ "hrid": params.0.hrid })),
            ));
        }

        let children = directory
            .children_of(&hrid)
            .into_iter()
            .map(|child| ReqMcpServer::format_hrid(&child, digits))
            .collect();

        drop(directory);

        GetChildrenResponse {
            hrid: ReqMcpServer::format_hrid(&hrid, digits),
            children,
        }
    };

    let summary = format!("{} children returned", response.children.len());
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "get_children response")?,
    ))
}

pub(super) async fn get_parents(
    server: &ReqMcpServer,
    params: Parameters<GetLineageParams>,
) -> Result<CallToolResult, McpError> {
    let hrid = ReqMcpServer::parse_hrid(&params.0.hrid)?;

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();

        let Some(view) = directory.find_by_hrid(&hrid) else {
            return Err(McpError::resource_not_found(
                "requirement not found",
                Some(json!({ "hrid": params.0.hrid })),
            ));
        };

        let parents = view
            .parents
            .iter()
            .map(|(_, parent)| ReqMcpServer::format_hrid(&parent.hrid, digits))
            .collect();

        drop(directory);

        GetParentsResponse {
            hrid: ReqMcpServer::format_hrid(&hrid, digits),
            parents,
        }
    };

    let summary = format!("{} parent(s) returned", response.parents.len());
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "get_parents response")?,
    ))
}

pub(super) async fn get_ancestors(
    server: &ReqMcpServer,
    params: Parameters<GetLineageParams>,
) -> Result<CallToolResult, McpError> {
    let hrid = ReqMcpServer::parse_hrid(&params.0.hrid)?;

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();

        if directory.find_by_hrid(&hrid).is_none() {
            return Err(McpError::resource_not_found(
                "requirement not found",
                Some(json!({ "hrid": params.0.hrid })),
            ));
        }

        let ancestors = directory
            .ancestors_of(&hrid)
            .into_iter()
            .map(|hrid| ReqMcpServer::format_hrid(&hrid, digits))
            .collect();

        drop(directory);

        GetAncestorsResponse {
            hrid: ReqMcpServer::format_hrid(&hrid, digits),
            ancestors,
        }
    };

    let summary = format!("{} ancestor(s) returned", response.ancestors.len());
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "get_ancestors response")?,
    ))
}

pub(super) async fn get_descendants(
    server: &ReqMcpServer,
    params: Parameters<GetLineageParams>,
) -> Result<CallToolResult, McpError> {
    let hrid = ReqMcpServer::parse_hrid(&params.0.hrid)?;

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();

        if directory.find_by_hrid(&hrid).is_none() {
            return Err(McpError::resource_not_found(
                "requirement not found",
                Some(json!({ "hrid": params.0.hrid })),
            ));
        }

        let descendants = directory
            .descendants_of(&hrid)
            .into_iter()
            .map(|hrid| ReqMcpServer::format_hrid(&hrid, digits))
            .collect();

        drop(directory);

        GetDescendantsResponse {
            hrid: ReqMcpServer::format_hrid(&hrid, digits),
            descendants,
        }
    };

    let summary = format!("{} descendant(s) returned", response.descendants.len());
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "get_descendants response")?,
    ))
}
