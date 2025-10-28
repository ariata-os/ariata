//! Ariata CLI - Command-line interface for the Ariata personal data platform

use ariata::{Ariata, AriataBuilder};
use clap::{Parser, Subcommand};
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
    /// Interactive setup wizard
    Init,

    /// Run database migrations
    Migrate,

    /// Start the HTTP server
    Server {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "8000")]
        port: u16,
    },

    /// Browse available sources and streams (catalog)
    Catalog {
        #[command(subcommand)]
        action: Option<CatalogCommands>,
    },

    /// Manage data sources
    Source {
        #[command(subcommand)]
        action: SourceCommands,
    },

    /// Manage streams for a source
    Stream {
        #[command(subcommand)]
        action: StreamCommands,
    },

    /// Add a new source (OAuth or device)
    Add {
        /// Source type (google, notion, ios, mac, etc.)
        source_type: String,

        /// Device ID (required for device sources like ios, mac)
        #[arg(long)]
        device_id: Option<String>,

        /// Device name (required for device sources)
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum CatalogCommands {
    /// List all available sources
    Sources,

    /// Show details about an available source
    Source {
        /// Source name (e.g., google, notion)
        name: String,
    },
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

#[derive(Subcommand)]
enum StreamCommands {
    /// List all streams for a source
    List {
        /// Source ID (UUID)
        source_id: String,
    },

    /// Show details about a specific stream
    Show {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,
    },

    /// Enable a stream
    Enable {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,
    },

    /// Disable a stream
    Disable {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,
    },

    /// Set cron schedule for a stream
    Schedule {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,

        /// Cron expression (e.g., "0 */6 * * *")
        #[arg(long)]
        cron: Option<String>,
    },

    /// Show sync history for a specific stream
    History {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,

        /// Number of recent syncs to show
        #[arg(long, default_value = "10")]
        limit: i64,
    },

    /// Trigger a manual sync for a specific stream
    Sync {
        /// Source ID (UUID)
        source_id: String,

        /// Stream name (e.g., calendar, gmail)
        stream_name: String,
    },
}

async fn handle_add_source(
    ariata: Ariata,
    source_type: &str,
    device_id: Option<String>,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Adding {} source...", source_type);

    // Check if this is a device source
    let descriptor = ariata::get_source_info(source_type);

    let is_device_source = descriptor
        .map(|d| matches!(d.auth_type, ariata::registry::AuthType::Device))
        .unwrap_or(false);

    if is_device_source {
        // Handle device registration
        let device_id =
            device_id.ok_or_else(|| "device_id is required for device sources".to_string())?;
        let name = name.ok_or_else(|| "name is required for device sources".to_string())?;

        println!("Registering device: {} ({})...", name, source_type);

        let request = ariata::RegisterDeviceRequest {
            device_type: source_type.to_string(),
            device_id,
            name,
        };

        let source = ariata::register_device(ariata.database.pool(), request).await?;

        println!("\n✅ Device registered successfully!");
        println!("   Name: {}", source.name);
        println!("   ID: {}", source.id);

        // List available streams
        let streams = ariata::list_source_streams(ariata.database.pool(), source.id).await?;
        if !streams.is_empty() {
            println!("\n📊 Available streams (all disabled by default):");
            for stream in streams {
                println!(
                    "   - {} ({})",
                    stream.stream_name,
                    if stream.is_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            println!(
                "\n💡 Enable streams with: ariata stream enable {} <stream_name>",
                source.id
            );
        }

        return Ok(());
    }

    // Handle OAuth flow
    let redirect_uri = "http://localhost:8080";
    let response = ariata::initiate_oauth_flow(source_type, Some(redirect_uri.to_string()), None)
        .await
        .map_err(|e| format!("Failed to initiate OAuth flow: {e}"))?;

    println!("\n🌐 Please visit the following URL to authorize:");
    println!("{}", response.authorization_url);
    println!("\nPress Enter after you've authorized and been redirected...");

    // Open browser automatically
    #[cfg(not(target_os = "windows"))]
    std::process::Command::new("open")
        .arg(&response.authorization_url)
        .spawn()
        .ok();
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(&["/C", "start", &response.authorization_url])
        .spawn()
        .ok();

    // Wait for user input
    use std::io::{self, Write};
    io::stdout().flush()?;
    let mut _input = String::new();
    io::stdin().read_line(&mut _input)?;

    // Get the authorization code from the redirect URL
    println!("\n📋 Please paste the full redirect URL here:");
    io::stdout().flush()?;
    let mut redirect_url = String::new();
    io::stdin().read_line(&mut redirect_url)?;

    // Parse callback URL parameters
    let callback_params = parse_callback_url(&redirect_url, source_type)?;

    // Handle callback and create source
    let source = ariata::handle_oauth_callback(ariata.database.pool(), &callback_params).await?;

    println!("\n✅ Source created successfully!");
    println!("   Name: {}", source.name);
    println!("   ID: {}", source.id);

    // List available streams
    let streams = ariata::list_source_streams(ariata.database.pool(), source.id).await?;
    if !streams.is_empty() {
        println!("\n📊 Available streams (all disabled by default):");
        for stream in streams {
            println!(
                "   - {} ({})",
                stream.stream_name,
                if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        println!(
            "\n💡 Enable streams with: ariata stream enable {} <stream_name>",
            source.id
        );
    }

    Ok(())
}

fn parse_callback_url(
    url: &str,
    provider: &str,
) -> Result<ariata::OAuthCallbackParams, Box<dyn std::error::Error>> {
    use oauth2::url::Url;

    // Trim whitespace (including newline from stdin)
    let url = url.trim();

    let parsed_url = Url::parse(url)?;
    let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().collect();

    // Extract common OAuth parameters
    let code = params.get("code").map(|s| s.to_string());
    let access_token = params.get("access_token").map(|s| s.to_string());
    let refresh_token = params.get("refresh_token").map(|s| s.to_string());
    let expires_in = params
        .get("expires_in")
        .and_then(|s| s.parse::<i64>().ok());
    let state = params.get("state").map(|s| s.to_string());

    // Notion-specific fields
    let workspace_id = params.get("workspace_id").map(|s| s.to_string());
    let workspace_name = params.get("workspace_name").map(|s| s.to_string());
    let bot_id = params.get("bot_id").map(|s| s.to_string());

    // Validate that we have either code or access_token
    if code.is_none() && access_token.is_none() {
        return Err("OAuth callback URL must contain either 'code' or 'access_token' parameter".into());
    }

    Ok(ariata::OAuthCallbackParams {
        code,
        access_token,
        refresh_token,
        expires_in,
        provider: provider.to_string(),
        state,
        workspace_id,
        workspace_name,
        bot_id,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    let _ = dotenv::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Handle Init command early (doesn't need Ariata client)
    if matches!(cli.command, Commands::Init) {
        let config = ariata::setup::run_init().await?;

        // Save configuration
        ariata::setup::save_config(&config)?;

        // Run migrations if requested
        if config.run_migrations {
            println!();
            println!("📊 Running migrations...");
            let db = ariata::database::Database::new(&config.database_url)?;
            db.initialize().await?;
            ariata::setup::validation::display_success("Migrations complete");
        }

        ariata::setup::display_completion();
        return Ok(());
    }

    // Get database URL from environment
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/ariata".to_string());

    // Initialize Ariata client
    let ariata = AriataBuilder::new().postgres(&database_url).build().await?;

    match cli.command {
        Commands::Init => unreachable!(), // Handled above

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

        Commands::Catalog { action } => {
            handle_catalog_command(action)?;
        }

        Commands::Source { action } => {
            handle_source_command(ariata, action).await?;
        }

        Commands::Stream { action } => {
            handle_stream_command(ariata, action).await?;
        }

        Commands::Add {
            source_type,
            device_id,
            name,
        } => {
            handle_add_source(ariata, &source_type, device_id, name).await?;
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
            println!(
                "  Status: {}",
                if source.is_active {
                    "active"
                } else {
                    "inactive"
                }
            );

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
            println!(
                "{:<26} {:<10} {:<10} {:<10} {}",
                "Started", "Status", "Records", "Duration", "Error"
            );
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

fn handle_catalog_command(
    action: Option<CatalogCommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None | Some(CatalogCommands::Sources) => {
            // List all available sources from registry
            let sources = ariata::list_available_sources();

            println!("Available Data Sources:");
            println!("{:<15} {:<30} {}", "Type", "Name", "Auth");
            println!("{}", "-".repeat(60));

            for source in sources {
                let auth_type = match source.auth_type {
                    ariata::registry::AuthType::OAuth2 => "OAuth2",
                    ariata::registry::AuthType::Device => "Device",
                    ariata::registry::AuthType::ApiKey => "API Key",
                    ariata::registry::AuthType::None => "None",
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
            let info = ariata::get_source_info(&name)
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
    }

    Ok(())
}

async fn handle_stream_command(
    ariata: Ariata,
    action: StreamCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        StreamCommands::List { source_id } => {
            let source_id = source_id.parse()?;
            let streams = ariata::list_source_streams(ariata.database.pool(), source_id).await?;

            if streams.is_empty() {
                println!("No streams found for this source");
                return Ok(());
            }

            println!("Streams for source {}:", source_id);
            println!(
                "{:<20} {:<10} {:<25} {}",
                "Stream", "Status", "Schedule", "Last Sync"
            );
            println!("{}", "-".repeat(80));

            for stream in streams {
                let status = if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                };
                let schedule = stream.cron_schedule.unwrap_or_else(|| "manual".to_string());
                let last_sync = stream
                    .last_sync_at
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "never".to_string());

                println!(
                    "{:<20} {:<10} {:<25} {}",
                    stream.stream_name, status, schedule, last_sync
                );
            }
        }

        StreamCommands::Show {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;
            let stream =
                ariata::get_stream_info(ariata.database.pool(), source_id, &stream_name).await?;

            println!("Stream: {} / {}", source_id, stream.stream_name);
            println!("  Table: {}", stream.table_name);
            println!(
                "  Status: {}",
                if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );

            if let Some(schedule) = stream.cron_schedule {
                println!("  Schedule: {}", schedule);
            } else {
                println!("  Schedule: manual");
            }

            if let Some(last_sync) = stream.last_sync_at {
                println!("  Last Sync: {}", last_sync);
            } else {
                println!("  Last Sync: never");
            }

            // Show config if it's not an empty object
            if let serde_json::Value::Object(map) = &stream.config {
                if !map.is_empty() {
                    println!(
                        "  Config: {}",
                        serde_json::to_string_pretty(&stream.config)?
                    );
                }
            }
        }

        StreamCommands::Enable {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;

            println!("Enabling stream: {} / {}", source_id, stream_name);

            // Enable with default config (None = use defaults)
            ariata::enable_stream(ariata.database.pool(), source_id, &stream_name, None).await?;

            println!("✅ Stream enabled successfully");
        }

        StreamCommands::Disable {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;

            println!("Disabling stream: {} / {}", source_id, stream_name);

            ariata::disable_stream(ariata.database.pool(), source_id, &stream_name).await?;

            println!("✅ Stream disabled successfully");
        }

        StreamCommands::Schedule {
            source_id,
            stream_name,
            cron,
        } => {
            let source_id = source_id.parse()?;

            if let Some(cron_schedule) = cron {
                println!(
                    "Setting schedule for {} / {}: {}",
                    source_id, stream_name, cron_schedule
                );
                ariata::update_stream_schedule(
                    ariata.database.pool(),
                    source_id,
                    &stream_name,
                    Some(cron_schedule),
                )
                .await?;
                println!("✅ Schedule updated successfully");
            } else {
                println!("Clearing schedule for {} / {}", source_id, stream_name);
                ariata::update_stream_schedule(
                    ariata.database.pool(),
                    source_id,
                    &stream_name,
                    None,
                )
                .await?;
                println!("✅ Schedule cleared (stream will be manual only)");
            }
        }

        StreamCommands::Sync {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;

            println!("Syncing stream: {} / {}...", source_id, stream_name);

            let log =
                ariata::sync_stream(ariata.database.pool(), source_id, &stream_name, None).await?;

            println!("\n✅ Sync completed!");
            println!("  Status: {}", log.status);
            println!("  Records fetched: {}", log.records_fetched.unwrap_or(0));
            println!("  Records written: {}", log.records_written.unwrap_or(0));
            if let Some(duration) = log.duration_ms {
                println!("  Duration: {}ms", duration);
            }
            if let Some(error) = log.error_message {
                println!("  Error: {}", error);
            }
        }

        StreamCommands::History {
            source_id,
            stream_name,
            limit,
        } => {
            let source_id = source_id.parse()?;

            let logs = ariata::get_stream_sync_history(
                ariata.database.pool(),
                source_id,
                &stream_name,
                limit,
            )
            .await?;

            if logs.is_empty() {
                println!("No sync history found for this stream");
                return Ok(());
            }

            println!(
                "Sync History for {} / {} (showing {} most recent):",
                source_id,
                stream_name,
                logs.len()
            );
            println!(
                "{:<26} {:<10} {:<10} {:<10} {}",
                "Started", "Status", "Records", "Duration", "Error"
            );
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
