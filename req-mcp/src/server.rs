//! MCP server implementation exposing requirement discovery, navigation, and
//! editing tools.

use requiem_core::Hrid;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool_handler, ErrorData as McpError, ServerHandler,
};
use serde::Serialize;
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
            tool_router: Self::build_tool_router(),
        }
    }

    pub(crate) fn format_hrid(hrid: &Hrid, digits: usize) -> String {
        hrid.display(digits).to_string()
    }

    pub(crate) fn parse_hrid(raw: &str) -> Result<Hrid, McpError> {
        Hrid::try_from(raw).map_err(|error| {
            McpError::invalid_params(
                "invalid HRID provided",
                Some(json!({ "hrid": raw, "reason": error.to_string() })),
            )
        })
    }

    pub(crate) fn success(summary: impl Into<String>, data: Value) -> CallToolResult {
        CallToolResult {
            content: vec![Content::text(summary.into())],
            structured_content: Some(data),
            is_error: Some(false),
            meta: None,
        }
    }

    pub(crate) fn stub(tool: &str, params: Option<Value>) -> CallToolResult {
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

    pub(crate) fn serialize<T: Serialize>(value: T, context: &str) -> Result<Value, McpError> {
        serde_json::to_value(value).map_err(|error| {
            McpError::internal_error(
                "failed to serialize response",
                Some(json!({ "context": context, "reason": error.to_string() })),
            )
        })
    }
}

#[tool_handler]
impl ServerHandler for ReqMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Requirements graph MCP server (requires REQ_ROOT pointing at your requirements \
                 repo). Start with list_requirement_kinds, then list_requirements(kind) to get \
                 HRIDs. Fetch details with get_requirement(hrid) and traverse with \
                 get_children(hrid), get_parents(hrid), get_ancestors(hrid), or \
                 get_descendants(hrid). Create new kinds/requirements with \
                 create_requirement_kind and create_requirement. For link drift, call review to \
                 list suspect child→parent links (fingerprint mismatches), then \
                 review_requirement to accept if the child still satisfies the parent. \
                 Search/update tools remain placeholders for now."
                    .to_owned(),
            ),
            ..ServerInfo::default()
        }
    }
}
