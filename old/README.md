![Ariata Cover](.github/images/cover2.png)

<p align="center">
    <b>Ariata - the open source, personal ecosystem. <br/>
    A protocol for ingestion and management of personal data.
</p>

> [!WARNING]
> **Experimental Phase**: Expect rapid iteration and sweeping changes. We are currently migrating our work to a core python library for managing the ETL/ELT of personal data.

[![Release](https://img.shields.io/badge/Release-None-red.svg)](https://github.com/ariata-os/ariata/releases)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da?logo=discord&logoColor=white)](https://discord.gg/sSQKzDWqgv)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: ELv2](https://img.shields.io/badge/License-ELv2-orange.svg)](https://www.elastic.co/licensing/elastic-license)

## What is Ariata

Ariata is a comprehensive health intelligence platform that combines biometric data from Apple Health with continuous location tracking to reveal how your environment, movements, and daily activities impact your wellbeing. Unlike cloud services that monetize your data, Ariata runs on your infrastructure, ensuring complete privacy and control.

<https://github.com/user-attachments/assets/50d037b5-7e06-49b2-ad5a-69d14fb079af>

### Health & Wellness Intelligence

Ariata correlates your health data with real-world context to provide insights impossible with traditional fitness trackers:

- **Heart rate and HRV analysis** across different locations and activities
- **Sleep quality tracking** correlated with environment and movement patterns
- **Stress pattern recognition** by analyzing biometric responses to locations
- **Activity optimization** understanding how movement affects your health metrics
- **Environmental health factors** like noise levels and location patterns
- **Recovery analysis** based on where and how you spend your time

### Your Data, Your Control

Your health data is incredibly valuable—companies build empires on it. Ariata lets you own and analyze your health data:

- **Complete privacy:** Runs on YOUR infrastructure, not cloud servers
- **Direct database access:** Query your health data with SQL
- **Open source transparency:** Audit every line of code
- **No data monetization:** We don't sell or analyze your data
- **HealthKit integration:** Deep integration with Apple Health ecosystem

## Your Data, Your Database

Unlike cloud services that lock away your data, Ariata gives you **direct PostgreSQL access**. Query your life with SQL, build custom analytics, or export everything—it's your database.

```python
# Connect directly to YOUR data
import psycopg2
import pandas as pd

conn = psycopg2.connect(
    "postgresql://readonly_user:secure_pass@your-server:5432/ariata"
)

# Query your heart rate during meetings
df = pd.read_sql("""
    SELECT h.timestamp, h.heart_rate as bpm, c.summary as meeting
    FROM stream_ios_healthkit h
    JOIN stream_google_calendar c
        ON h.timestamp BETWEEN c.start_time AND c.end_time
    WHERE h.heart_rate IS NOT NULL
""", conn)
```

**Manage credentials** at `/settings/database` in your Ariata UI—create read-only users for analysis or full access for integrations. Works with any PostgreSQL client: TablePlus, DBeaver, Jupyter notebooks, or your favorite BI tool.

## ✨ Features

### Data Sources

See the [Implementation Status](#-implementation-status) section below for detailed availability of all sources and streams.

### Architecture

```txt
Sources → Streams → Timeline
```

- **Sources**: External services and devices (Google, iOS, Mac, etc.)
- **Streams**: Time-series data tables with full fidelity storage
- **Timeline**: Your queryable life history aggregated from all streams

## Status

### Implementation Overview

| Source | Stream | Status | Description |
|--------|--------|--------|-------------|
| Google | Calendar | ✅ | Calendar events and meetings |
| Google | Gmail | 📋 | Email messages and attachments |
| Google | Drive | 📋 | Document edits and shared files |
| iOS | HealthKit | ✅ | Health metrics (heart rate, steps, sleep, workouts, HRV) |
| iOS | Location | ✅ | GPS coordinates, speed, and altitude |
| iOS | Microphone | ✅ | Audio levels and transcription |
| Mac | Applications | ✅ | App usage and focus tracking |
| Mac | iMessage | 📋 | Messages and attachments |
| Mac | Browser | 📋 | History, bookmarks, and downloads |
| Notion | Pages | ✅ | Page and database content |
| Amazon | Orders | 📋 | Purchase history and delivery tracking |
| WhatsApp | Messages | 📋 | Conversations and voice notes |
| LinkedIn | Profile | 📋 | Profile views and messages |
| X (Twitter) | Posts | 📋 | Tweets and engagement metrics |
| Spotify | Listening | 📋 | Listening history and playlists |
| Plaid | Banking | 📋 | Transactions and investments |
| GitHub | Repository | 📋 | Commits, PRs, and issues |
| Slack | Workspace | 📋 | Messages and mentions |
| Strava | Activities | ✅ | Workouts and performance data |
| Zoom | Meetings | 📋 | Meeting attendance and recordings |

- **iOS Requirements**: Minimum iOS 14.0, requires location/health/microphone permissions
- **Mac Requirements**: Minimum macOS 11.0, requires accessibility and automation permissions

## 🚀 Quick Start

Get Ariata running in under 2 minutes:

```bash
# Clone the repository
git clone https://github.com/ariata-os/ariata
cd ariata

# Copy environment template
cp .env.example .env

# Start all services
docker compose up -d

# Wait for services to initialize (30 seconds)
sleep 30

# Check everything is running
curl http://localhost:3000/api/health

# Open the web interface
open http://localhost:3000
```

That's it! The system will:

- Initialize PostgreSQL with PostGIS and pgvector extensions
- Set up MinIO for object storage
- Start Redis for task queuing
- Launch the SvelteKit web application
- Spin up Celery workers for background processing

### Next Steps

- **Configure data sources**: Visit Settings → Sources in the web UI
- **iOS app**: Build from `apps/ios/` and point to `http://YOUR_IP:3000`
- **Mac agent**: Get token from web UI, run `ariata-mac init TOKEN`
- **Remote access (5G/anywhere)**: See [TAILSCALE_DEPLOY.md](./TAILSCALE_DEPLOY.md)

## 📦 Prerequisites

- Docker & Docker Compose (v2.0+)
- 8GB RAM minimum, 16GB recommended
- 20GB free disk space

## 🔐 Database Access

Ariata provides direct PostgreSQL access for power users. Connect with any SQL client, Jupyter notebooks, or your favorite programming language.

### Managing Database Users

Navigate to `/settings/database` in your Ariata web UI to:

- Create read-only users for safe data analysis
- Create read-write users for custom integrations
- Generate secure connection strings

### Example Queries

```sql
-- Recent heart rate data
SELECT timestamp, heart_rate
FROM stream_ios_healthkit
WHERE heart_rate IS NOT NULL
AND timestamp > NOW() - INTERVAL '24 hours'
ORDER BY timestamp DESC;

-- Location history
SELECT timestamp, longitude as lon, latitude as lat
FROM stream_ios_location
WHERE timestamp::date = CURRENT_DATE
ORDER BY timestamp;

-- Daily step summary
SELECT 
  DATE(timestamp) as day,
  SUM(steps) as total_steps,
  AVG(heart_rate) as avg_heart_rate
FROM stream_ios_healthkit
GROUP BY DATE(timestamp)
ORDER BY day DESC;
```

## 🏗️ Technical Details

### ELT Data Pipeline

Ariata uses an ELT (Extract, Load, Transform) architecture to preserve raw data while enabling flexible analysis:

1. **Extract**: Pull raw data from APIs and devices
2. **Load**: Store in MinIO and PostgreSQL with full fidelity
3. **Transform**: Process and aggregate data for analysis

This approach ensures you never lose data and can reprocess with improved algorithms later.

### Processing Modes

- **Real-time**: Continuous processing for immediate insights
- **Batch**: Nightly consolidation for pattern discovery
- **On-demand**: Query-time transformations for flexibility

### Tech Stack

**Backend**: Python, Celery, FastAPI, PostgreSQL (PostGIS/pgvector), Redis, MinIO

**Frontend**: SvelteKit, TypeScript, TailwindCSS

**Mobile**: Swift/SwiftUI (iOS/macOS)

**ML/AI**: PELT change detection, HDBSCAN clustering, Vector embeddings

## 🔧 Development

### Prerequisites

- Node.js 18+ and pnpm
- Python 3.11+
- Docker & Docker Compose
- Xcode (for iOS/macOS development)

### Commands

```bash
# Web Development
cd apps/web
pnpm install
pnpm dev

# Python Development (with uv)
cd sources
uv sync
uv run python -m base.scheduler.celery_app

# iOS Development
cd apps/ios
open Ariata.xcodeproj

# Mac CLI Development
cd apps/mac
swift build
swift run ariata-mac

# Run tests
make test

# Format code
make format

# Type checking
make typecheck
```

### Environment Variables

Copy `.env.example` to `.env` and configure:

- Database credentials
- MinIO access keys
- OAuth client IDs (for Google/Notion)
- Encryption keys

## 📄 License

Most components are MIT licensed. The ML/AI processing modules use Elastic License v2.

See [LICENSE](LICENSE) file for details.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 💬 Community

- [Discord](https://discord.gg/sSQKzDWqgv) - Join our community
- [GitHub Issues](https://github.com/ariata-os/ariata/issues) - Report bugs or request features
- [Documentation](https://docs.ariata.com) - Coming soon

## 🙏 Acknowledgments

Built with amazing open source projects including PostgreSQL, Redis, MinIO, SvelteKit, and many more.

---

<p align="center">
  <i>Your data. Your insights. Your AI.</i>
</p>
