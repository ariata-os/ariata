# Makefile
# Provides simple shortcuts for managing the Ariata project services.

# Default target - show help when just 'make' is run
.DEFAULT_GOAL := help

# === CONFIGURATION ===
# Database Configuration
DB_USER := ariata_user
DB_PASS := ariata_password
DB_NAME := ariata
DB_HOST := localhost
DB_PORT := 5432
DB_URL := postgresql://$(DB_USER):$(DB_PASS)@$(DB_HOST):$(DB_PORT)/$(DB_NAME)
DB_URL_DOCKER := postgresql://$(DB_USER):$(DB_PASS)@postgres:$(DB_PORT)/$(DB_NAME)

# Service Ports (defaults, can be overridden by .env)
WEB_PORT ?= 3000
STUDIO_PORT ?= 4983
MINIO_PORT ?= 9000
MINIO_CONSOLE_PORT ?= 9001
REDIS_PORT ?= 6379
PROCESSING_PORT ?= 8001

# === PHONY TARGETS ===
.PHONY: help dev stop logs studio clean
.PHONY: schema seed reset reset-clean db-reset
.PHONY: schema-generate schema-migrate schema-push
.PHONY: env-check ps rebuild prod
.PHONY: deploy-init deploy-ec2 deploy-update deploy-logs
.PHONY: mac-install mac-run
.PHONY: minio-download minio-download-date minio-process

# === DEVELOPMENT COMMANDS ===

# Check if .env exists, create from example if not
env-check:
	@if [ ! -f .env ]; then \
		echo "📋 Creating .env from .env.example..."; \
		cp .env.example .env; \
		echo "✅ Created .env file"; \
	fi

# Start development environment
dev: env-check
	@echo "🚀 Starting development environment..."
	@docker-compose up --build -d
	@echo "⏳ Waiting for services..."
	@sleep 5
	@echo "🪣 Setting up storage..."
	@docker-compose exec -T minio mc alias set local http://localhost:9000 $${MINIO_ROOT_USER:-minioadmin} $${MINIO_ROOT_PASSWORD:-minioadmin} &>/dev/null 2>&1 || true
	@docker-compose exec -T minio mc mb local/ariata --ignore-existing &>/dev/null 2>&1 || true
	@echo "📝 Setting up database..."
	@docker-compose exec -T -e DATABASE_URL="$(DB_URL_DOCKER)" web npx drizzle-kit push --force &>/dev/null 2>&1 || true
	@echo "🌱 Loading test data..."
	@docker-compose exec -T -e DATABASE_URL="$(DB_URL_DOCKER)" -e MINIO_ENDPOINT="minio:9000" web pnpm db:seed &>/dev/null 2>&1 || true
	@$(MAKE) studio &>/dev/null 2>&1
	@echo ""
	@echo "✅ Ready!"
	@echo ""
	@echo "  Web:     http://localhost:$(WEB_PORT)"
	@echo "  Studio:  http://localhost:$(STUDIO_PORT)"
	@echo "  MinIO:   http://localhost:$(MINIO_CONSOLE_PORT)"
	@echo ""
	@echo "  Run 'make logs' to see logs"
	@echo "  Run 'make stop' to shut down"
	@echo ""

# Stop all services
stop:
	@echo "🛑 Stopping services..."
	@docker-compose down
	@echo "✅ Stopped"

# View logs
logs:
	@docker-compose logs -f

# === MAINTENANCE COMMANDS ===
# Delete everything and start fresh
clean:
	@echo "⚠️  Delete all containers, volumes, and data?"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker-compose down -v; \
		echo "✅ Clean complete. Run 'make dev' to start fresh."; \
	else \
		echo "Cancelled."; \
	fi

# === ADVANCED COMMANDS (not shown in help) ===
# Show service status
ps:
	@docker-compose ps

# Force rebuild containers
rebuild:
	@echo "🔨 Rebuilding containers..."
	@docker-compose down
	@docker-compose build --no-cache
	@docker-compose up -d

