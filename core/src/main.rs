//! Ariata CLI - Command-line interface for the Ariata personal data platform

use clap::{Parser, Subcommand};
use ariata::{Ariata, AriataBuilder};
use std::env;

#[derive(Parser)]
#[command(name = "ariata")]
#[command(version, about = "Ariata personal data platform CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run database migrations
    Migrate,

    /// Start the HTTP server
    Server {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Manage data sources
    Source {
        #[command(subcommand)]
        action: SourceCommands,
    },

    /// Start the scheduler for periodic syncs
    Scheduler,
}

#[derive(Subcommand)]
enum SourceCommands {
    /// List all configured sources
    List,

    /// Show details about a source
    Show {
        /// Source ID (UUID)
        id: String,
    },

    /// Get source status with sync statistics
    Status {
        /// Source ID (UUID)
        id: String,
    },

    /// Trigger a manual sync for a source
    Sync {
        /// Source ID (UUID)
        id: String,
    },

    /// Delete a source
    Delete {
        /// Source ID (UUID)
        id: String,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Show sync history for a source
    History {
        /// Source ID (UUID)
        id: String,

        /// Number of recent syncs to show
        #[arg(long, default_value = "10")]
        limit: i64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/ariata".to_string());

    // Initialize Ariata client
    let ariata = AriataBuilder::new()
        .postgres(&database_url)
        .build()
        .await?;

    match cli.command {
        Commands::Migrate => {
            println!("Running database migrations...");
            ariata.database.initialize().await?;
            println!("✅ Migrations completed successfully");
        }

        Commands::Server { host, port } => {
            println!("Starting Ariata server on {}:{}", host, port);
            println!("API available at http://{}:{}/api", host, port);
            println!("Health check: http://{}:{}/health", host, port);
            println!();
            println!("Press Ctrl+C to stop");

            ariata::server::run(ariata, &host, port).await?;
        }

        Commands::Source { action } => {
            handle_source_command(ariata, action).await?;
        }

        Commands::Scheduler => {
            use std::sync::Arc;

            println!("Starting scheduler...");
            println!();

            // Create OAuth manager for token management
            let oauth = Arc::new(ariata::OAuthManager::new(ariata.database.clone()));

            // Create and start scheduler
            let scheduler = ariata::Scheduler::new(ariata.database.clone(), oauth).await?;

            // Start the scheduler (loads schedules from database)
            scheduler.start().await?;

            println!("✅ Scheduler started successfully");
            println!("   - Periodic syncs are now active");
            println!("   - Token refresh every 30 minutes");
            println!("   - Daily cleanup at 2:00 AM");
            println!();
            println!("Press Ctrl+C to stop");

            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;

            println!();
            println!("Shutting down scheduler...");
            scheduler.stop().await?;
            println!("✅ Scheduler stopped");
        }
    }

    Ok(())
}

async fn handle_source_command(
    ariata: Ariata,
    action: SourceCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SourceCommands::List => {
            let sources = ariata::list_sources(ariata.database.pool()).await?;

            if sources.is_empty() {
                println!("No sources configured");
                return Ok(());
            }

            println!("Configured Sources:");
            println!("{:<38} {:<20} {:<15} {}", "ID", "Name", "Type", "Status");
            println!("{}", "-".repeat(80));

            for source in sources {
                let status = if source.is_active {
                    "active"
                } else {
                    "inactive"
                };
                println!(
                    "{} {:<20} {:<15} {}",
                    source.id, source.name, source.source_type, status
                );
            }
        }

        SourceCommands::Show { id } => {
            let source_id = id.parse()?;
            let source = ariata::get_source(ariata.database.pool(), source_id).await?;

            println!("Source Details:");
            println!("  ID: {}", source.id);
            println!("  Name: {}", source.name);
            println!("  Type: {}", source.source_type);
            println!("  Status: {}", if source.is_active { "active" } else { "inactive" });

            if let Some(last_sync) = source.last_sync_at {
                println!("  Last Sync: {}", last_sync);
            } else {
                println!("  Last Sync: never");
            }

            if let Some(error) = source.error_message {
                println!("  Error: {}", error);
            }

            println!("  Created: {}", source.created_at);
            println!("  Updated: {}", source.updated_at);
        }

        SourceCommands::Status { id } => {
            let source_id = id.parse()?;
            let status = ariata::get_source_status(ariata.database.pool(), source_id).await?;

            println!("Source: {} ({})", status.name, status.source_type);
            println!();
            println!("Sync Statistics:");
            println!("  Total Syncs: {}", status.total_syncs);
            println!("  Successful: {}", status.successful_syncs);
            println!("  Failed: {}", status.failed_syncs);

            if status.total_syncs > 0 {
                let success_rate =
                    (status.successful_syncs as f64 / status.total_syncs as f64) * 100.0;
                println!("  Success Rate: {:.1}%", success_rate);
            }

            if let Some(last_status) = status.last_sync_status {
                println!("  Last Sync Status: {}", last_status);
            }

            if let Some(duration) = status.last_sync_duration_ms {
                println!("  Last Sync Duration: {}ms", duration);
            }
        }

        SourceCommands::Sync { id } => {
            let source_id = id.parse()?;

            println!("Triggering sync for source {}...", id);

            match ariata::sync_source(ariata.database.pool(), source_id).await {
                Ok(_) => {
                    println!("✅ Sync completed successfully");
                }
                Err(e) => {
                    eprintln!("❌ Sync failed: {}", e);
                    eprintln!();
                    eprintln!("Note: Direct sync triggering is not yet implemented.");
                    eprintln!("      Use the scheduler for automatic periodic syncs:");
                    eprintln!("      ariata scheduler");
                }
            }
        }

        SourceCommands::Delete { id, yes } => {
            let source_id = id.parse()?;

            // Get source details first
            let source = ariata::get_source(ariata.database.pool(), source_id).await?;

            if !yes {
                println!("Are you sure you want to delete source:");
                println!("  Name: {}", source.name);
                println!("  Type: {}", source.source_type);
                println!("  ID: {}", source.id);
                println!();
                println!("This will delete ALL data associated with this source!");
                println!();
                print!("Type 'yes' to confirm: ");

                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().to_lowercase() != "yes" {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            ariata::delete_source(ariata.database.pool(), source_id).await?;
            println!("✅ Source deleted successfully");
        }

        SourceCommands::History { id, limit } => {
            let source_id = id.parse()?;

            let logs = ariata::get_sync_history(ariata.database.pool(), source_id, limit).await?;

            if logs.is_empty() {
                println!("No sync history found for this source");
                return Ok(());
            }

            println!("Sync History (showing {} most recent):", logs.len());
            println!("{:<26} {:<10} {:<10} {:<10} {}", "Started", "Status", "Records", "Duration", "Error");
            println!("{}", "-".repeat(80));

            for log in logs {
                let records = log.records_written.unwrap_or(0);
                let duration = log
                    .duration_ms
                    .map(|d| format!("{}ms", d))
                    .unwrap_or_else(|| "-".to_string());
                let error = log
                    .error_message
                    .map(|e| {
                        if e.len() > 30 {
                            format!("{}...", &e[..27])
                        } else {
                            e
                        }
                    })
                    .unwrap_or_else(|| "-".to_string());

                println!(
                    "{} {:<10} {:<10} {:<10} {}",
                    log.started_at.format("%Y-%m-%d %H:%M:%S"),
                    log.status,
                    records,
                    duration,
                    error
                );
            }
        }
    }

    Ok(())
}
