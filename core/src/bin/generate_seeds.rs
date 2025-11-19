//! Generate seed configuration files from the Rust registry
//!
//! This binary auto-generates config/seeds/_generated_source_connections.json and
//! config/seeds/_generated_stream_connections.json from the compile-time source registry,
//! ensuring consistency and eliminating duplication.
//!
//! **Usage:**
//! ```bash
//! cargo run --bin generate-seeds
//! # or
//! make generate-seeds
//! ```
//!
//! **What it generates:**
//! - `config/seeds/_generated_source_connections.json` - Only internal sources (auth_type = None)
//! - `config/seeds/_generated_stream_connections.json` - All streams for internal sources
//!
//! **Why only internal sources?**
//! OAuth sources (Google, Notion) require user-specific tokens and are created via OAuth flow.
//! Device sources (iOS, Mac) require device pairing and are created via pairing flow.
//! Only internal sources can be seeded directly into the database.

use anyhow::{Context, Result};
use ariata::registry::{self, AuthType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Source connection seed configuration for JSON
#[derive(Debug, Serialize, Deserialize)]
struct SourceConnectionSeed {
    id: Uuid,
    source: String,
    name: String,
    auth_type: String,
    is_active: bool,
    is_internal: bool,
}

/// Source connections JSON structure
#[derive(Debug, Serialize)]
struct SourceConnectionsJson {
    version: String,
    connections: Vec<SourceConnectionSeed>,
}

/// Stream connection seed configuration for JSON
#[derive(Debug, Serialize, Deserialize)]
struct StreamConnectionSeed {
    id: Uuid,
    source_connection_id: Uuid,
    stream_name: String,
    table_name: String,
    is_enabled: bool,
}

/// Stream connections JSON structure
#[derive(Debug, Serialize)]
struct StreamConnectionsJson {
    version: String,
    connections: Vec<StreamConnectionSeed>,
}

/// Generate a deterministic UUID from a string
/// This ensures UUIDs remain stable across regenerations
fn deterministic_uuid(namespace: &str, name: &str) -> Uuid {
    // Use UUID v5 (SHA-1 hash based) with a custom namespace
    let namespace_uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    Uuid::new_v5(&namespace_uuid, format!("{}:{}", namespace, name).as_bytes())
}

fn main() -> Result<()> {
    println!("🔧 Generating seed configurations from registry...");

    // Get all sources from registry
    let all_sources = registry::list_sources();

    // Filter to only internal sources (auth_type = None)
    let internal_sources: Vec<_> = all_sources
        .iter()
        .filter(|s| s.auth_type == AuthType::None)
        .collect();

    if internal_sources.is_empty() {
        println!("⚠️  No internal sources found in registry (auth_type = None)");
        println!("   Generating empty configuration files");
    } else {
        println!("📦 Found {} internal source(s):", internal_sources.len());
        for source in &internal_sources {
            println!("   - {} ({} streams)", source.display_name, source.streams.len());
        }
    }

    // Generate source connections configuration
    let mut source_connections = Vec::new();
    let mut stream_connections = Vec::new();

    for source in internal_sources {
        let source_connection_id = deterministic_uuid("source", source.name);

        // Convert auth_type enum to string
        let auth_type_str = match source.auth_type {
            AuthType::OAuth2 => "oauth2",
            AuthType::Device => "device",
            AuthType::ApiKey => "api_key",
            AuthType::None => "none",
        };

        // Add source connection
        source_connections.push(SourceConnectionSeed {
            id: source_connection_id,
            source: source.name.to_string(),
            name: format!("{}-app", source.name),
            auth_type: auth_type_str.to_string(),
            is_active: true,
            is_internal: true,
        });

        // Add stream connections for this source
        for stream in &source.streams {
            let stream_id = deterministic_uuid("stream", &format!("{}:{}", source.name, stream.name));

            stream_connections.push(StreamConnectionSeed {
                id: stream_id,
                source_connection_id,
                stream_name: stream.name.to_string(),
                table_name: stream.table_name.to_string(),
                is_enabled: true,
            });
        }
    }

    // Create JSON structures
    let source_connections_json = SourceConnectionsJson {
        version: "1.0.0".to_string(),
        connections: source_connections,
    };

    let stream_connections_json = StreamConnectionsJson {
        version: "1.0.0".to_string(),
        connections: stream_connections,
    };

    // Determine output paths
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_dir = Path::new(manifest_dir).join("..").join("config").join("seeds");

    let source_connections_path = config_dir.join("_generated_source_connections.json");
    let stream_connections_path = config_dir.join("_generated_stream_connections.json");

    // Write _generated_source_connections.json
    let source_connections_content = serde_json::to_string_pretty(&source_connections_json)
        .context("Failed to serialize _generated_source_connections.json")?;
    fs::write(&source_connections_path, source_connections_content)
        .with_context(|| format!("Failed to write _generated_source_connections.json to {}", source_connections_path.display()))?;
    println!("✅ Generated: {}", source_connections_path.display());

    // Write _generated_stream_connections.json
    let stream_connections_content = serde_json::to_string_pretty(&stream_connections_json)
        .context("Failed to serialize _generated_stream_connections.json")?;
    fs::write(&stream_connections_path, stream_connections_content)
        .with_context(|| format!("Failed to write _generated_stream_connections.json to {}", stream_connections_path.display()))?;
    println!("✅ Generated: {}", stream_connections_path.display());

    println!();
    println!("🎉 Seed generation complete!");
    println!();
    println!("📝 Summary:");
    println!("   - {} source connection(s) generated", source_connections_json.connections.len());
    println!("   - {} stream connection(s) generated", stream_connections_json.connections.len());
    println!();
    println!("💡 Next steps:");
    println!("   1. Review the generated files");
    println!("   2. Run 'cargo run --bin prod-seed' to seed the database");

    Ok(())
}
