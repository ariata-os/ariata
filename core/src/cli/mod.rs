//! CLI module - command-line interface for Ariata

pub mod commands;
pub mod display;
pub mod types;

use crate::Ariata;
use types::{Cli, Commands};

/// Run the CLI application
pub async fn run(cli: Cli, ariata: Ariata) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init => {
            // This command is handled in main.rs before the Ariata client is created
            unreachable!("Init command should be handled in main.rs");
        }

        Commands::Migrate => {
            println!("Running database migrations...");
            ariata.database.initialize().await?;
            println!("✅ Migrations completed successfully");
        }

        Commands::Catalog { action } => {
            commands::handle_catalog_command(action)?;
        }

        Commands::Add {
            source_type,
            device_id,
            name,
        } => {
            commands::handle_add_source(ariata, &source_type, device_id, name).await?;
        }

        Commands::Source { action } => {
            commands::handle_source_command(ariata, action).await?;
        }

        Commands::Stream { action } => {
            commands::handle_stream_command(ariata, action).await?;
        }

        Commands::Sync { source_id } => {
            let source_id = source_id.parse()?;

            println!("Syncing source: {}...", source_id);

            // Get all enabled streams for this source
            let streams = crate::list_source_streams(ariata.database.pool(), source_id).await?;
            let enabled_streams: Vec<_> = streams.iter().filter(|s| s.is_enabled).collect();

            if enabled_streams.is_empty() {
                println!("⚠️  No enabled streams for this source");
                println!("Enable streams with: ariata stream enable {} <stream_name>", source_id);
                return Ok(());
            }

            println!("Syncing {} enabled stream(s)...\n", enabled_streams.len());

            let sync_mode = crate::SyncMode::full_refresh();
            let mut jobs_created = 0;
            let mut failed_count = 0;

            for stream in enabled_streams {
                println!("  Creating sync job for {}...", stream.stream_name);

                match crate::api::jobs::trigger_stream_sync(
                    ariata.database.pool(),
                    source_id,
                    &stream.stream_name,
                    Some(sync_mode.clone())
                ).await {
                    Ok(response) => {
                        jobs_created += 1;
                        println!("    ✅ Job created: {} (status: {})", response.job_id, response.status);
                    }
                    Err(e) => {
                        failed_count += 1;
                        println!("    ❌ Error: {}", e);
                    }
                }
            }

            println!("\n📊 Sync Summary:");
            println!("  Jobs created: {}", jobs_created);
            if failed_count > 0 {
                println!("  Failed to create jobs: {}", failed_count);
            }
            println!("\nNote: Jobs are running in the background. Use 'ariata jobs list' to monitor progress.");
        }

        Commands::Server { host, port } => {
            println!("Starting Ariata server on {}:{}", host, port);
            println!("API available at http://{}:{}/api", host, port);
            println!("Health check: http://{}:{}/health", host, port);
            println!();
            println!("Press Ctrl+C to stop");

            crate::server::run(ariata, &host, port).await?;
        }
    }

    Ok(())
}
