//! Ariata CLI - Command line interface for Ariata

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ariata::{Ariata, AriataBuilder};
use ariata::scheduler::SyncTask;
use ariata::sources::DataSource;

#[derive(Parser)]
#[command(name = "ariata")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Postgres connection string
    #[arg(long, env = "DATABASE_URL")]
    postgres: Option<String>,

    /// S3 bucket name
    #[arg(long, env = "S3_BUCKET")]
    s3_bucket: Option<String>,

    /// S3 endpoint (for MinIO)
    #[arg(long, env = "S3_ENDPOINT")]
    s3_endpoint: Option<String>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Ariata configuration
    Init {
        /// PostgreSQL connection string
        #[arg(long)]
        postgres: String,

        /// S3 bucket name (optional)
        #[arg(long)]
        s3_bucket: Option<String>,
    },

    /// Show Ariata status and health
    Status,

    /// Manage data sources
    #[command(subcommand)]
    Source(SourceCommands),

    /// Execute SQL query
    Query {
        /// SQL query to execute
        sql: String,

        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Run ingestion server
    Server {
        /// Port to listen on
        #[arg(long, default_value = "8000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Ingest data from stdin
    Ingest {
        /// Source name
        #[arg(long)]
        source: String,
    },

    /// Sync data from OAuth sources
    Sync {
        /// Source to sync (google_calendar, strava, notion, or all)
        source: String,
    },

    /// OAuth authentication commands
    #[command(subcommand)]
    OAuth(OAuthCommands),

    /// Scheduler daemon for continuous syncing
    Scheduler,
}

#[derive(Subcommand)]
enum OAuthCommands {
    /// Initiate OAuth flow for a provider
    Connect {
        /// Provider name (google, strava)
        provider: String,
    },

    /// Handle OAuth callback
    Callback {
        /// Provider name
        provider: String,
        /// Authorization code
        code: String,
    },

    /// List connected OAuth accounts
    List,

    /// Refresh OAuth token
    Refresh {
        /// Provider name
        provider: String,
    },
}

#[derive(Subcommand)]
enum SourceCommands {
    /// List available and active sources
    List,

    /// Add a new source
    Add {
        /// Source type
        source_type: String,

        /// Source name
        name: String,
    },

    /// Sync a source
    Sync {
        /// Source name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ariata=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { postgres, s3_bucket } => {
            cmd_init(postgres, s3_bucket).await?;
        }
        Commands::Status => {
            cmd_status(&cli).await?;
        }
        Commands::Source(ref source_cmd) => {
            match source_cmd {
                SourceCommands::List => cmd_source_list(&cli).await?,
                SourceCommands::Add { source_type, name } => {
                    cmd_source_add(&cli, &source_type, &name).await?
                }
                SourceCommands::Sync { name } => cmd_source_sync(&cli, &name).await?,
            }
        }
        Commands::Query { ref sql, ref format } => {
            cmd_query(&cli, &sql, &format).await?;
        }
        Commands::Server { port, ref host } => {
            cmd_server(&cli, port, &host).await?;
        }
        Commands::Ingest { ref source } => {
            cmd_ingest(&cli, &source).await?;
        }
        Commands::Sync { ref source } => {
            cmd_sync(&cli, &source).await?;
        }
        Commands::OAuth(ref oauth_cmd) => {
            match oauth_cmd {
                OAuthCommands::Connect { provider } => cmd_oauth_connect(&cli, &provider).await?,
                OAuthCommands::Callback { provider, code } => cmd_oauth_callback(&cli, &provider, &code).await?,
                OAuthCommands::List => cmd_oauth_list(&cli).await?,
                OAuthCommands::Refresh { provider } => cmd_oauth_refresh(&cli, &provider).await?,
            }
        }
        Commands::Scheduler => {
            cmd_scheduler(&cli).await?;
        }
    }

    Ok(())
}

