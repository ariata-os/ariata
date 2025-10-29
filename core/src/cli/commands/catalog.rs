//! Catalog command handlers - browse available sources and streams

use crate::cli::types::CatalogCommands;

/// Handle catalog/registry browsing commands
pub fn handle_catalog_command(
    action: Option<CatalogCommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None | Some(CatalogCommands::Sources) => {
            // List all available sources from registry
            let sources = crate::list_available_sources();

            println!("Available Data Sources:");
            println!("{:<15} {:<30} {}", "Type", "Name", "Auth");
            println!("{}", "-".repeat(60));

            for source in sources {
                let auth_type = match source.auth_type {
                    crate::registry::AuthType::OAuth2 => "OAuth2",
                    crate::registry::AuthType::Device => "Device",
                    crate::registry::AuthType::ApiKey => "API Key",
                    crate::registry::AuthType::None => "None",
                };
                println!(
                    "{:<15} {:<30} {}",
                    source.name, source.display_name, auth_type
                );
            }

            println!();
            println!("Use 'ariata catalog source <type>' for details about streams");
        }

        Some(CatalogCommands::Source { name }) => {
            let info = crate::get_source_info(&name)
                .ok_or_else(|| format!("Source '{}' not found", name))?;

            println!("Source: {}", info.display_name);
            println!("Type: {}", info.name);
            println!("Description: {}", info.description);
            println!();
            println!("Authentication: {:?}", info.auth_type);

            if let Some(oauth_config) = &info.oauth_config {
                println!("OAuth Scopes: {}", oauth_config.scopes.join(", "));
            }

            println!();
            println!("Available Streams:");
            println!("{:<20} {:<40} {}", "Stream", "Description", "Sync Modes");
            println!("{}", "-".repeat(80));

            for stream in &info.streams {
                let mut modes = Vec::new();
                if stream.supports_incremental {
                    modes.push("incremental");
                }
                if stream.supports_full_refresh {
                    modes.push("full");
                }
                let modes_str = modes.join(", ");

                println!(
                    "{:<20} {:<40} {}",
                    stream.name, stream.description, modes_str
                );
            }
        }

        Some(CatalogCommands::Streams) => {
            // List all streams across all sources
            let streams = crate::list_all_streams();

            println!("Available Streams:\n");
            println!("{:<15} {:<15} {:<45} {}", "Source", "Stream", "Description", "Sync Modes");
            println!("{}", "─".repeat(90));

            for (source_name, stream) in streams {
                let mut modes = Vec::new();
                if stream.supports_incremental {
                    modes.push("incremental");
                }
                if stream.supports_full_refresh {
                    modes.push("full");
                }
                let modes_str = modes.join(", ");

                println!(
                    "{:<15} {:<15} {:<45} {}",
                    source_name,
                    stream.name,
                    stream.description,
                    modes_str
                );
            }

            println!();
            println!("Use 'ariata catalog source <type>' for details about a specific source");
            println!("Use 'ariata add <type>' to connect a source");
        }
    }

    Ok(())
}
