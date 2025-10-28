# Makefile
# Provides simple shortcuts for managing the Ariata project services.

# Default target - show help when just 'make' is run
.DEFAULT_GOAL := help

# === CONFIGURATION ===
# Database Configuration (Single database with schemas)
DB_USER := postgres
DB_PASS := postgres
DB_NAME := ariata
DB_HOST := localhost
DB_PORT := 5432
DB_URL := postgresql://$(DB_USER):$(DB_PASS)@$(DB_HOST):$(DB_PORT)/$(DB_NAME)
DB_URL_DOCKER := postgresql://$(DB_USER):$(DB_PASS)@postgres:$(DB_PORT)/$(DB_NAME)

# Service Ports
WEB_DEV_PORT := 5173
WEB_PROD_PORT := 3000
API_PORT := 8000
MINIO_PORT := 9000
MINIO_CONSOLE_PORT := 9001
STUDIO_PORT := 4983

# === PHONY TARGETS ===
.PHONY: help dev dev-watch stop restart logs clean ps rebuild
.PHONY: migrate migrate-rust migrate-drizzle
.PHONY: db-reset db-status
.PHONY: prod prod-build prod-restart
.PHONY: env-check minio-setup
.PHONY: test test-rust test-web
.PHONY: mac-build mac-install mac-run

# === HELP ===
help:
	@echo ""
	@echo "🚀 Ariata - Personal Data ELT Platform"
	@echo ""
	@echo "Development Commands:"
	@echo "  make dev          Start all services (background)"
	@echo "  make dev-watch    Start all services and follow logs"
	@echo "  make stop         Stop all services"
	@echo "  make restart      Restart all services"
	@echo "  make logs         View logs from all services"
	@echo "  make ps           Show running services"
	@echo ""
	@echo "Database Commands:"
	@echo "  make migrate      Run all migrations (Rust elt + Drizzle app schemas)"
	@echo "  make db-reset     Reset all schemas (WARNING: deletes data)"
	@echo "  make db-status    Check database schemas status"
	@echo ""
	@echo "Production Commands:"
	@echo "  make prod         Start production environment"
	@echo "  make prod-build   Build production images"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test         Run all tests"
	@echo "  make test-rust    Run Rust tests"
	@echo "  make test-web     Run web tests"
	@echo ""
	@echo "Maintenance Commands:"
	@echo "  make clean        Remove all containers and volumes"
	@echo "  make rebuild      Rebuild all containers"
	@echo ""
	@echo "Services:"
	@echo "  - Web (dev):      http://localhost:$(WEB_DEV_PORT)"
	@echo "  - Rust API:       http://localhost:$(API_PORT)"
	@echo "  - MinIO Console:  http://localhost:$(MINIO_CONSOLE_PORT)"
	@echo ""

# === DEVELOPMENT COMMANDS ===

# Check if .env exists, create from example if not
env-check:
	@if [ ! -f .env ]; then \
		echo "📋 Creating .env from .env.example..."; \
		cp .env.example .env; \
		echo "✅ Created .env file"; \
		echo "⚠️  Please update .env with your actual credentials"; \
	fi

# Start development environment
dev: env-check
	@echo "🚀 Starting development environment..."
	@docker-compose up --build -d
	@echo "⏳ Waiting for services to be ready..."
	@sleep 10
	@$(MAKE) minio-setup
	@$(MAKE) migrate
	@echo ""
	@echo "✅ Development environment ready!"
	@echo ""
	@echo "  Web (dev):    http://localhost:$(WEB_DEV_PORT)"
	@echo "  Rust API:     http://localhost:$(API_PORT)"
	@echo "  MinIO:        http://localhost:$(MINIO_CONSOLE_PORT) (minioadmin/minioadmin)"
	@echo ""
	@echo "  Run 'make logs' to view logs"
	@echo "  Run 'make stop' to shut down"
	@echo ""

# Start development environment and follow logs
dev-watch: dev
	@echo "📺 Following logs (Ctrl+C to exit, services keep running)..."
	@echo ""
	@docker-compose logs -f

# Stop all services
stop:
	@echo "🛑 Stopping all services..."
	@docker-compose down
	@echo "✅ All services stopped"

