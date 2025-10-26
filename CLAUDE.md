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

### Architecture Rationale: Why Monolithic Rust?

Ariata uses a **single Rust package** for all data sources, unlike enterprise tools (Airbyte) that use Docker containers per source. This is the correct architecture for personal data:

**Why personal data is different:**
- **Device coupling**: iOS/macOS apps require direct hardware access (HealthKit, Location, Microphone) that can't run in containers
- **Cross-stream correlation**: Features like "heart rate during meetings" require joining HealthKit + Calendar data in-process, not via IPC
- **Shared authentication**: OAuth tokens, device tokens, and sync checkpoints benefit from centralized management
- **Single-user system**: No multi-tenancy isolation needed—simpler code, better performance
- **Real-time ingestion**: Sub-second latency for streaming data from devices

**Extensibility via plugins:**
- Users can add custom sources by implementing the `DataSource` trait in `plugins/` directory
- No need to fork—plugins compile alongside core or load dynamically as `.dylib`
- See `core/examples/custom_source.rs` for template

**When to use modular (Airbyte-style) architecture:**
- ❌ Multi-tenancy (isolation) - Not needed for single-user
- ❌ Untrusted code - We control all sources
- ❌ Polyglot (multiple languages) - Rust is sufficient
- ❌ Cloud-scale - Runs on personal infrastructure

The monolithic approach is **intentional**, not a limitation.

### Key Features

- **High-performance ELT**: Rust's performance for data processing
- **Type safety**: Using `thiserror` for library errors (not `anyhow`)
- **PostgreSQL integration**: `sqlx` for database operations with compile-time query checking
- **S3/MinIO storage**: AWS SDK for object storage
- **HTTP ingestion server**: `axum` for receiving data from devices
- **OAuth composability**: Universal `OAuthHttpClient` + trait-based error handling for all providers
- **Observability**: Two-layer logging (tracing + database sync_logs) for production monitoring
- **CLI tool**: `ariata` binary for management operations

### OAuth Composability Architecture

The Rust core uses a **trait-based composability pattern** to minimize code duplication across OAuth providers:

**Universal HTTP Client** (`core/src/sources/base/oauth_client.rs`):
- Single `OAuthHttpClient` (~250 lines) handles all OAuth providers
- Automatic retry with exponential backoff (1s → 2s → 4s → 8s → 16s → 30s max)
- Token refresh on 401 errors
- Rate limit detection (429 status)
- Provider-specific error handling via `ErrorHandler` trait

**Provider-Specific Logic** via traits:
- `ErrorHandler` trait for classifying errors (auth, rate limit, sync token, server, client)
- Each provider implements custom error detection (e.g., Google returns both 400 AND 410 for invalid sync tokens)
- Example: `GoogleErrorHandler` checks status codes + response body patterns

**Benefits:**
- **45% code reduction**: GoogleClient went from 206 lines to 113 lines
- **30-minute onboarding**: Adding new OAuth providers (Notion, Strava) now takes ~30 min vs 4-6 hours
- **Consistent behavior**: All providers get retry, backoff, token refresh for free
- **Testability**: ErrorHandler trait makes provider-specific logic unit-testable

**File structure:**
```
core/src/sources/
├── base/
│   ├── oauth_client.rs     # Universal OAuth HTTP client
│   ├── error_handler.rs    # ErrorHandler trait + DefaultErrorHandler
│   └── sync_mode.rs        # SyncMode (FullRefresh/Incremental) + SyncResult
├── google/
│   ├── client.rs           # GoogleClient (delegates to OAuthHttpClient)
│   ├── error_handler.rs    # Google-specific error classification
│   └── calendar/mod.rs     # Calendar sync implementation
├── notion/...              # Same pattern
└── strava/...              # Same pattern
```

### Observability & Sync Logging

The Rust core implements **two-layer logging** for production observability:

**Layer 1: Application Logs** (via `tracing` library):
- Real-time structured logging during sync operations
- Instrumented with `#[tracing::instrument]` for automatic context propagation
- Fields: `source_id`, `sync_mode`, `calendar_id`, `records_fetched`, `duration_ms`
- Output to stdout/files/observability platforms (DataDog, CloudWatch, etc)
- Used for development debugging and real-time monitoring

**Layer 2: Database Sync Logs** (`sync_logs` table):
- Permanent audit trail of **every sync operation** across all sources
- Powers analytics: success rates, throughput trends, error patterns
- Required for compliance and debugging historical issues
- Matches enterprise ELT patterns (Airbyte, Fivetran, dbt)

**Schema** (`core/migrations/003_sync_logs.sql`):
```sql
CREATE TABLE sync_logs (
    id UUID PRIMARY KEY,
    source_id UUID REFERENCES sources(id),
    sync_mode TEXT,           -- 'full_refresh' or 'incremental'
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,
    status TEXT,              -- 'success', 'failed', 'partial'
    records_fetched INTEGER,
    records_written INTEGER,
    records_failed INTEGER,
    error_message TEXT,
    error_class TEXT,         -- 'auth_error', 'rate_limit', 'sync_token_error', etc
    sync_cursor_before TEXT,  -- Token/cursor at start
    sync_cursor_after TEXT,   -- Token/cursor for next sync
    created_at TIMESTAMPTZ
);
```

**Usage** (automatic via `SyncLogger`):
- Every `sync()` call logs to database on success/failure
- Error classification for monitoring dashboards
- Cursor tracking for incremental sync debugging
- Historical queries: "What was sync success rate last week?"

**Why both layers?**
- **Tracing**: Ephemeral, low-latency, detailed debugging
- **Database**: Permanent, queryable, audit trail + analytics

### Error Handling

- **Library code** (in `core/src/`): Use `crate::error::Result<T>` with `thiserror`
- **Binary code** (in `core/src/main.rs`): Use `anyhow::Result`
- **Examples** (in `core/examples/`): Use `anyhow::Result`

Rationale: Libraries should expose typed errors for callers to handle, binaries can use generic errors.

### Migrations

**Location**: `core/migrations/*.sql`

Migrations are **numbered** and run in order:

- `001_core.sql` - Core schema (sources table with OAuth tokens)
- `002_google_calendar.sql` - Google Calendar stream table
- `003_sync_logs.sql` - Sync logging for observability

**Running migrations**:

```bash
# Using sqlx CLI
cd core && sqlx migrate run

# Or via the Rust CLI
cargo run -- migrate
```
