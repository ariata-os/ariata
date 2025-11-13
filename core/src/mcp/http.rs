//! MCP HTTP endpoint integration for Axum
//!
//! This module provides MCP HTTP/SSE transport that integrates with the existing
//! Axum server, using the official rmcp Tower service.

use axum::{routing::any_service, Router};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService, StreamableHttpServerConfig,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use super::AriataMcpServer;

/// Add MCP routes to an existing Axum router
///
/// This function adds the `/mcp` endpoint to your router that handles:
/// - GET: Server-Sent Events (SSE) for streaming responses
/// - POST: JSON-RPC requests
/// - DELETE: Session cleanup
///
/// # Example
/// ```rust,no_run
/// use axum::Router;
/// use ariata::mcp::{AriataMcpServer, http::add_mcp_routes};
///
/// let router = Router::new();
/// let mcp_server = AriataMcpServer::new(pool);
/// let router = add_mcp_routes(router, mcp_server);
/// ```
pub fn add_mcp_routes(router: Router, server: AriataMcpServer) -> Router {
    info!("Configuring MCP endpoint at /mcp");

    // Configure the MCP service
    let config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(30)),
        stateful_mode: true,
    };

    let session_manager = Arc::new(LocalSessionManager::default());
    let server_arc = Arc::new(server);

    // Create the MCP service using Tower integration
    // This service implements tower_service::Service and works directly with Axum
    let mcp_service = StreamableHttpService::new(
        move || Ok(server_arc.as_ref().clone()),
        session_manager,
        config,
    );

    // Add the MCP endpoint to the router
    // The `any_service` route handler accepts GET (SSE), POST (requests), and DELETE (cleanup)
    // We use `any_service` instead of `any` because mcp_service is a Tower Service, not a Handler
    router.route("/mcp", any_service(mcp_service))
}
