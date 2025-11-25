use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, ErrorData as McpError};
use serde::{Deserialize, Serialize};

use crate::server::ReqMcpServer;

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequirementsParams {
    /// Text or regex-like query for requirement content.
    pub query: String,
    /// Optional kind filter.
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewParams {
    /// Optional kind filter for review queries.
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuspectLinkView {
    /// Child HRID with stale parent fingerprint.
    pub child: String,
    /// Parent HRID referenced by the child.
    pub parent: String,
    /// Fingerprint stored on the child.
    pub stored_fingerprint: String,
    /// Current fingerprint of the parent (empty if parent missing).
    pub current_fingerprint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResponse {
    /// Optional kind filter applied (matches either child or parent).
    #[serde(default)]
    pub kind: Option<String>,
    /// Suspect links needing review.
    pub suspect_links: Vec<SuspectLinkView>,
}

#[allow(clippy::unused_async)]
pub(super) async fn search_requirements(
    _server: &ReqMcpServer,
    params: Parameters<SearchRequirementsParams>,
) -> Result<CallToolResult, McpError> {
    Ok(ReqMcpServer::stub(
        "search_requirements",
        Some(ReqMcpServer::serialize(
            &params.0,
            "search_requirements params",
        )?),
    ))
}

#[allow(clippy::unused_async)]
pub(super) async fn review(
    server: &ReqMcpServer,
    params: Parameters<ReviewParams>,
) -> Result<CallToolResult, McpError> {
    let params = params.0;
    let kind_filter = params.kind.as_ref().map(|k| k.trim().to_uppercase());

    let response = {
        let directory = server.state.directory.read().await;
        let digits = directory.config().digits();

        let suspect_links = directory
            .suspect_links()
            .into_iter()
            .filter(|link| {
                kind_filter.as_ref().is_none_or(|kind| {
                    link.child_hrid.kind() == kind || link.parent_hrid.kind() == kind
                })
            })
            .map(|link| SuspectLinkView {
                child: ReqMcpServer::format_hrid(&link.child_hrid, digits),
                parent: ReqMcpServer::format_hrid(&link.parent_hrid, digits),
                stored_fingerprint: link.stored_fingerprint,
                current_fingerprint: link.current_fingerprint,
            })
            .collect();

        drop(directory);

        ReviewResponse {
            kind: kind_filter,
            suspect_links,
        }
    };

    let summary = format!(
        "{} suspect link(s) pending review",
        response.suspect_links.len()
    );
    Ok(ReqMcpServer::success(
        summary,
        ReqMcpServer::serialize(response, "review response")?,
    ))
}
