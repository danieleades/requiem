//! MCP server implementation exposing requirement discovery and navigation
//! tools.

use std::collections::BTreeSet;

use requiem_core::Hrid;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::ServerState;

/// MCP server backed by a loaded requirements directory.
#[derive(Clone)]
pub struct ReqMcpServer {
    /// Shared directory and configuration state.
    pub(crate) state: ServerState,
    /// Generated router containing all exposed tools.
    pub(crate) tool_router: ToolRouter<Self>,
}

impl ReqMcpServer {
    /// Create a new server with the provided state.
    #[must_use]
    pub fn new(state: ServerState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    fn format_hrid(hrid: &Hrid, digits: usize) -> String {
        hrid.display(digits).to_string()
    }

    fn serialize<T: Serialize>(value: T, context: &str) -> Result<Value, McpError> {
        serde_json::to_value(value).map_err(|error| {
            McpError::internal_error(
                "failed to serialize response",
                Some(json!({ "context": context, "reason": error.to_string() })),
            )
        })
    }

    fn parse_hrid(raw: &str) -> Result<Hrid, McpError> {
        Hrid::try_from(raw).map_err(|error| {
            McpError::invalid_params(
                "invalid HRID provided",
                Some(json!({ "hrid": raw, "reason": error.to_string() })),
            )
        })
    }

    fn success(summary: impl Into<String>, data: Value) -> CallToolResult {
        CallToolResult {
            content: vec![Content::text(summary.into())],
            structured_content: Some(data),
            is_error: Some(false),
            meta: None,
        }
    }

    fn stub(tool: &str, params: Option<Value>) -> CallToolResult {
        CallToolResult {
            content: vec![Content::text(format!("{tool} is not implemented yet"))],
            structured_content: Some(json!({
                "status": "not_implemented",
                "tool": tool,
                "params": params.unwrap_or(Value::Null),
            })),
            is_error: Some(true),
            meta: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListRequirementsParams {
    /// Required kind filter, e.g. "USR".
    kind: String,
    /// Optional substring search applied to title or body (case-insensitive).
    #[serde(default)]
    query: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RequirementSummary {
    /// Human-readable identifier.
    hrid: String,
    /// Requirement title.
    title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListRequirementsResponse {
    /// Normalized kind filter that was applied.
    kind: String,
    /// Optional query string that was applied.
    #[serde(default)]
    query: Option<String>,
    /// Matching requirements.
    results: Vec<RequirementSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListRequirementKindsResponse {
    /// All known requirement kinds found in the repository.
    kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetRequirementParams {
    /// Human-readable identifier to look up.
    hrid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RequirementDetails {
    /// Human-readable identifier.
    hrid: String,
    /// Title of the requirement.
    title: String,
    /// Markdown body content.
    body: String,
    /// Tags on the requirement.
    tags: Vec<String>,
    /// Direct parent HRIDs.
    parents: Vec<String>,
    /// Direct child HRIDs.
    children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetChildrenParams {
    /// Human-readable identifier to retrieve children for.
    hrid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetChildrenResponse {
    /// The requested HRID.
    hrid: String,
    /// Direct child HRIDs.
    children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SearchRequirementsParams {
    /// Text or regex-like query for requirement content.
    query: String,
    /// Optional kind filter.
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ReviewParams {
    /// Optional kind filter for review queries.
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetLineageParams {
    /// Human-readable identifier to traverse from.
    hrid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateRequirementKindParams {
    /// New kind identifier, e.g. "USR".
    kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateRequirementParams {
    /// Optional namespace segments (upper-case) preceding the kind.
    #[serde(default)]
    namespace: Vec<String>,
    /// Requirement kind, e.g. "USR".
    kind: String,
    /// Title for the new requirement.
    title: String,
    /// Markdown body for the new requirement.
    body: String,
    /// Optional parent HRIDs to link to.
    #[serde(default)]
    parents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpdateRequirementParams {
    /// HRID of the requirement to update.
    hrid: String,
    /// Updated title (optional).
    #[serde(default)]
    title: Option<String>,
    /// Updated body (optional).
    #[serde(default)]
    body: Option<String>,
    /// Replace parent HRIDs (optional).
    #[serde(default)]
    parents: Option<Vec<String>>,
    /// Replace tags (optional).
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ReviewRequirementParams {
    /// HRID of the requirement being reviewed.
    hrid: String,
    /// Optional status marker, e.g. "approved" or "`changes_requested`".
    #[serde(default)]
    status: Option<String>,
}

#[tool_router]
impl ReqMcpServer {
    #[tool(description = "List available requirement kinds")]
    async fn list_requirement_kinds(&self) -> Result<CallToolResult, McpError> {
        let kinds: Vec<String> = {
            let directory = self.state.directory.read().await;
            let mut kinds: BTreeSet<String> = BTreeSet::new();
            for requirement in directory.requirements() {
                kinds.insert(requirement.hrid.kind().to_string());
            }
            drop(directory);
            kinds.into_iter().collect()
        };

        let response = ListRequirementKindsResponse { kinds };
        let summary = format!("Found {} requirement kinds", response.kinds.len());
        Ok(Self::success(
            summary,
            Self::serialize(response, "list_requirement_kinds response")?,
        ))
    }

    #[tool(description = "List requirements for a kind with optional substring filter")]
    async fn list_requirements(
        &self,
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
            let directory = self.state.directory.read().await;
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
                    hrid: Self::format_hrid(view.hrid, digits),
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

        Ok(Self::success(
            summary,
            Self::serialize(response, "list_requirements response")?,
        ))
    }

    #[tool(description = "Get a requirement by HRID, including parents and children")]
    async fn get_requirement(
        &self,
        params: Parameters<GetRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        let hrid = Self::parse_hrid(&params.0.hrid)?;

        let directory = self.state.directory.read().await;
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
            .map(|(_, parent)| Self::format_hrid(&parent.hrid, digits))
            .collect();

        let children: Vec<String> = directory
            .children_of(&hrid)
            .iter()
            .map(|child| Self::format_hrid(child, digits))
            .collect();

        drop(directory);

        let response = RequirementDetails {
            hrid: Self::format_hrid(&hrid, digits),
            title: view_title,
            body: view_body,
            tags: view_tags,
            parents,
            children,
        };

        let summary = format!("Fetched requirement {}", response.hrid);
        Ok(Self::success(
            summary,
            Self::serialize(response, "get_requirement response")?,
        ))
    }

    #[tool(description = "List direct children for a requirement")]
    async fn get_children(
        &self,
        params: Parameters<GetChildrenParams>,
    ) -> Result<CallToolResult, McpError> {
        let hrid = Self::parse_hrid(&params.0.hrid)?;

        let response = {
            let directory = self.state.directory.read().await;
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
                .map(|child| Self::format_hrid(&child, digits))
                .collect();

            drop(directory);

            GetChildrenResponse {
                hrid: Self::format_hrid(&hrid, digits),
                children,
            }
        };

        let summary = format!("{} children returned", response.children.len());
        Ok(Self::success(
            summary,
            Self::serialize(response, "get_children response")?,
        ))
    }

    #[tool(description = "Search requirements (stub)")]
    async fn search_requirements(
        &self,
        params: Parameters<SearchRequirementsParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "search_requirements",
            Some(Self::serialize(&params.0, "search_requirements params")?),
        ))
    }

    #[tool(description = "List requirements needing review (stub)")]
    async fn review(&self, params: Parameters<ReviewParams>) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "review",
            Some(Self::serialize(&params.0, "review params")?),
        ))
    }

    #[tool(description = "Get parent requirements (stub)")]
    async fn get_parents(
        &self,
        params: Parameters<GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "get_parents",
            Some(Self::serialize(&params.0, "get_parents params")?),
        ))
    }

    #[tool(description = "Get ancestor requirements (stub)")]
    async fn get_ancestors(
        &self,
        params: Parameters<GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "get_ancestors",
            Some(Self::serialize(&params.0, "get_ancestors params")?),
        ))
    }

    #[tool(description = "Get descendant requirements (stub)")]
    async fn get_descendants(
        &self,
        params: Parameters<GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "get_descendants",
            Some(Self::serialize(&params.0, "get_descendants params")?),
        ))
    }

    #[tool(description = "Create a requirement kind (stub)")]
    async fn create_requirement_kind(
        &self,
        params: Parameters<CreateRequirementKindParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "create_requirement_kind",
            Some(Self::serialize(
                &params.0,
                "create_requirement_kind params",
            )?),
        ))
    }

    #[tool(description = "Create a requirement (stub)")]
    async fn create_requirement(
        &self,
        params: Parameters<CreateRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "create_requirement",
            Some(Self::serialize(&params.0, "create_requirement params")?),
        ))
    }

    #[tool(description = "Update a requirement (stub)")]
    async fn update_requirement(
        &self,
        params: Parameters<UpdateRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "update_requirement",
            Some(Self::serialize(&params.0, "update_requirement params")?),
        ))
    }

    #[tool(description = "Review a requirement (stub)")]
    async fn review_requirement(
        &self,
        params: Parameters<ReviewRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::stub(
            "review_requirement",
            Some(Self::serialize(&params.0, "review_requirement params")?),
        ))
    }
}

#[tool_handler]
impl ServerHandler for ReqMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Use list_requirement_kinds → list_requirements → get_requirement → get_children \
                 to explore requirements. Other tools are stubbed."
                    .to_string(),
            ),
            ..ServerInfo::default()
        }
    }
}
