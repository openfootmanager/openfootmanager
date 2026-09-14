use std::sync::Arc;

use crate::mcp_server::config::McpConfig;
use crate::SaveManagerState;
use ofm_core::state::StateManager;

/// Shared context available to all MCP tool handlers.
#[derive(Clone)]
pub struct McpContext {
    pub state_manager: Arc<StateManager>,
    pub save_manager_state: Arc<SaveManagerState>,
    pub app_handle: tauri::AppHandle,
    pub config: McpConfig,
}