async fn cmd_init(postgres: String, s3_bucket: Option<String>) -> Result<()> {
    println!("✓ Initializing Ariata configuration...");
    println!("  PostgreSQL: {}", postgres);
    if let Some(bucket) = &s3_bucket {
        println!("  S3 Bucket: {}", bucket);
    }

    // TODO: Save config to ~/.ariata/config.toml

    println!("\nNext steps:");
    println!("  1. Run 'ariata status' to verify connection");
    println!("  2. Run 'ariata source list' to see available sources");
    println!("  3. Run 'ariata source add <type>' to add a source");

    Ok(())
}

async fn cmd_status(cli: &Cli) -> Result<()> {
    let client = build_client(cli).await?;

    println!("\nAriata Status");
    println!("{}", "=".repeat(50));

    let status = client.status().await?;

    if status.is_healthy {
        println!("Overall:  ✅ healthy");
    } else {
        println!("Overall:  ❌ degraded");
    }

    println!("Database: ✅ {}", status.database_status);
    println!("Storage:  ✅ {}", status.storage_status);
    println!("Sources:  ℹ️  {} active", status.active_sources);

    Ok(())
}

async fn cmd_source_list(cli: &Cli) -> Result<()> {
    let client = build_client(cli).await?;

    println!("\nAvailable Source Types:");
    let available = client.list_available_sources().await?;
    for source in available {
        println!("  • {}", source);
    }

    println!("\nActive Sources:");
    let active = client.list_active_sources().await?;
    if active.is_empty() {
        println!("  No active sources. Use 'ariata source add' to add one.");
    } else {
        println!("{:<20} {:<15} {:<10}", "Name", "Type", "Status");
        println!("{}", "-".repeat(45));
        for source in active {
            println!("{:<20} {:<15} {:<10}", source.name, source.source_type, source.status);
        }
    }

    Ok(())
}

async fn cmd_source_add(cli: &Cli, source_type: &str, name: &str) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Adding {} source '{}'...", source_type, name);

    let source = client.add_source(name, source_type).await?;

    println!("✓ Source '{}' added successfully!", source.name);
    println!("  ID: {}", source.id);
    println!("  Type: {}", source.source_type);

    Ok(())
}

async fn cmd_source_sync(cli: &Cli, name: &str) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Syncing {}...", name);

    let result = client.sync_source(name).await?;

    println!("✓ Sync completed successfully!");
    println!("  Records synced: {}", result.records_synced);
    println!("  Duration: {:?}", result.duration);

    Ok(())
}

async fn cmd_query(cli: &Cli, sql: &str, format: &str) -> Result<()> {
    let client = build_client(cli).await?;

    let results = client.query(sql).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        "table" | _ => {
            // Simple table output
            if results.is_empty() {
                println!("No results");
            } else {
                // Print headers
                let headers: Vec<_> = results[0].keys().collect();
                println!("{}", headers.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\t"));
                println!("{}", "-".repeat(80));

                // Print rows (limit to 100)
                for (_i, row) in results.iter().take(100).enumerate() {
                    let values: Vec<_> = headers.iter()
                        .map(|h| row.get(*h).map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    println!("{}", values.join("\t"));
                }

                if results.len() > 100 {
                    println!("\nShowing first 100 of {} results", results.len());
                }
            }
        }
    }

    Ok(())
}

async fn cmd_server(cli: &Cli, port: u16, host: &str) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Starting Ariata server at http://{}:{}", host, port);

    client.run_server(host, port).await?;

    Ok(())
}

async fn cmd_ingest(cli: &Cli, source: &str) -> Result<()> {
    let client = build_client(cli).await?;

    // Read from stdin
    use std::io::Read;
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;

    let data: serde_json::Value = serde_json::from_str(&buffer)?;

    let result = client.ingest(source, data).await?;

    println!("✓ Ingested {} records", result.records_ingested);

    Ok(())
}