# === SCHEMA MANAGEMENT ===
# Complete schema pipeline: YAML → Registry → Drizzle → SQL → Python
schema:
	@echo "🔄 Updating schema..."
	@echo "  📋 Generating registry from YAML configs..."
	@docker-compose exec -T celery-worker uv run python /app/scripts/generate_registry.py || (echo "  ❌ Registry generation failed" && exit 1)
	@echo "  📄 Generating Drizzle TypeScript from YAML..."
	@docker-compose exec -T web npx tsx /app/scripts/generate-drizzle-from-yaml.ts || (echo "  ❌ Drizzle generation failed" && exit 1)
	@echo "  🐍 Generating Pydantic models from YAML..."
	@docker-compose exec -T celery-worker uv run python /app/scripts/generate-pydantic-from-yaml.py || (echo "  ❌ Pydantic generation failed" && exit 1)
	@echo "  🗄️  Generating SQL migrations..."
	@docker-compose exec -T -e DATABASE_URL="$(DB_URL_DOCKER)" web npx drizzle-kit generate || (echo "  ❌ SQL migration generation failed" && exit 1)
	@echo "  ⚡ Pushing schema to database..."
	@docker-compose exec -T -e DATABASE_URL="$(DB_URL_DOCKER)" web npx drizzle-kit push --force >/dev/null 2>&1 || (echo "  ❌ Database push failed" && exit 1)
	@echo "✅ Schema updated!"

# === DATA MANAGEMENT ===
# Seed database with test data
seed:
	@echo "🌱 Loading test data..."
	@echo "  🔄 Running seed script..."
	@docker-compose exec -T -e DATABASE_URL="$(DB_URL_DOCKER)" -e MINIO_ENDPOINT="minio:9000" web npx tsx scripts/seed.ts || (echo "  ❌ Seeding failed" && exit 1)
	@echo "✅ Test data loaded!"

# Reset all data (keeps schema)
reset:
	@echo "🔄 Reset database? (drops and recreates database, reloads schema and test data)"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🗑️  Dropping database..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$(DB_NAME)' AND pid <> pg_backend_pid();" || echo "  ⚠️  No active connections"; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "DROP DATABASE IF EXISTS $(DB_NAME);" || (echo "  ❌ Failed to drop database" && exit 1); \
		echo "  ✅ Database dropped"; \
		echo "🆕 Creating database..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "CREATE DATABASE $(DB_NAME);" || (echo "  ❌ Failed to create database" && exit 1); \
		echo "  ✅ Database created"; \
		echo "🔧 Installing extensions..."; \
		docker-compose exec -T postgres psql -U $(DB_USER) -d $(DB_NAME) < scripts/init-db.sql || (echo "  ❌ Failed to initialize database" && exit 1); \
		echo "  ✅ Extensions installed"; \
		echo "📝 Recreating schema..."; \
		$(MAKE) schema || (echo "  ❌ Schema creation failed" && exit 1); \
		echo "⏳ Waiting for schema to be ready..."; \
		sleep 2; \
		echo "🪣 Clearing MinIO storage..."; \
		docker-compose exec -T minio mc alias set local http://localhost:9000 $${MINIO_ROOT_USER:-minioadmin} $${MINIO_ROOT_PASSWORD:-minioadmin} 2>/dev/null || echo "  ⚠️  MinIO alias already exists"; \
		docker-compose exec -T minio mc rm --recursive --force local/ariata >/dev/null 2>&1 || echo "  ⚠️  No existing MinIO data"; \
		docker-compose exec -T minio mc mb local/ariata --ignore-existing 2>/dev/null || echo "  ⚠️  Bucket already exists"; \
		echo "  ✅ MinIO storage cleared"; \
		echo "🌱 Loading test data..."; \
		$(MAKE) seed || (echo "  ❌ Seeding failed" && exit 1); \
		echo "✨ Reset complete!"; \
	else \
		echo "Cancelled."; \
	fi

# Reset all data WITHOUT test data (completely clean)
reset-clean:
	@echo "🔄 Reset database to empty state? (drops and recreates database, reloads schema, NO test data)"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🗑️  Dropping database..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$(DB_NAME)' AND pid <> pg_backend_pid();" || echo "  ⚠️  No active connections"; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "DROP DATABASE IF EXISTS $(DB_NAME);" || (echo "  ❌ Failed to drop database" && exit 1); \
		echo "  ✅ Database dropped"; \
		echo "🆕 Creating database..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d postgres -c "CREATE DATABASE $(DB_NAME);" || (echo "  ❌ Failed to create database" && exit 1); \
		echo "  ✅ Database created"; \
		echo "🔧 Installing extensions..."; \
		docker-compose exec -T postgres psql -U $(DB_USER) -d $(DB_NAME) < scripts/init-db.sql || (echo "  ❌ Failed to initialize database" && exit 1); \
		echo "  ✅ Extensions installed"; \
		echo "📝 Recreating schema..."; \
		$(MAKE) schema || (echo "  ❌ Schema creation failed" && exit 1); \
		echo "🪣 Clearing MinIO storage..."; \
		docker-compose exec -T minio mc alias set local http://localhost:9000 $${MINIO_ROOT_USER:-minioadmin} $${MINIO_ROOT_PASSWORD:-minioadmin} 2>/dev/null || echo "  ⚠️  MinIO alias already exists"; \
		docker-compose exec -T minio mc rm --recursive --force local/ariata >/dev/null 2>&1 || echo "  ⚠️  No existing MinIO data"; \
		docker-compose exec -T minio mc mb local/ariata --ignore-existing 2>/dev/null || echo "  ⚠️  Bucket already exists"; \
		echo "  ✅ MinIO storage cleared"; \
		echo "✨ Reset complete! Database and storage are now empty."; \
	else \
		echo "Cancelled."; \
	fi

