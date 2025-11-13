//! Ariata MCP Server Binary
//!
//! This binary starts the Ariata MCP server with stdio transport for AI assistants like Claude Desktop.
//! For HTTP transport, use the main Ariata server (`cargo run serve`) which includes MCP at /mcp endpoint.

use anyhow::Result;
use ariata::mcp::AriataMcpServer;
use clap::Parser;
use dotenv::dotenv;
use rmcp::{transport::stdio, ServiceExt};
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "ariata-mcp-server")]
#[command(about = "Ariata MCP Server - Expose your personal data warehouse to AI assistants via stdio")]
struct Args {}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Parse command-line arguments
    let _args = Args::parse();

    // Initialize tracing to stderr to avoid interfering with stdio transport
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    info!("Starting Ariata MCP Server (stdio transport)");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment or .env file");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    // Create MCP server
    let server = AriataMcpServer::new(pool);

    // Start server on stdio
    info!("Starting MCP server on stdio");
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    info!("MCP server shut down");

    Ok(())
}
