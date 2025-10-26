//! Example: List all available sources and streams from the registry
//!
//! This demonstrates how frontends and CLIs can discover what sources
//! and streams are available, along with their configuration options.
//!
//! Run with: cargo run --example list_catalog

use ariata::{list_available_sources, get_source_info, list_all_streams};

fn main() {
    println!("=== Ariata Source Catalog ===\n");

    // List all available sources
    let sources = list_available_sources();
    println!("Found {} sources:\n", sources.len());

    for source in sources {
        println!("📦 {} ({})", source.display_name, source.name);
        println!("   {}", source.description);
        println!("   Auth: {:?}", source.auth_type);
        println!("   Streams: {}", source.streams.len());

        if let Some(oauth) = &source.oauth_config {
            println!("   OAuth Scopes: {:?}", oauth.scopes);
        }

        println!();
    }

    // Get detailed info about Google source
    println!("\n=== Google Source Details ===\n");
    if let Some(google) = get_source_info("google") {
        for stream in &google.streams {
            println!("🔄 {} ({})", stream.display_name, stream.name);
            println!("   {}", stream.description);
            println!("   Table: {}", stream.table_name);
            println!("   Incremental: {}", stream.supports_incremental);
            println!("   Full Refresh: {}", stream.supports_full_refresh);
            println!("\n   Config Schema:");
            println!("   {}\n", serde_json::to_string_pretty(&stream.config_schema).unwrap());
            println!("   Example Config:");
            println!("   {}\n", serde_json::to_string_pretty(&stream.config_example).unwrap());
        }
    }

    // List all streams in table format
    println!("\n=== All Streams (Table Name Mapping) ===\n");
    let all_streams = list_all_streams();
    println!("{:<15} {:<15} {:<30}", "Source", "Stream", "Table Name");
    println!("{}", "-".repeat(60));

    for (source, stream) in all_streams {
        println!("{:<15} {:<15} {:<30}", source, stream.name, stream.table_name);
    }

    println!("\n✅ Catalog loaded successfully!");
}
