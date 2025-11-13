//! Model Context Protocol (MCP) Server
//!
//! This module implements an MCP server that exposes Ariata's data warehouse
//! as tools and resources for AI assistants like Claude Desktop.
//!
//! The server provides:
//! - Read-only SQL queries against ontology tables
//! - List available data sources and their status
//! - Trigger manual syncs
//! - Dynamic schema introspection

pub mod http;
pub mod schema;
pub mod server;
pub mod tools;

pub use server::AriataMcpServer;
