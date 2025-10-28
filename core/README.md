# Ariata Core

High-performance Rust library and CLI for personal data collection, storage, and analysis.

## Quick Start

### Installation

```bash
cargo install --path .
```

### Setup

Run the interactive setup wizard:

```bash
ariata init
```

This will guide you through:
1. **Database configuration** - PostgreSQL connection string
2. **Migrations** - Automatically set up database schema
3. **Encryption** - Generate a key for securing OAuth tokens
4. **Storage** - Choose between local filesystem or S3/MinIO

The wizard will create a `.env` file with your configuration.

### Manual Setup

If you prefer to configure manually, create a `.env` file:

```bash
# Minimum required configuration
DATABASE_URL=postgresql://postgres:postgres@localhost/ariata
STORAGE_PATH=./data
ARIATA_ENCRYPTION_KEY=your_generated_32_byte_key_here
```

Then run migrations:

```bash
ariata migrate
```

## Prerequisites

- **PostgreSQL** - Install locally or use Docker:
  ```bash
  # Using Docker
  docker run -d \
    --name ariata-postgres \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=ariata \
    -p 5432:5432 \
    postgres:16
  ```

- **S3/MinIO (optional)** - Only needed for certain data sources (iOS microphone, etc.):
  ```bash
  # Using Docker
  docker run -d \
    --name ariata-minio \
    -p 9000:9000 \
    -p 9001:9001 \
    -e MINIO_ROOT_USER=minioadmin \
    -e MINIO_ROOT_PASSWORD=minioadmin \
    minio/minio server /data --console-address ":9001"
  ```

## Usage

### Browse Available Integrations

```bash
ariata catalog sources
```

### Connect a Data Source

```bash
# OAuth sources (uses public oauth-proxy at auth.ariata.com)
ariata add google
ariata add notion
ariata add strava

# Device sources
ariata add ios --device-id YOUR_DEVICE_ID --name "My iPhone"
ariata add mac --device-id YOUR_MAC_ID --name "My MacBook"
```

### List Your Sources

```bash
ariata source list
```

### Enable Data Streams

```bash
# List available streams for a source
ariata stream list <source-id>

# Enable a specific stream
ariata stream enable <source-id> <stream-name>
```

### Start the HTTP Server

```bash
ariata server
```

API will be available at `http://localhost:8000/api`

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | - | PostgreSQL connection string |
| `STORAGE_PATH` | No | `./data` | Local storage directory |
| `S3_ENDPOINT` | No | - | S3/MinIO endpoint (overrides local storage) |
| `S3_BUCKET` | No | - | S3 bucket name |
| `S3_ACCESS_KEY` | No | - | S3 access key |
| `S3_SECRET_KEY` | No | - | S3 secret key |
| `ARIATA_ENCRYPTION_KEY` | Recommended | - | 32-byte key for token encryption |
| `OAUTH_PROXY_URL` | No | `https://auth.ariata.com` | OAuth proxy URL |

### OAuth Credentials

OAuth credentials are **not required** for the CLI. Ariata uses a public OAuth proxy at `auth.ariata.com` that handles the OAuth flow and returns tokens to your CLI.

The proxy only stores credentials temporarily during the OAuth flow and never persists them.

## Library Usage

You can also use Ariata as a Rust library:

```rust
use ariata::{AriataBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Build client
    let ariata = AriataBuilder::new()
        .postgres("postgresql://localhost/ariata")
        .storage_path("./data")
        .build()
        .await?;

    // Initialize (run migrations)
    ariata.initialize().await?;

    // Use the client
    let sources = ariata::list_sources(ariata.database.pool()).await?;
    println!("Found {} sources", sources.len());

    Ok(())
}
```

## Development

### Running Tests

```bash
cargo test
```

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release
```

## Supported Data Sources

- **Google** - Calendar, Gmail
- **Notion** - Workspaces, Pages
- **Strava** - Activities
- **iOS** - Health, Microphone, Screen Time (requires companion app)
- **macOS** - Screen Time (requires companion app)

## License

MIT