async fn cmd_sync(cli: &Cli, source: &str) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Starting sync for: {}", source);

    // Initialize OAuth manager and scheduler
    let oauth = std::sync::Arc::new(ariata::oauth::OAuthManager::new(client.database.clone()));
    let scheduler = ariata::scheduler::Scheduler::new(client.database.clone(), oauth.clone()).await?;

    match source {
        "google_calendar" | "google" => {
            let google = ariata::sources::google::calendar_source(oauth);
            let result = google.sync().await?;
            println!("✓ Synced {} records in {}ms", result.records_synced, result.duration_ms);
        }
        "strava" => {
            let strava = ariata::sources::strava::activities_source(oauth);
            let _result = strava.fetch(None).await?;
            println!("✓ Strava sync completed");
        }
        "notion" => {
            // Notion uses API token, not OAuth
            println!("Notion sync not yet implemented");
        }
        "all" => {
            println!("Syncing all sources...");
            // TODO: Sync all configured sources
        }
        _ => {
            println!("Unknown source: {}. Available: google_calendar, strava, notion, all", source);
        }
    }

    Ok(())
}

async fn cmd_oauth_connect(_cli: &Cli, provider: &str) -> Result<()> {
    println!("Initiating OAuth flow for: {}", provider);

    // TODO: Start OAuth flow
    println!("Visit the following URL to authorize:");
    println!("https://oauth.example.com/authorize?provider={}", provider);

    Ok(())
}

async fn cmd_oauth_callback(_cli: &Cli, provider: &str, code: &str) -> Result<()> {
    println!("Processing OAuth callback for: {}", provider);
    println!("Authorization code: {}...", &code[..8.min(code.len())]);

    // TODO: Exchange code for tokens

    Ok(())
}

async fn cmd_oauth_list(cli: &Cli) -> Result<()> {
    let client = build_client(cli).await?;

    println!("\nConnected OAuth accounts:");

    let query = "SELECT provider, updated_at FROM oauth_credentials ORDER BY provider";
    let results = client.query(query).await?;

    if results.is_empty() {
        println!("  No connected accounts");
    } else {
        for row in results {
            let provider = row.get("provider").and_then(|v| v.as_str()).unwrap_or("unknown");
            println!("  • {}", provider);
        }
    }

    Ok(())
}

async fn cmd_oauth_refresh(cli: &Cli, provider: &str) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Refreshing token for: {}", provider);

    let oauth = ariata::oauth::OAuthManager::new(client.database.clone());
    oauth.refresh_token(provider).await?;

    println!("✓ Token refreshed successfully");

    Ok(())
}

async fn cmd_scheduler(cli: &Cli) -> Result<()> {
    let client = build_client(cli).await?;

    println!("Starting Ariata scheduler...");
    println!("Press Ctrl+C to stop\n");

    // Initialize OAuth manager and scheduler
    let oauth = std::sync::Arc::new(ariata::oauth::OAuthManager::new(client.database.clone()));
    let scheduler = ariata::scheduler::Scheduler::new(client.database.clone(), oauth.clone()).await?;

    // Register tasks
    let google = Box::new(ariata::sources::google::calendar_source(oauth.clone()));
    scheduler.register_task(google).await;

    // Add schedules
    scheduler.add_schedule(ariata::scheduler::ScheduleConfig {
        source_name: "google_calendar".to_string(),
        cron_expression: "0 */30 * * * *".to_string(), // Every 30 minutes
        enabled: true,
        last_run: None,
        next_run: None,
    }).await?;

    // Start scheduler
    scheduler.start().await?;

    // Wait for interrupt
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down scheduler...");
    scheduler.stop().await?;

    Ok(())
}

async fn build_client(cli: &Cli) -> Result<Ariata> {
    let mut builder = AriataBuilder::new();

    if let Some(postgres) = &cli.postgres {
        builder = builder.postgres(postgres);
    }

    if let Some(bucket) = &cli.s3_bucket {
        builder = builder.s3_bucket(bucket);

        if let Some(endpoint) = &cli.s3_endpoint {
            builder = builder.s3_endpoint(endpoint);
        }
    }

    Ok(builder.build().await?)
}