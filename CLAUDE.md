# Claude's Readme

Important instruction reminders:

- Do what has been asked; nothing more, nothing less.
- NEVER create files unless they're absolutely necessary for achieving your goal.
- ALWAYS prefer editing an existing file to creating a new one.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Run commands within the Docker container with docker compose exec.

This is a single-user system where the schema follows an ELT pipeline as:

- source_configs: Global catalog of source types (Google, iOS, etc.)
- sources: Active source instances (e.g., "My iPhone", "Work Calendar")
- stream_configs: Global catalog of stream types
- streams: Active stream instances with settings
- stream_*: Time-series data tables (e.g., stream_google_calendar, stream_ios_healthkit)

## DB Credentials

Here are the postgresql credentials for development:

DB_USER=ariata_user
DB_PASSWORD=ariata_password
DB_NAME=ariata
DB_HOST=postgres
DB_PORT=5432

## Monorepo Structure

``` txt
ariata/
├── apps/                      # 📱 User-facing applications and gateways
│   ├── oauth-proxy/           # 🔐 OAuth proxy for Google/Notion services (deployed separately)
│   ├── ios/                   # 🍎 Native iOS application for data collection
│   ├── mac/                   # 🖥️ Native macOS agent application
│   └── web/                   # 🌐 SvelteKit frontend web application
│
├── sources/                   # 📦 Single source of truth for all data pipeline logic
│   ├── base/                  # 🏗️ Shared utilities and infrastructure
│   │   ├── auth/              # 🔐 Authentication handlers (OAuth, device tokens)
│   │   ├── interfaces/        # 📝 Abstract base classes for sources
│   │   ├── processing/        # 🏃‍♀️ Data processing and transformation
│   │   ├── scheduler/         # ⏰ Celery app and background tasks
│   │   ├── storage/           # 💾 MinIO, database, and cache clients
│   │   └── transitions/       # 📈 Change detection algorithms
│   ├── ios/                   # 🍎 iOS data sources
│   │   ├── location/          # 📍 GPS and location tracking
│   │   ├── healthkit/         # 🏃 Health and fitness data
│   │   └── mic/               # 🎤 Audio transcription
│   ├── mac/                   # 🖥️ macOS data sources
│   │   └── apps/              # 💻 Application usage tracking
│   ├── google/                # 🔍 Google service integrations
│   │   └── calendar/          # 📅 Calendar events and sync
│   ├── notion/                # 📝 Notion integration
│   │   └── pages/             # 📄 Page content sync
│   ├── _generated_registry.yaml  # 📚 Auto-generated registry of all sources/streams
│   └── _generated_registry.py    # 🐍 Python version of registry
│
├── scripts/                   # 📜 Utility scripts for development and operations
│   ├── init-db.sql            # 🗄️ Database initialization
│   ├── minio-entrypoint.sh    # 🪣 MinIO container setup
│   ├── deploy-ec2-setup.sh    # ☁️ Production deployment
│   ├── generate_registry.py   # 🔄 Generate source registry from YAML configs
│   └── *.py                   # 🔧 Various utility scripts
│
├── docs/                      # 📚 Project documentation
├── notebooks/                 # 📓 Jupyter notebooks for analysis
├── tests/                     # 🧪 E2E tests, integration tests, and test data
│
├── Makefile                   # 🛠️ Simple aliases for common developer commands
├── biome.json                 # 💅 Linting and formatting configuration
├── docker-compose.yml         # 🚀 Local development stack definition
├── docker-compose.prod.yml    # 🚢 Production stack configuration
└── README.md                  # 📖 Project documentation
```

## Source Architecture Pattern

The Ariata platform follows a two-tier data architecture: **sources → streams**. This configuration-driven approach handles 20+ integrations without code sprawl:

### Benefits

- Adding a new source requires minimal code (just sync logic)
- UI automatically adapts to new sources without frontend changes
- Configuration can be validated and tested separately
- Easy to enable/disable sources without code changes
- Clear separation between business logic and presentation
- YAML format allows inline documentation and better readability
- Centralized infrastructure in `sources/base/` reduces duplication
- Auto-generated models keep database schema in sync

## Technical Opinions

### Backend Stack

- **Python** for all backend services and data processing
- **FastAPI/SvelteKit** for API endpoints (transitioned from separate FastAPI to SvelteKit's native API)
- **Celery + Redis** for task scheduling and background processing
- **MinIO** for object storage (raw data and processed streams)
- **PostgreSQL** for metadata storage (with Drizzle ORM)
- **Docker Compose** for local development environment

### Frontend Stack

- **SvelteKit** for the web application
- **Swift/SwiftUI** for native iOS and macOS applications
- **TypeScript** throughout the web stack

### Data Processing

- **Stream-based architecture** for real-time data processing
- **Stream-based architecture** for real-time data processing
- **Configuration-driven** source and stream definitions
- **Batch processing** with configurable upload intervals

### Development Practices

- **Monorepo structure** with clear separation of concerns
- **Type safety** with TypeScript and Python type hints
- **Automated schema generation** from Drizzle to Python models (run `pnpm db:python` in apps/web/)
- **Comprehensive error handling** and retry mechanisms
