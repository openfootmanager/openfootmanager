pub mod config;
pub mod context;
mod formatting;
mod server;
pub mod tools;
mod tools_impl;

use std::sync::Arc;

use ofm_core::state::StateManager;

use crate::SaveManagerState;

pub use config::McpConfig;

/// Start the MCP SSE server on the configured port.
///
/// This spawns an axum HTTP server that implements the MCP Streamable HTTP
/// transport. The server runs on the tokio runtime (separate from Tauri's
/// thread pool) and accesses `StateManager` / `SaveManagerState` via
/// `Arc` references.
pub async fn start_mcp_server(
    config: McpConfig,
    state_manager: Arc<StateManager>,
    save_manager_state: Arc<SaveManagerState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mcp_handler = server::OfmMcpHandler::new(
        config.clone(),
        state_manager,
        save_manager_state,
        app_handle,
    );

    let session_manager =
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default();

    let service_factory = move || Ok::<_, std::io::Error>(mcp_handler.clone());

    let streamable_http_config =
        rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default()
            .with_sse_keep_alive(Some(std::time::Duration::from_secs(30)))
            .with_sse_retry(Some(std::time::Duration::from_secs(3)))
            .with_stateful_mode(true)
            .with_json_response(false)
            .with_allowed_hosts(config.allowed_hosts.clone());

    let service = rmcp::transport::streamable_http_server::StreamableHttpService::new(
        service_factory,
        Arc::new(session_manager),
        streamable_http_config,
    );

    let app = axum::Router::new()
        .fallback(axum::routing::any(move |req| {
            let service = service.clone();
            async move {
                let response = service.handle(req).await;
                response
            }
        }))
        .into_make_service();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(|e| format!("Failed to bind MCP server to port {}: {}", config.port, e))?;

    log::info!(
        "[mcp] MCP SSE server listening on 0.0.0.0:{}",
        config.port
    );

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("MCP server error: {}", e))
}
