//! CLI argument types and command structures

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ariata")]
#[command(version, about = "Ariata personal data platform CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

    /// Sync all streams for a source
    Sync {
        /// Source ID (UUID)
        source_id: String,
    },
}

#[derive(Subcommand)]
pub enum CatalogCommands {
    /// List all available sources
    Sources,

    /// Show details about an available source
    Source {
        /// Source name (e.g., google, notion)
        name: String,
    },

    /// List all available streams across all sources
    Streams,
}

#[derive(Subcommand)]
pub enum SourceCommands {
    /// List all configured sources
    List {
        /// Show only pending device pairings
        #[arg(long)]
        pending: bool,
    },

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
pub enum StreamCommands {
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
