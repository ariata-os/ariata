# Claude's Readme

Important instruction reminders:

- Do what has been asked; nothing more, nothing less.
- NEVER create files unless they're absolutely necessary for achieving your goal.
- ALWAYS prefer editing an existing file to creating a new one.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.

## Architecture Migration Status

**⚠️ IMPORTANT: We are migrating from Python to Rust**

The Ariata platform is undergoing a major architectural shift:
- **OLD**: Python-based ELT pipeline in `sources/` directory (being deprecated)
- **NEW**: High-performance Rust library in `core/` directory (active development)

When working on the codebase:
- **Prefer Rust** (`core/`) for all new data pipeline features
- **Python** (`sources/`) is legacy - only maintain existing code, do not add new features
- **TypeScript/SvelteKit** (`apps/web/`) remains the frontend
- **Swift** (`apps/ios/`, `apps/mac/`) for native device clients

## System Overview

Ariata is a personal data ELT platform for ingesting, storing, and analyzing data from multiple sources. This is a **single-user system** where all data belongs to one person.

### Data Model

The schema follows an ELT (Extract, Load, Transform) pipeline:

```
sources (configs)
  └── streams (time-series tables)
       └── stream_* tables (actual data)
```

**Core tables:**
- `sources` - Active source instances (e.g., "My iPhone", "Work Calendar")
- `streams` - Active stream instances with settings
- `stream_{source}_{stream}` - Time-series data tables (e.g., `stream_ios_healthkit`, `stream_google_calendar`)

### Naming Conventions

**All stream tables MUST follow the pattern: `stream_{source}_{stream}`**

Examples:
- ✅ `stream_ios_location` - Location data from iOS devices
- ✅ `stream_ios_healthkit` - Health metrics from iOS HealthKit
- ✅ `stream_google_calendar` - Calendar events from Google
- ✅ `stream_mac_apps` - Application usage from macOS
- ❌ `stream_location` - Missing source prefix (ambiguous)
- ❌ `stream_calendar` - Missing source prefix (ambiguous)

## Rust Core Library (`core/`)

The Rust core is the **single source of truth** for all data pipeline logic.

### Key Features

- **High-performance ELT**: Rust's performance for data processing
- **Type safety**: Using `thiserror` for library errors (not `anyhow`)
- **PostgreSQL integration**: `sqlx` for database operations with compile-time query checking
- **S3/MinIO storage**: AWS SDK for object storage
- **HTTP ingestion server**: `axum` for receiving data from devices
- **OAuth support**: Built-in OAuth2 flows for cloud sources
- **CLI tool**: `ariata` binary for management operations

### Error Handling

- **Library code** (in `core/src/`): Use `crate::error::Result<T>` with `thiserror`
- **Binary code** (in `core/src/main.rs`): Use `anyhow::Result`
- **Examples** (in `core/examples/`): Use `anyhow::Result`

Rationale: Libraries should expose typed errors for callers to handle, binaries can use generic errors.

### Directory Structure

```
core/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # CLI binary
│   ├── error.rs            # Typed error definitions (thiserror)
│   ├── client.rs           # Main Ariata client interface
│   ├── database/           # PostgreSQL operations
│   ├── storage/            # S3/MinIO operations
│   ├── server/             # HTTP ingestion server (axum)
│   │   ├── ingest.rs       # Data ingestion endpoint
│   │   └── oauth.rs        # OAuth callback handling
│   ├── sources/            # Source-specific implementations
│   │   ├── base/           # Shared traits and utilities
│   │   ├── google/         # Google Calendar, Gmail
│   │   ├── strava/         # Strava activities
│   │   └── notion/         # Notion pages
│   ├── streams/            # Stream processing logic
│   ├── pipeline/           # Data transformation pipeline
│   ├── scheduler/          # Background sync scheduling
│   └── oauth/              # OAuth manager
├── migrations/             # PostgreSQL migrations (numbered)
│   ├── 001_initial_schema.sql
│   ├── 003_oauth_and_scheduler.sql
│   ├── 004_fix_naming_conventions.sql
│   └── 005_auto_generated_streams.sql
├── examples/               # Usage examples
└── Cargo.toml              # Rust dependencies
```

### Running the Rust Core

```bash
# Build the library and binary
cd core && cargo build

# Run the CLI
cargo run -- --help

# Run the ingestion server
cargo run -- server --port 8000

# Check compilation without running
cargo check

# Run tests
cargo test
```

### Adding New Sources to Rust

When implementing a new source in Rust:

1. Create module in `core/src/sources/{source}/`
2. Define types in `types.rs`
3. Implement auth in `auth.rs`
4. Implement sync logic in `client.rs`
5. Create processor for transforming data
6. Register in `core/src/sources/mod.rs`

## Database

### Credentials (Development)

```
DB_USER=ariata_user
DB_PASSWORD=ariata_password
DB_NAME=ariata
DB_HOST=postgres
DB_PORT=5432
```

### Migrations

**Location**: `core/migrations/*.sql`

