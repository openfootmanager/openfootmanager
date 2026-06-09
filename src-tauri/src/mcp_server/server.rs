use std::future::Future;
use std::sync::Arc;

use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{Implementation, ServerCapabilities};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};

use crate::mcp_server::config::McpConfig;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools::OfmToolRouter;

/// The MCP server handler for OpenFoot Manager.
///
/// Implements `rmcp::ServerHandler` which routes incoming `tools/call` requests
/// to the appropriate tool function via the `ToolRouter`.
#[derive(Clone)]
pub struct OfmMcpHandler {
    context: Arc<McpContext>,
    tool_router: OfmToolRouter,
}

impl OfmMcpHandler {
    pub fn new(
        config: McpConfig,
        state_manager: Arc<ofm_core::state::StateManager>,
        save_manager_state: Arc<crate::SaveManagerState>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        // Build tool router, respecting disabled tools from config
        let disabled = Self::collect_disabled_tools(&config);
        let context = Arc::new(McpContext {
            state_manager,
            save_manager_state,
            app_handle,
            config,
        });

        let tool_router = crate::mcp_server::tools::build_tool_router(&context, &disabled);

        Self {
            context,
            tool_router,
        }
    }

    /// Collect all disabled tool names from mode + denylist.
    fn collect_disabled_tools(config: &McpConfig) -> Vec<String> {
        let mut disabled: Vec<String> = config
            .mode
            .disabled_tools()
            .iter()
            .map(|s| s.to_string())
            .collect();
        disabled.extend(config.disabled_tools.clone());
        disabled
    }
}

impl ServerHandler for OfmMcpHandler {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .build();
        rmcp::model::ServerInfo::new(capabilities).with_server_info(Implementation::new(
            "OpenFoot Manager MCP Server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListToolsResult, McpError>> + MaybeSendFuture + '_
    {
        let tools = self.tool_router.list_all();
        std::future::ready(Ok(rmcp::model::ListToolsResult {
            tools,
            ..Default::default()
        }))
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::CallToolResult, McpError>> + MaybeSendFuture + '_
    {
        // ToolCallContext borrows the service, so we need to pass &Arc<McpContext>
        // which our context field is. The borrow lives as long as self.
        let tool_call_context = ToolCallContext::new(&self.context, request, context);
        async move { self.tool_router.call(tool_call_context).await }
    }

    fn get_tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        self.tool_router.get(name).cloned()
    }
}