# Restart all services
restart: stop dev

# View logs from all services
logs:
	@docker-compose logs -f

# View logs from specific service
logs-core:
	@docker-compose logs -f core

logs-web:
	@docker-compose logs -f web

logs-postgres:
	@docker-compose logs -f postgres

logs-minio:
	@docker-compose logs -f minio

# === MIGRATION COMMANDS ===

# Run all migrations (Rust + Drizzle)
migrate: migrate-rust migrate-drizzle

# Run Rust migrations (ELT database)
migrate-rust:
	@echo "🗄️  Running Rust migrations for ariata_elt..."
	@docker-compose exec core ariata migrate || \
		(echo "⚠️  Core service not running. Starting it first..." && \
		 docker-compose up -d core && sleep 5 && \
		 docker-compose exec core ariata migrate)
	@echo "✅ Rust migrations complete"

# Run Drizzle migrations (App schema)
migrate-drizzle:
	@echo "🗄️  Running Drizzle migrations for 'app' schema..."
	@cd apps/web && DATABASE_URL="$(DB_URL)" npx drizzle-kit migrate || \
		(echo "⚠️  Running migrations in Docker..." && \
		 docker-compose exec -e DATABASE_URL="$(DB_URL_DOCKER)" web npx drizzle-kit migrate)
	@echo "✅ Drizzle migrations complete"

# Generate new Drizzle migration
migrate-drizzle-generate:
	@echo "📝 Generating Drizzle migration..."
	@cd apps/web && DATABASE_URL="$(DB_URL)" npx drizzle-kit generate
	@echo "✅ Migration generated in apps/web/drizzle/"

# Push schema directly (no migration files)
migrate-drizzle-push:
	@echo "⚡ Pushing Drizzle schema to database..."
	@cd apps/web && DATABASE_URL="$(DB_URL)" npx drizzle-kit push
	@echo "✅ Schema pushed"

# === DATABASE COMMANDS ===

# Check database status
db-status:
	@echo "📊 Database Status (ariata):"
	@echo ""
	@echo "ELT Schema (elt):"
	@docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt elt.*" 2>/dev/null || echo "  ❌ Not accessible"
	@echo ""
	@echo "App Schema (app):"
	@docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt app.*" 2>/dev/null || echo "  ❌ Not accessible"

# Reset database (WARNING: destructive)
db-reset:
	@echo "⚠️  WARNING: This will delete ALL data in all schemas!"
	@echo "Database: $(DB_NAME) (schemas: elt, app)"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🗑️  Dropping schemas..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS elt CASCADE;" 2>/dev/null || true; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS app CASCADE;" 2>/dev/null || true; \
		echo "🆕 Recreating schemas..."; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA elt;" || exit 1; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA app;" || exit 1; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA elt TO $(DB_USER);" || exit 1; \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA app TO $(DB_USER);" || exit 1; \
		echo "✅ Schemas recreated"; \
		echo "📝 Running migrations..."; \
		$(MAKE) migrate; \
		echo "✨ Database reset complete!"; \
	else \
		echo "❌ Cancelled"; \
	fi

# === MINIO COMMANDS ===

# Setup MinIO bucket
minio-setup:
	@echo "🪣 Setting up MinIO..."
	@docker-compose exec minio mc alias set local http://localhost:9000 minioadmin minioadmin 2>/dev/null || true
	@docker-compose exec minio mc mb local/ariata-data --ignore-existing 2>/dev/null || true
	@echo "✅ MinIO bucket ready"

# === PRODUCTION COMMANDS ===

# Build production images
prod-build:
	@echo "🔨 Building production images..."
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml build
	@echo "✅ Production images built"

# Start production environment
prod: env-check prod-build
	@echo "🚀 Starting production environment..."
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
	@echo "⏳ Waiting for services..."
	@sleep 15
	@$(MAKE) minio-setup
	@echo ""
	@echo "✅ Production environment ready!"
	@echo ""
	@echo "  Web (prod):   http://localhost:$(WEB_PROD_PORT)"
	@echo "  Rust API:     http://localhost:$(API_PORT)"
	@echo ""

# Restart production services
prod-restart:
	@echo "🔄 Restarting production services..."
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml down
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
	@echo "✅ Production services restarted"