Migrations are **numbered** and run in order:
- `001_*.sql` - Core schema (sources, streams, stream_data)
- `003_*.sql` - OAuth and scheduler tables + initial stream tables
- `004_*.sql` - Fix legacy naming conventions
- `005_*.sql` - Auto-generated stream schemas from YAML

**Running migrations**:
```bash
# Using sqlx CLI
cd core && sqlx migrate run

# Or via the Rust CLI
cargo run -- migrate
```

### Schema Source of Truth

**YAML configs remain authoritative** for stream schemas during the migration period:

1. **Define schemas** in `sources/{source}/{stream}/_stream.yaml`
2. **Auto-generate** SQL migrations:
   ```bash
   python scripts/generate_sql_migrations.py
   ```
3. **Generated file**: `core/migrations/005_auto_generated_streams.sql`

Eventually, YAML will be replaced by Rust structs as source of truth.

## Applications

### Web Frontend (`apps/web/`)

**SvelteKit + TypeScript** application for the UI.

- **Database ORM**: Drizzle (TypeScript)
- **API Routes**: SvelteKit API routes (replacing FastAPI)
- **Schema Generation**: `pnpm db:generate` generates TypeScript types from database

Key commands:
```bash
cd apps/web

# Install dependencies
pnpm install

# Generate TypeScript schemas from DB
pnpm db:generate

# Start dev server
pnpm dev

# Build for production
pnpm build
```

### iOS Client (`apps/ios/`)

**Swift + SwiftUI** application for iPhone data collection.

- Collects: HealthKit, CoreLocation, Microphone audio
- Batches and compresses data locally
- Uploads to Rust ingestion server via HTTP

### macOS Agent (`apps/mac/`)

**Swift** background agent for Mac data collection.

- Collects: Application usage, Messages (future)
- Runs as LaunchAgent in background
- Uploads to Rust ingestion server

## Legacy Python (`sources/`)

**⚠️ DEPRECATED - Do not add new features here**

The `sources/` directory contains the legacy Python-based ELT pipeline. It is being phased out in favor of the Rust core.

Existing functionality:
- Python processors for each source/stream
- Celery-based background tasks
- YAML-based configuration system

**Migration path**: Reimplement each Python source in Rust (`core/src/sources/`)

## Development Workflow

### Making Schema Changes

1. **Edit YAML** config in `sources/{source}/{stream}/_stream.yaml`
2. **Generate SQL migration**: `python scripts/generate_sql_migrations.py`
3. **Run migration**: `cd core && sqlx migrate run`
4. **Generate TypeScript types**: `cd apps/web && pnpm db:generate`
5. **Update Rust structs** manually in `core/src/sources/{source}/types.rs`

### Building Everything

```bash
# From repo root
make build          # Build all projects
make test           # Run all tests
make dev            # Start all dev services (docker-compose)
```

### Docker Development

```bash
# Start all services (postgres, minio, redis, web)
docker compose up -d

# Run commands in containers
docker compose exec web pnpm db:generate
docker compose exec postgres psql -U ariata_user -d ariata

# View logs
docker compose logs -f web
```

## Technical Stack

### Backend
- **Rust** - High-performance ELT core library (`core/`)
- **PostgreSQL** - Primary data store
- **S3/MinIO** - Object storage for large binary data
- **sqlx** - Compile-time checked SQL queries
- **axum** - HTTP server for ingestion
- **tokio** - Async runtime

### Frontend
- **SvelteKit** - Web application framework
- **TypeScript** - Type-safe frontend code
- **Drizzle ORM** - Database ORM and migrations

### Native Clients
- **Swift/SwiftUI** - iOS and macOS applications
- **HealthKit** - iOS health data integration
- **CoreLocation** - GPS tracking

### Infrastructure
- **Docker Compose** - Local development environment
- **Redis** - Cache and background job queue (legacy Python)
- **Nginx** - Reverse proxy (production)

## Common Tasks

### Add a New Stream Table

1. Create YAML schema: `sources/{source}/{stream}/_stream.yaml`
2. Generate migration: `python scripts/generate_sql_migrations.py`
3. Apply migration: `cd core && sqlx migrate run`
4. Define Rust types: `core/src/sources/{source}/types.rs`
5. Implement processor: `core/src/sources/{source}/{stream}/processor.rs`

### Run the Ingestion Server

```bash
cd core
cargo run -- server --port 8000 --host 0.0.0.0
```

### Query Data Directly

```bash
docker compose exec postgres psql -U ariata_user -d ariata

# Example query
SELECT COUNT(*) FROM stream_ios_healthkit
WHERE heart_rate > 100
AND timestamp > NOW() - INTERVAL '1 day';
```

### Debug iOS App Connection

Check ingestion endpoint:
```bash
curl http://localhost:8000/health
curl -X POST http://localhost:8000/ingest \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test", "data": []}'
```

## Testing

```bash
# Rust tests
cd core && cargo test

# Web tests
cd apps/web && pnpm test

# E2E tests
cd tests && pytest
```
