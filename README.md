# Ariata: An open source ecosystem for owning and exporting your personal data

[![Release](https://img.shields.io/badge/Release-None-red.svg)](https://github.com/ariata-os/ariata/releases)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da?logo=discord&logoColor=white)](https://discord.gg/sSQKzDWqgv)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: ELv2](https://img.shields.io/badge/License-ELv2-orange.svg)](https://www.elastic.co/licensing/elastic-license)

> [!WARNING]
> **Experimental Phase**: Expect rapid iteration and sweeping changes as we refine the core applications and infrastructure.

## What is Ariata

Ariata is your personal AI agent that ingests your digital life—from calendar events and locations to health metrics and screen time—constructing a coherent, queryable timeline. Unlike cloud services that monetize your data, Ariata runs on your infrastructure, ensuring complete privacy and control.

<https://github.com/user-attachments/assets/7d3b0c3b-f20b-4b74-b250-c5ff4c3d8d3d>

Your data is incredibly valuable—companies build trillion-dollar empires on it. Ariata lets you reclaim that value for yourself:

- **Train personal AI on YOUR data**, not theirs
- **Life logging and memory augmentation** for perfect recall
- **Health and productivity optimization** through pattern recognition  
- **Build a queryable life archive** of your entire digital existence
- **Generate insights for self-improvement** from your actual behavior
- **See what data companies collect** and take back control

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

### Legend

- ✅ **Stable**: Fully implemented and tested
- 🚧 **In Progress**: Actively being developed
- 📋 **Planned**: On the roadmap

### Notes

- **Authentication Requirements**: Cloud sources require OAuth2 setup. Device sources require the native app installation.
- **Sync Intervals**: Shown in parentheses for active streams. Pull-based sources check for updates, push-based sources upload batched data.
- **PELT Algorithm**: Change Point Detection using Pruned Exact Linear Time with either L1 (sum of absolute differences) or L2 (sum of squared differences) cost functions.
- **iOS Requirements**: Minimum iOS 14.0, requires location/health/microphone permissions
- **Mac Requirements**: Minimum macOS 11.0, requires accessibility and automation permissions

## 🚀 Quick Start

Get Ariata running in under 2 minutes:

```bash
# Clone the repository
git clone https://github.com/ariata-os/ariata
cd ariata

# Start the entire stack (PostgreSQL, Redis, MinIO, Web App, Workers)
# make dev automatically clones .env.example as .env if none available
make dev

# Open the dashboard
open http://localhost:3000

```

That's it! The system will:

- Initialize PostgreSQL with PostGIS and pgvector extensions
- Set up MinIO for object storage
- Start Redis for task queuing
- Launch the SvelteKit web application
- Spin up Celery workers for background processing

## 🌐 Self-Hosting & Networking

### Recommended: Tailscale Setup (5 Minutes)

Tailscale creates a secure, private network between your devices. Your Ariata instance stays completely private while remaining accessible from all your devices.

#### Quick Start with Tailscale

```bash
# 1. Install Tailscale on your server (where Docker is running)
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up

# 2. Note your Tailscale IP (shown after login, e.g., 100.64.1.5)

# 3. Install Tailscale app on your devices:
# - iOS: App Store → Tailscale
# - macOS: brew install --cask tailscale
# - Windows/Linux: https://tailscale.com/download

# 4. Update your .env file:
PUBLIC_IP=100.64.1.5  # Your Tailscale IP from step 2
FRONTEND_URL=http://100.64.1.5:3000

# 5. Restart Ariata:
make restart

# 6. Access from any device on your Tailscale network:
open http://100.64.1.5:3000
# Or use MagicDNS: http://your-machine.tail-scale.ts.net:3000
```

**Why Tailscale?**

Zero exposed port -- servers aren't on the public internet. E2EE WireGuard protocol. Behind firewalls, NAT, cellular networks. 100 devices, 3 users, perfect for personal use.

### Direct Database Access

Once on Tailscale, you can connect directly to your PostgreSQL database:

```python
# Python
import psycopg2
import pandas as pd

conn = psycopg2.connect(
    "postgresql://ariata_user:ariata_password@100.64.1.5:5432/ariata"
)
df = pd.read_sql("SELECT * FROM stream_ios_healthkit WHERE heart_rate IS NOT NULL", conn)
```

```javascript
// JavaScript/TypeScript
import { Client } from 'pg';

const client = new Client({
  connectionString: 'postgresql://ariata_user:ariata_password@100.64.1.5:5432/ariata'
});
await client.connect();
const result = await client.query('SELECT * FROM stream_google_calendar');
```

See the [Database Access](#database-access) section for managing read-only users and connection strings.

### Alternative Networking Options

**Local Only (Simplest):**

```bash
# No changes needed, access at:
http://localhost:3000
```

**Public VPS (Advanced):**

- Use the included `deploy-ec2-setup.sh` script
- Add HTTPS with Caddy or Traefik
- Consider authentication layer (Authelia)

### How External Services Work

Even with Tailscale, these features work perfectly:

- **OAuth (Google/Notion)**: Handled by `auth.ariata.com` proxy
- **API Syncing**: Outbound connections work normally
- **AI Features**: Can call OpenAI/Anthropic APIs
- **Calendar/Email**: Fetches data via polling

Your instance makes outbound connections but accepts no inbound traffic from the internet.

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

### Available Commands

```bash
make dev              # Start development environment
make stop             # Stop all services
make clean            # Clean up containers and volumes
make logs             # View application logs
make db-studio        # Open Drizzle Studio for database inspection
make test             # Run test suite
make format           # Format code with Biome
make lint             # Lint codebase
```

### Project Structure

```
ariata/
├── apps/                      # User-facing applications
│   ├── web/                   # SvelteKit dashboard
│   ├── ios/                   # Native iOS app
│   ├── mac/                   # Native macOS agent
│   └── google-auth-proxy/     # OAuth proxy for Google services
├── sources/                   # Data pipeline logic
│   ├── base/                  # Shared infrastructure
│   ├── google/                # Google service integrations
│   ├── ios/                   # iOS data sources
│   ├── mac/                   # macOS data sources
│   ├── notion/                # Notion integration
│   └── _registry.yaml         # Master source/stream registry
└── scripts/                   # Utility scripts
```

## 🤝 Contributing

We believe that only an open-source solution to personal data management can truly respect user privacy while covering the long tail of data sources. We welcome contributions in several areas:

### How to Contribute

1. **Code Contributions**: Implement new data sources, improve existing ones, or enhance the core platform
2. **Architecture Reviews**: Share expertise on iOS/Swift, distributed systems, or data processing
3. **Documentation**: Help others understand and use Ariata effectively
4. **Bug Reports**: Find something broken? Let us know!

```bash
# Fork and clone the repository
git clone https://github.com/ariata-os/ariata
cd ariata

# Create a feature branch
git checkout -b feature/your-feature-name

# Make your changes and test
make test

# Submit a pull request
```

## 📄 License

Ariata uses a dual-license model:

- **MIT License**: Core functionality and most components
- **Elastic License 2.0 (ELv2)**: Certain enterprise components

**You can**: Self-host, modify, extend, and use Ariata for personal or commercial purposes.

**You cannot**: Offer Ariata as a hosted service or remove license functionality.

See [LICENSE](./LICENSE) for complete details.

## 📞 Contact & Support

- **Community**: Slack coming soon
- **GitHub Issues**: [Report bugs or request features](https://github.com/ariata-os/ariata/issues)

---

## Axioms

Headless personal data.
The protocol for personal intelligence.
Your data should work for you, not against you.
The search box for your own life.
