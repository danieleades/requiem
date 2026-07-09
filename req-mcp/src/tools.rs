//! MCP tool definitions and the tool router, grouped into submodules by
//! concern (discovery, editing, lineage, search).

mod discovery;
mod editing;
mod lineage;
mod search;

use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
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
        description = "Update the title, body, and/or tags of an existing requirement in place; \
                       omitted fields are left unchanged",
        annotations(
            title = "Update Requirement",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
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
        description = "Link an existing requirement to a parent (child satisfies parent); \
                       relinking an existing pair refreshes the stored fingerprint",
        annotations(
            title = "Link Requirement",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn link_requirement(
        &self,
        params: Parameters<editing::LinkRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::link_requirement(self, params).await
    }

    #[tool(
        description = "Remove the parent-child link between two requirements",
        annotations(
            title = "Unlink Requirement",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn unlink_requirement(
        &self,
        params: Parameters<editing::UnlinkRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::unlink_requirement(self, params).await
    }

    #[tool(
        description = "Delete a requirement; mode controls children handling \
                       (refuse/orphan/cascade) and dryRun previews without changing anything",
        annotations(
            title = "Delete Requirement",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn delete_requirement(
        &self,
        params: Parameters<editing::DeleteRequirementParams>,
    ) -> Result<CallToolResult, McpError> {
        editing::delete_requirement(self, params).await
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

// The `#[tool_handler]` macro expands to `ServerHandler` methods that are async
// (as required by the trait) but contain no `.await`; allow the resulting lint.
// `unknown_lints` is allowed alongside it because the lint only exists on
// recent (nightly) clippy, and older/stable clippy would warn about the name.
#[allow(unknown_lints)]
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler]
impl ServerHandler for ReqMcpServer {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` is `#[non_exhaustive]`, so it can only be built from its
        // `Default` and then customised in place.
        #[allow(clippy::field_reassign_with_default)]
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Requirements graph MCP server (requires REQ_ROOT pointing at your requirements \
             repo). Start with list_requirement_kinds, then list_requirements(kind) to get HRIDs. \
             Fetch details with get_requirement(hrid) and traverse with get_children(hrid), \
             get_parents(hrid), get_ancestors(hrid), or get_descendants(hrid). Create new \
             kinds/requirements with create_requirement_kind and create_requirement. Edit \
             existing requirements with update_requirement (title/body/tags), manage traceability \
             with link_requirement and unlink_requirement, and remove requirements with \
             delete_requirement (mode: refuse/orphan/cascade; dryRun to preview). For link drift, \
             call review to list suspect child→parent links (fingerprint mismatches), then \
             review_requirement to accept if the child still satisfies the parent."
                .to_owned(),
        );
        info
    }
}
