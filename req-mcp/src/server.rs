//! MCP server implementation exposing requirement discovery, navigation, and
//! editing tools.

use requiem_core::Hrid;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{content::Content, CallToolResult},
    ErrorData as McpError,
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
        let mut result = CallToolResult::success(vec![Content::text(summary.into())]);
        result.structured_content = Some(data);
        result
    }

    pub(crate) fn stub(tool: &str, params: Option<Value>) -> CallToolResult {
        let mut result = CallToolResult::success(vec![Content::text(format!(
            "{tool} is not implemented yet"
        ))]);
        result.is_error = Some(true);
        result.structured_content = Some(json!({
            "status": "not_implemented",
            "tool": tool,
            "params": params.unwrap_or(Value::Null),
        }));
        result
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
