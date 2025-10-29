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
	@echo "Development Commands (Native):"
	@echo "  make dev          Start infrastructure (Postgres + MinIO)"
	@echo "                    Then run services natively:"
	@echo "                      Terminal 1: cd core && cargo run -- serve"
	@echo "                      Terminal 2: cd apps/web && npm run dev"
	@echo "  make dev-servers  Auto-run both servers in background"
	@echo "  make stop         Stop all development services"
	@echo "  make logs         View infrastructure logs"
	@echo "  make ps           Show running services"
	@echo ""
	@echo "Database Commands:"
	@echo "  make migrate      Run all migrations (Rust elt + Drizzle app schemas)"
	@echo "  make db-reset     Reset all schemas (WARNING: deletes data)"
	@echo "  make db-status    Check database schemas status"
	@echo ""
	@echo "Production Commands (Full Docker):"
	@echo "  make prod         Start production environment"
	@echo "  make prod-build   Build production images"
	@echo "  make prod-stop    Stop production services"
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
	@echo "Development URLs:"
	@echo "  Web:      http://localhost:$(WEB_DEV_PORT)"
	@echo "  API:      http://localhost:$(API_PORT)"
	@echo "  MinIO:    http://localhost:$(MINIO_CONSOLE_PORT) (minioadmin/minioadmin)"
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

# Start development environment (infrastructure only)
dev: env-check
	@echo "🚀 Starting development environment..."
	@echo ""
	@echo "📦 Starting infrastructure (Postgres + MinIO)..."
	@docker-compose -f docker-compose.dev.yml up -d
	@echo "⏳ Waiting for services..."
	@sleep 8
	@$(MAKE) minio-setup
	@echo ""
	@echo "✅ Infrastructure ready!"
	@echo ""
	@echo "📋 Next steps - Open 2 terminals:"
	@echo ""
	@echo "  Terminal 1 (Rust API):"
	@echo "    cd core && cargo run -- serve"
	@echo ""
	@echo "  Terminal 2 (Web app):"
	@echo "    cd apps/web && npm run dev"
	@echo ""
	@echo "Or run both in background:"
	@echo "    make dev-servers"
	@echo ""
	@echo "Services will be available at:"
	@echo "  Web:      http://localhost:$(WEB_DEV_PORT)"
	@echo "  API:      http://localhost:$(API_PORT)"
	@echo "  MinIO:    http://localhost:$(MINIO_CONSOLE_PORT) (minioadmin/minioadmin)"
	@echo ""

# Run both dev servers in background (parallel make)
dev-servers:
	@echo "🚀 Starting development servers in background..."
	@$(MAKE) -j 2 dev-core dev-web

# Stop development infrastructure and servers
stop:
	@echo "🛑 Stopping development environment..."
	@docker-compose -f docker-compose.dev.yml down
	@pkill -f "cargo run" 2>/dev/null || true
	@pkill -f "vite dev" 2>/dev/null || true
	@echo "✅ Development stopped"

# Restart development
restart: stop dev

# View infrastructure logs
logs:
	@docker-compose -f docker-compose.dev.yml logs -f

# View logs from specific service
logs-postgres:
	@docker-compose -f docker-compose.dev.yml logs -f postgres

logs-minio:
	@docker-compose -f docker-compose.dev.yml logs -f minio

# === HELPER TARGETS (Internal) ===

# Run Rust API natively (blocking)
dev-core:
	@echo "🦀 Starting Rust API server on localhost:8000..."
	@cd core && cargo run -- serve

# Run web app natively (blocking)
dev-web:
	@echo "⚡ Starting SvelteKit dev server on localhost:5173..."
	@cd apps/web && npm run dev

# === MIGRATION COMMANDS ===

# Run all migrations (Rust + Drizzle)
migrate: migrate-rust migrate-drizzle

# Run Rust migrations (ELT database) - works with native dev
migrate-rust:
	@echo "🗄️  Running Rust migrations for ariata_elt..."
	@if docker ps | grep -q ariata-core; then \
		docker-compose exec core ariata migrate; \
	else \
		cd core && cargo run -- migrate; \
	fi
	@echo "✅ Rust migrations complete"