# === DEVELOPMENT UI ===
# Open Drizzle Studio database UI
studio:
	@docker-compose exec -d -e DATABASE_URL="$(DB_URL_DOCKER)" web pnpm drizzle-kit studio --host 0.0.0.0 --port $(STUDIO_PORT) &>/dev/null 2>&1 || true
	@echo "🎨 Drizzle Studio: http://localhost:$(STUDIO_PORT)"

# === ADVANCED SCHEMA COMMANDS (not shown in help) ===
# Individual steps of the schema pipeline
schema-generate:
	@echo "📋 Generating registry from YAML configs..."
	@python3 scripts/generate_registry.py
	@echo "📄 Generating Drizzle TypeScript from YAML..."
	@npx tsx scripts/generate-drizzle-from-yaml.ts
	@echo "🐍 Generating Pydantic models from YAML..."
	@python3 scripts/generate_pydantic_from_yaml.py

schema-migrate:
	@echo "🗄️  Generating SQL migrations from Drizzle..."
	@cd apps/web && pnpm drizzle-kit generate

schema-push:
	@echo "⚡ Pushing schema to database..."
	@docker-compose exec -e DATABASE_URL="$(DB_URL_DOCKER)" web npx drizzle-kit push

# Validate YAML configurations
validate:
	@echo "🔍 Validating YAML configurations..."
	@python3 scripts/validate_yaml_configs.py

# === DEPLOYMENT COMMANDS ===

# Start production environment (no build)
prod: env-check
	@echo "🚀 Starting Ariata in production mode..."
	@if [ -z "$${PUBLIC_IP}" ]; then \
		export PUBLIC_IP=$$(curl -s ifconfig.me 2>/dev/null || echo "localhost"); \
	fi; \
	PUBLIC_IP=$$PUBLIC_IP docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
	@echo "✅ Production environment started!"
	@echo "   Access at: http://$${PUBLIC_IP:-localhost}:$(WEB_PORT)"

# Package for deployment
deploy-init:
	@echo "📦 Creating deployment package..."
	@tar -czf deploy.tar.gz \
		docker-compose.yml \
		docker-compose.prod.yml \
		.env.example \
		Makefile \
		services/ \
		sources/ \
		apps/ \
		assets/ \
		scripts/ \
		--exclude='**/__pycache__' \
		--exclude='**/node_modules' \
		--exclude='**/.DS_Store' \
		--exclude='**/.git'
	@echo "✅ Created deploy.tar.gz"

# Deploy to EC2
deploy-ec2:
	@read -p "EC2 Host (user@ip): " host; \
	echo "📤 Uploading to $$host..."; \
	scp deploy.tar.gz $$host:~/ && \
	echo "🚀 Deploying on remote host..."; \
	ssh $$host 'tar -xzf deploy.tar.gz && make env-setup && make prod'

# Update deployment (git pull + restart)
deploy-update:
	@read -p "EC2 Host (user@ip): " host; \
	ssh $$host "cd ariata && git pull && make prod"

# View remote logs
deploy-logs:
	@read -p "EC2 Host (user@ip): " host; \
	ssh $$host "cd ariata && docker-compose logs -f"

# === MINIO COMMANDS ===

# Download today's streams from MinIO
minio-download:
	@echo "📥 Downloading today's stream data from MinIO..."
	@docker-compose exec celery-worker uv run python /app/scripts/download-minio-streams.py
	@echo "✅ Stream data saved to assets/test-data/captured-streams/"

# Download streams for a specific date
minio-download-date:
	@read -p "Date (YYYY-MM-DD): " date; \
	echo "📥 Downloading streams for $$date..."; \
	docker-compose exec celery-worker uv run python /app/scripts/download-minio-streams.py $$date

