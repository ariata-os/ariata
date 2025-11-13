![Ariata Cover](.github/images/cover2.png)

# Ariata

Personal data ELT platform. Extract data from Google, iOS, Mac, Notion, Strava → Store in PostgreSQL + MinIO → Query with SQL.

> **Status**: Active development, beware. Things will break.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da?logo=discord&logoColor=white)](https://discord.gg/sSQKzDWqgv)

## What It Is

Ariata is a single-user ELT pipeline for personal data:

- **Extract**: Pull from APIs (Google, Notion, Strava) and devices (iOS, Mac)
- **Load**: Store raw streams in PostgreSQL + MinIO with full fidelity
- **Transform**: Normalize into signals for cross-source analysis

Self-hosted, open source, Rust-based. Your data stays on your infrastructure.

## Why Monolithic Rust?

Unlike enterprise tools (Airbyte: Docker per source), Ariata uses **one Rust package** for all sources:

**Personal data is different:**

- **User experience first**: One person manages this (not a full-time ELT team). Must be maintain-less and simple.
- **Device coupling**: iOS/Mac apps need direct hardware access (HealthKit, Location, Microphone) that can't run in containers.
- **Real-time hot pipeline**: Sub-second latency for streaming data enables proactive personal AI (future).
- **Single-user**: No multi-tenancy overhead = simpler code, better performance.

**Extensibility**: Implement `DataSource` trait in `core/plugins/` for custom sources.

## Implementation Status

| Source | Stream | Status |
|--------|--------|--------|
| Google | Calendar | ✅ |
| Google | Gmail | ✅ |
| iOS | HealthKit | ✅ |
| iOS | Location | ✅ |
| iOS | Microphone | ✅ |
| Mac | Apps | ✅ |
| Mac | Browser | ✅ |
| Mac | iMessage | ✅ |
| Notion | Pages | ✅ |
| Strava | Activities | ✅ |

See [CLAUDE.md](CLAUDE.md) for full implementation details and architecture documentation.

## Quick Start

```bash
# Clone and setup
git clone https://github.com/ariata-os/ariata
cd ariata

# Start everything (infrastructure + migrations + servers)
make dev

# In separate terminals, run:
cd core && cargo run -- server       # Terminal 1: API server
cd apps/web && npm run dev           # Terminal 2: Web UI
```

Access: `http://localhost:5173` (web) | `http://localhost:8000` (API)

**First time setup**: `make dev` handles everything - Docker containers, database migrations, and SQLx cache generation.

## Architecture

```
Sources (Cloud: Google, Notion | Device: iOS, Mac)
   ↓
Streams (stream_ios_healthkit, stream_ios_location, stream_google_calendar, etc.)
   ↓
Storage (PostgreSQL: raw streams | MinIO: audio/video/blobs)
   ↓
Signals (normalized/aggregated streams for analysis)
   ↓
Query with SQL
```

**Extract:**

- **Source** = Data origin (e.g., iOS, Google, Notion)
- **Stream** = Raw, non-normalized data from a source (e.g., iOS → stream_ios_healthkit, stream_ios_location, stream_ios_microphone)

**Load:**

- **Storage** = PostgreSQL for structured streams, MinIO for large unstructured data (audio recordings, future video streams)

**Transform:**

- **Signals** = Normalized/aggregated streams for cross-source analysis

**Core**: Rust library with OAuth, schedulers, and device processors
**Clients**: iOS/Mac apps push real-time data to ingestion server

## Development

```bash
# Start development environment
make dev              # Starts Postgres + MinIO + runs migrations

# Run tests
make test-rust        # Rust tests
make test-web         # Web tests

# Database commands
make migrate          # Run migrations
make prepare          # Regenerate SQLx cache (after schema changes)
make db-reset         # Reset database (WARNING: deletes data)

# View all commands
make help
```

**Requirements**:

- Docker & Docker Compose (for infrastructure)
- Rust 1.70+ (for core server)
- Node.js 18+ (for web UI)
- Make (for build commands)

## iOS/Mac Development with ngrok

iOS apps require HTTPS connections for production use. For local development, use ngrok to expose your Rust backend via a secure HTTPS tunnel:

```bash
# Install ngrok (one-time setup)
brew install ngrok

# Sign up for free account and add your authtoken
ngrok config add-authtoken YOUR_TOKEN

# Start Rust server with ngrok tunnel
make core-ngrok
```

**Get your HTTPS URL:**
1. Open http://localhost:4040 (ngrok dashboard)
2. Copy the HTTPS URL (e.g., `https://abc123.ngrok-free.app`)
3. Use this URL in your iOS/Mac app settings

**Note:** Free ngrok URLs change each time you restart. For persistent URLs, upgrade to a paid ngrok plan.

**What this does:**
- Starts Rust backend on `localhost:8000`
- Creates ngrok tunnel with valid SSL certificate
- iOS/Mac apps can connect via HTTPS (resolves TLS errors)
- Web app continues using `localhost:8000` directly

## License

MIT (core library) + Elastic License v2 (ML modules)

See [LICENSE](LICENSE) for details.

---

<p align="center">
  <i>Your data. Your infrastructure. Your life.</i>
</p>
