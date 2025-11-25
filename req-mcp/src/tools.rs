mod discovery;
mod editing;
mod lineage;
mod search;

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::server::ReqMcpServer;

#[tool_router]
impl ReqMcpServer {
    #[tool(
        description = "List all requirement kinds found in the repository",
        annotations(
            title = "List Requirement Kinds",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_requirement_kinds(&self) -> Result<CallToolResult, McpError> {
        discovery::list_requirement_kinds(self).await
    }

    #[tool(
        description = "List requirements for a kind with optional substring search on title/body",
        annotations(
            title = "List Requirements",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_requirements(
        &self,
        params: Parameters<discovery::ListRequirementsParams>,
    ) -> Result<CallToolResult, McpError> {
        discovery::list_requirements(self, params).await
    }

    #[tool(
        description = "Get a requirement by HRID with title, body, tags, parents, children",
        annotations(
            title = "Get Requirement",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_requirement(
        &self,
        params: Parameters<discovery::GetRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        discovery::get_requirement(self, params).await
    }

    #[tool(
        description = "List direct child HRIDs for a requirement",
        annotations(
            title = "Get Children",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_children(
        &self,
        params: Parameters<lineage::GetChildrenParams>,
    ) -> Result<CallToolResult, McpError> {
        lineage::get_children(self, params).await
    }

    #[tool(
        description = "Search requirements by text or regex (not implemented yet)",
        annotations(
            title = "Search Requirements",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_requirements(
        &self,
        params: Parameters<search::SearchRequirementsParams>,
    ) -> Result<CallToolResult, McpError> {
        search::search_requirements(self, params).await
    }

    #[tool(
        description = "List suspect parent-child links where stored fingerprints drifted; start \
                       here before marking reviewed",
        annotations(
            title = "List Pending Reviews",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn review(
        &self,
        params: Parameters<search::ReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        search::review(self, params).await
    }

    #[tool(
        description = "List direct parents for a requirement",
        annotations(
            title = "Get Parents",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_parents(
        &self,
        params: Parameters<lineage::GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        lineage::get_parents(self, params).await
    }

    #[tool(
        description = "List all ancestor requirements recursively (deduped)",
        annotations(
            title = "Get Ancestors",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_ancestors(
        &self,
        params: Parameters<lineage::GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        lineage::get_ancestors(self, params).await
    }

    #[tool(
        description = "List all descendant requirements recursively (deduped)",
        annotations(
            title = "Get Descendants",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_descendants(
        &self,
        params: Parameters<lineage::GetLineageParams>,
    ) -> Result<CallToolResult, McpError> {
        lineage::get_descendants(self, params).await
    }

    #[tool(
        description = "Create a new requirement kind and persist it to config",
        annotations(
            title = "Create Requirement Kind",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_requirement_kind(
        &self,
        params: Parameters<editing::CreateRequirementKindParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::create_requirement_kind(self, params).await
    }

    #[tool(
        description = "Create a requirement with optional namespace and parents",
        annotations(
            title = "Create Requirement",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_requirement(
        &self,
        params: Parameters<editing::CreateRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::create_requirement(self, params).await
    }

    #[tool(
        description = "Update a requirement's title, body, parents, or tags (not implemented yet)",
        annotations(
            title = "Update Requirement",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn update_requirement(
        &self,
        params: Parameters<editing::UpdateRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::update_requirement(self, params).await
    }

    #[tool(
        description = "Mark a suspect link as reviewed by refreshing the stored parent \
                       fingerprint on the child",
        annotations(
            title = "Review Requirement",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn review_requirement(
        &self,
        params: Parameters<editing::ReviewRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::review_requirement(self, params).await
    }
}

impl ReqMcpServer {
    pub(crate) fn build_tool_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::tool_router()
    }
}