# Stop production services
prod-stop:
	@echo "🛑 Stopping production services..."
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml down
	@echo "✅ Production services stopped"

# === TESTING COMMANDS ===

# Run all tests
test: test-rust test-web

# Run Rust tests
test-rust:
	@echo "🧪 Running Rust tests..."
	@docker-compose exec core cargo test
	@echo "✅ Rust tests complete"

# Run web tests
test-web:
	@echo "🧪 Running web tests..."
	@docker-compose exec web pnpm test
	@echo "✅ Web tests complete"

# === MAINTENANCE COMMANDS ===

# Show service status
ps:
	@docker-compose ps

# Rebuild all containers (no cache)
rebuild:
	@echo "🔨 Rebuilding all containers..."
	@docker-compose down
	@docker-compose build --no-cache
	@echo "✅ Rebuild complete. Run 'make dev' to start"

# Clean everything (containers, volumes, images)
clean:
	@echo "⚠️  WARNING: This will delete:"
	@echo "  - All containers"
	@echo "  - All volumes (including databases!)"
	@echo "  - All cached images"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker-compose down -v --rmi all; \
		echo "✅ Cleaned. Run 'make dev' to start fresh."; \
	else \
		echo "❌ Cancelled"; \
	fi

# === MAC CLI COMMANDS ===

# Build Mac CLI for development
mac-build:
	@echo "🔨 Building Mac CLI..."
	@cd apps/mac && swift build
	@echo "✅ Mac CLI built"

# Build Mac CLI release (universal binary)
mac-release:
	@echo "📦 Building Mac CLI release..."
	@cd apps/mac && ./Scripts/build-release.sh
	@echo "✅ Release build complete"

# Install Mac CLI locally
mac-install:
	@echo "📦 Installing Mac CLI to /usr/local/bin..."
	@cd apps/mac && swift build -c release
	@sudo cp apps/mac/.build/release/ariata-mac /usr/local/bin/
	@echo "✅ Installed. Run 'ariata-mac --help'"

# Test Mac CLI
mac-test:
	@echo "🧪 Testing Mac CLI..."
	@cd apps/mac && swift test
	@echo "✅ Mac CLI tests complete"

# Run Mac CLI
mac-run:
	@echo "🖥️  Running Mac CLI..."
	@cd apps/mac && swift run ariata-mac

# Build and install Mac CLI locally for testing
mac-local:
	@echo "🛠️  Building and installing Mac CLI locally..."
	@pkill -f "ariata-mac" 2>/dev/null || true
	@if launchctl list | grep -q "com.ariata.mac" 2>/dev/null; then \
		launchctl unload ~/Library/LaunchAgents/com.ariata.mac.plist 2>/dev/null || true; \
	fi
	@cd apps/mac && swift build -c release
	@cd apps/mac && ./Scripts/installer.sh --local
	@echo "✅ Local installation complete"

# === DRIZZLE STUDIO ===

# Open Drizzle Studio for app schema
studio:
	@echo "🎨 Starting Drizzle Studio..."
	@cd apps/web && DATABASE_URL="$(DB_URL)" npx drizzle-kit studio --host 0.0.0.0 --port $(STUDIO_PORT) &
	@echo "✅ Drizzle Studio: http://localhost:$(STUDIO_PORT)"

# === UTILITY COMMANDS ===

# Shell into Rust container
shell-core:
	@docker-compose exec core /bin/sh

# Shell into web container
shell-web:
	@docker-compose exec web /bin/sh

# Shell into postgres (elt schema)
shell-postgres:
	@docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME)

# View Rust logs
api-logs:
	@docker-compose logs -f core

# Health check all services
health:
	@echo "🏥 Checking service health..."
	@echo ""
	@echo "PostgreSQL:"
	@docker-compose exec postgres pg_isready -U $(DB_USER) && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "MinIO:"
	@curl -sf http://localhost:$(MINIO_PORT)/minio/health/live > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "Rust API:"
	@curl -sf http://localhost:$(API_PORT)/health > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "Web (dev):"
	@curl -sf http://localhost:$(WEB_DEV_PORT) > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
