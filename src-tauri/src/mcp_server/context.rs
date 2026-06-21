use std::sync::Arc;

use ofm_core::state::StateManager;
use crate::SaveManagerState;
use crate::mcp_server::config::McpConfig;

/// Shared context available to all MCP tool handlers.
#[derive(Clone)]
pub struct McpContext {
    pub state_manager: Arc<StateManager>,
    pub save_manager_state: Arc<SaveManagerState>,
    pub app_handle: tauri::AppHandle,
    pub config: McpConfig,
}