# Run Drizzle migrations (App schema) - native dev
migrate-drizzle:
	@echo "🗄️  Running Drizzle migrations for 'app' schema..."
	@cd apps/web && DATABASE_URL="$(DB_URL)" npx drizzle-kit migrate
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
	@docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt elt.*" 2>/dev/null || \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt elt.*" 2>/dev/null || echo "  ❌ Not accessible"
	@echo ""
	@echo "App Schema (app):"
	@docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt app.*" 2>/dev/null || \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt app.*" 2>/dev/null || echo "  ❌ Not accessible"

# Reset database (WARNING: destructive)
db-reset:
	@echo "⚠️  WARNING: This will delete ALL data in all schemas!"
	@echo "Database: $(DB_NAME) (schemas: elt, app)"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🗑️  Dropping schemas..."; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS elt CASCADE;" 2>/dev/null || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS elt CASCADE;" 2>/dev/null || true; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS app CASCADE;" 2>/dev/null || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS app CASCADE;" 2>/dev/null || true; \
		echo "🆕 Recreating schemas..."; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA elt;" || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA elt;" || exit 1; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA app;" || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "CREATE SCHEMA app;" || exit 1; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA elt TO $(DB_USER);" || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA elt TO $(DB_USER);" || exit 1; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA app TO $(DB_USER);" || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "GRANT ALL ON SCHEMA app TO $(DB_USER);" || exit 1; \
		echo "✅ Schemas recreated"; \
		echo "📝 Running migrations..."; \
		$(MAKE) migrate; \
		echo "✨ Database reset complete!"; \
	else \
		echo "❌ Cancelled"; \
	fi

# === MINIO COMMANDS ===

# Setup MinIO bucket (works with dev or prod)
minio-setup:
	@echo "🪣 Setting up MinIO..."
	@docker-compose -f docker-compose.dev.yml exec minio mc alias set local http://localhost:9000 minioadmin minioadmin 2>/dev/null || \
		docker-compose exec minio mc alias set local http://localhost:9000 minioadmin minioadmin 2>/dev/null || true
	@docker-compose -f docker-compose.dev.yml exec minio mc mb local/ariata-data --ignore-existing 2>/dev/null || \
		docker-compose exec minio mc mb local/ariata-data --ignore-existing 2>/dev/null || true
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
	@echo "Development Infrastructure:"
	@docker-compose -f docker-compose.dev.yml ps 2>/dev/null || echo "  Not running"
	@echo ""
	@echo "Production Services:"
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml ps 2>/dev/null || echo "  Not running"
	@echo ""
	@echo "Native Processes:"
	@pgrep -fl "cargo run" || echo "  No Rust API running"
	@pgrep -fl "vite dev" || echo "  No web dev server running"

# Rebuild production containers (no cache)
rebuild:
	@echo "🔨 Rebuilding production containers..."
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml down
	@docker-compose -f docker-compose.yml -f docker-compose.prod.yml build --no-cache
	@echo "✅ Rebuild complete. Run 'make prod' to start"

# Clean everything (containers, volumes, images)
clean:
	@echo "⚠️  WARNING: This will delete:"
	@echo "  - All containers"
	@echo "  - All volumes (including databases!)"
	@echo "  - All cached images"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker-compose -f docker-compose.dev.yml down -v 2>/dev/null || true; \
		docker-compose -f docker-compose.yml -f docker-compose.prod.yml down -v --rmi all 2>/dev/null || true; \
		pkill -f "cargo run" 2>/dev/null || true; \
		pkill -f "vite dev" 2>/dev/null || true; \
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

# Shell into postgres (works with dev or prod)
shell-postgres:
	@docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) 2>/dev/null || \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME)

# Health check all services
health:
	@echo "🏥 Checking service health..."
	@echo ""
	@echo "PostgreSQL:"
	@docker-compose -f docker-compose.dev.yml exec postgres pg_isready -U $(DB_USER) 2>/dev/null && echo "  ✅ Healthy" || \
		docker-compose exec postgres pg_isready -U $(DB_USER) 2>/dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "MinIO:"
	@curl -sf http://localhost:$(MINIO_PORT)/minio/health/live > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "Rust API:"
	@curl -sf http://localhost:$(API_PORT)/health > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
	@echo ""
	@echo "Web (dev):"
	@curl -sf http://localhost:$(WEB_DEV_PORT) > /dev/null && echo "  ✅ Healthy" || echo "  ❌ Unhealthy"
