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