# Process captured streams to filter by date
minio-process:
	@echo "🔄 Processing captured stream data..."
	@docker-compose exec celery-worker uv run python /app/scripts/filter-stream-data.py
	@echo "✅ Processed data saved to assets/test-data/captured-streams/processed/"



# === MAC CLI COMMANDS ===

# Build Mac CLI for development
mac-build:
	@echo "🔨 Building Mac CLI..."
	@cd apps/mac && swift build

# Build Mac CLI release (universal binary)
mac-release:
	@echo "📦 Building Mac CLI release..."
	@cd apps/mac && ./Scripts/build-release.sh

# Install Mac CLI locally
mac-install:
	@echo "📦 Installing Mac CLI to /usr/local/bin..."
	@cd apps/mac && swift build -c release
	@sudo cp apps/mac/.build/release/ariata-mac /usr/local/bin/
	@echo "✅ Installed. Run 'ariata-mac --help' to get started"

# Create GitHub release (requires gh CLI)
mac-publish:
	@echo "🚀 Creating GitHub release..."
	@cd apps/mac && \
	VERSION=$$(grep 'static let current' Sources/Version.swift | cut -d'"' -f2) && \
	echo "Creating release for version $$VERSION..." && \
	git tag -a "mac-v$$VERSION" -m "Mac CLI version $$VERSION" && \
	git push origin "mac-v$$VERSION" && \
	echo "✅ Tagged and pushed. GitHub Actions will build and release."

# Test Mac CLI
mac-test:
	@echo "🧪 Testing Mac CLI..."
	@cd apps/mac && swift test

# Run Mac CLI
mac-run:
	@echo "🖥️  Running Mac CLI..."
	@cd apps/mac && swift run ariata-mac

# Build and install Mac CLI locally for testing
mac-local:
	@echo "🛠️  Building and installing Mac CLI locally..."
	@echo "Step 1: Stopping any existing ariata-mac processes..."
	@pkill -f "ariata-mac" 2>/dev/null || true
	@if launchctl list | grep -q "com.ariata.mac" 2>/dev/null; then \
		launchctl unload ~/Library/LaunchAgents/com.ariata.mac.plist 2>/dev/null || true; \
	fi
	@echo "Step 2: Building release version..."
	@cd apps/mac && swift build -c release
	@echo "Step 3: Running installer with local build..."
	@cd apps/mac && ./Scripts/installer.sh --local
	@echo "✅ Local installation complete!"

# === HELP ===

help:
	@echo ""
	@echo "  🚀 Ariata - All Commands"
	@echo "  ────────────────────────────────"
	@echo ""
	@echo "  DEVELOPMENT"
	@echo "    make dev              Start development environment"
	@echo "    make stop             Stop all services"
	@echo "    make logs             View logs"
	@echo "    make studio           Open Drizzle Studio UI"
	@echo "    make ps               Show service status"
	@echo "    make rebuild          Force rebuild containers"
	@echo ""
	@echo "  SCHEMA MANAGEMENT"
	@echo "    make schema           Full pipeline: YAML→Drizzle→SQL→Python"
	@echo "    make schema-generate  Generate Drizzle schemas from YAML"
	@echo "    make schema-migrate   Generate SQL migrations"
	@echo "    make schema-push      Push schema to database"
	@echo ""
	@echo "  DATA MANAGEMENT"
	@echo "    make seed             Load test data"
	@echo "    make reset            Clear all data & reseed with test data"
	@echo "    make reset-clean      Clear all data (no test data)"
	@echo "    make clean            Delete everything (nuclear)"
	@echo ""
	@echo "  DEPLOYMENT"
	@echo "    make prod             Start production mode"
	@echo "    make deploy-init      Create deployment package"
	@echo "    make deploy-ec2       Deploy to EC2"
	@echo "    make deploy-update    Update deployment"
	@echo "    make deploy-logs      View remote logs"
	@echo ""
	@echo "  DATA EXPORT"
	@echo "    make minio-download   Download today's streams"
	@echo "    make minio-download-date  Download specific date"
	@echo "    make minio-process    Process captured streams"
	@echo ""
	@echo "  MAC CLI"
	@echo "    make mac-build        Build Mac CLI for development"
	@echo "    make mac-release      Build universal binary release"
	@echo "    make mac-install      Install to /usr/local/bin"
	@echo "    make mac-local        Build and test with local installer"
	@echo "    make mac-publish      Create GitHub release (requires gh CLI)"
	@echo "    make mac-test         Run tests"
	@echo "    make mac-run          Run Mac CLI"
	@echo ""
