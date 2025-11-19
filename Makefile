# Makefile
# Provides simple shortcuts for managing the Ariata project services.

# Default target - show help when just 'make' is run
.DEFAULT_GOAL := help

# Load environment variables from .env file
-include .env
export

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
.PHONY: help dev dev-watch stop restart logs clean clean-all ps rebuild
.PHONY: migrate migrate-rust prepare generate-seeds seed prod-seed seed-rome
.PHONY: db-reset db-status
.PHONY: prod prod-build prod-restart
.PHONY: env-check minio-setup
.PHONY: test test-rust test-web
.PHONY: mac mac-debug mac-status mac-clean
.PHONY: core-ngrok

# === HELP ===
help:
	@echo ""
	@echo "🚀 Ariata - Personal Data ELT Platform"
	@echo ""
	@echo "Development Commands (Native):"
	@echo "  make dev          Start infrastructure (Postgres + MinIO)"
	@echo "  make dev SEED=true  Start infrastructure with test data seeding"
	@echo "                    Then run services natively:"
	@echo "                      Terminal 1: cd core && cargo run -- server"
	@echo "                      Terminal 2: cd apps/web && npm run dev"
	@echo "  make dev-servers  Auto-run both servers in background"
	@echo "  make core-ngrok   Start Rust API with ngrok tunnel (for iOS dev)"
	@echo "  make stop         Stop all development services"
	@echo "  make logs         View infrastructure logs"
	@echo "  make ps           Show running services"
	@echo ""
	@echo "Database Commands:"
	@echo "  make migrate         Run database migrations (SQLx - manages both data and app schemas)"
	@echo "  make prepare         Regenerate SQLx .sqlx/ metadata (after schema changes)"
	@echo "  make generate-seeds  Generate config/seeds/*.json from Rust registry (sources/streams)"
	@echo "  make seed            Seed database with Monday in Rome reference dataset (real-world data)"
	@echo "  make prod-seed       Seed production defaults (models, agents, tools, sample tags)"
	@echo "  make db-reset        Reset all schemas (WARNING: deletes data)"
	@echo "  make db-status       Check database schemas status"
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
	@echo "Mac App Commands:"
	@echo "  make mac          Build and install Mac.app (Release) to /Applications"
	@echo "  make mac-debug    Build and install Mac.app (Debug - may have issues)"
	@echo "  make mac-status   Show background service, pairing, and queue status"
	@echo "  make mac-clean    Remove config/queue for fresh testing"
	@echo ""
	@echo "Maintenance Commands:"
	@echo "  make clean        Remove all containers and volumes"
	@echo "  make clean-all    Clean backend + Mac app (complete dev reset)"
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

# Start development environment (infrastructure + migrations + SQLx cache)
dev: env-check
	@echo "🚀 Starting development environment..."
	@echo ""
	@echo "📦 Starting infrastructure (Postgres + MinIO)..."
	@docker-compose -f docker-compose.dev.yml up -d
	@echo "⏳ Waiting for services..."
	@sleep 8
	@echo ""
	@echo "🗄️  Running database migrations..."
	@$(MAKE) migrate
	@echo ""
	@echo "🔄 Checking SQLx query cache..."
	@if [ ! -f core/.sqlx/query-*.json ]; then \
		echo "   Setting up database for SQLx..."; \
		docker-compose -f docker-compose.dev.yml exec -T postgres psql -U $(DB_USER) -d $(DB_NAME) -c "ALTER ROLE $(DB_USER) IN DATABASE $(DB_NAME) SET search_path TO data, public;" > /dev/null 2>&1; \
		echo "   Generating SQLx cache (first time setup)..."; \
		$(MAKE) prepare; \
	else \
		echo "   ✅ SQLx cache exists"; \
	fi
	@echo ""
	@$(MAKE) minio-setup
	@echo ""
	@echo "🌱 Seeding system defaults (models, agents, tools)..."
	@$(MAKE) prod-seed
	@echo ""
	@if [ "$$SEED" = "true" ]; then \
		echo "🌱 Seeding reference dataset (Monday in Rome)..."; \
		$(MAKE) seed; \
		echo ""; \
	fi
	@echo "✅ Development environment ready!"
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

# Run both dev servers in background (parallel make)
dev-servers:
	@echo "🚀 Starting development servers in background..."
	@$(MAKE) -j 2 dev-core dev-web

# Stop development infrastructure and servers
stop:
	@echo "🛑 Stopping development environment..."
	@echo "   Stopping Docker containers..."
	@docker-compose -f docker-compose.dev.yml down
	@echo "   Killing backend processes..."
	@pkill -f "cargo run" 2>/dev/null || true
	@lsof -ti:$(API_PORT) | xargs kill -9 2>/dev/null || true
	@echo "   Killing frontend processes..."
	@pkill -f "vite dev" 2>/dev/null || true
	@lsof -ti:$(WEB_PORT) | xargs kill -9 2>/dev/null || true
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

# Run Rust API with ngrok tunnel (for iOS development)
core-ngrok:
	@echo "🦀 Starting Rust API server with ngrok tunnel..."
	@echo ""
	@if ! command -v ngrok > /dev/null 2>&1; then \
		echo "❌ ngrok not found!"; \
		echo ""; \
		echo "Install with: brew install ngrok"; \
		echo "Or download from: https://ngrok.com/download"; \
		exit 1; \
	fi
	@echo "🧹 Cleaning up port $(API_PORT)..."
	@lsof -ti:$(API_PORT) | xargs kill -9 2>/dev/null || true
	@echo "🚀 Starting Rust server on localhost:$(API_PORT)..."
	@cd core && RUST_LOG=$(RUST_LOG) cargo run -- server & \
	SERVER_PID=$$!; \
	echo "⏳ Waiting for server to start..."; \
	sleep 3; \
	echo "🌐 Starting ngrok tunnel..."; \
	if [ -n "$$NGROK_DOMAIN" ]; then \
		echo "   Using static domain: $$NGROK_DOMAIN"; \
		ngrok http $(API_PORT) --domain=$$NGROK_DOMAIN --log=false > /dev/null 2>&1 & \
	else \
		echo "   Using random URL (set NGROK_DOMAIN in .env for static domain)"; \
		ngrok http $(API_PORT) --log=false > /dev/null 2>&1 & \
	fi; \
	NGROK_PID=$$!; \
	sleep 2; \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo "✅ ngrok tunnel established!"; \
	echo ""; \
	if [ -n "$$NGROK_DOMAIN" ]; then \
		echo "📱 Your HTTPS endpoint:"; \
		echo "   https://$$NGROK_DOMAIN"; \
	else \
		echo "📱 Get your HTTPS URL from:"; \
		echo "   http://localhost:4040"; \
		echo ""; \
		echo "🔍 Or run this command:"; \
		echo "   curl -s http://localhost:4040/api/tunnels | jq -r '.tunnels[0].public_url'"; \
	fi; \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo ""; \
	echo "📋 Press Ctrl+C to stop server and tunnel"; \
	trap "echo ''; echo '🛑 Stopping...'; kill $$SERVER_PID $$NGROK_PID 2>/dev/null; exit 0" INT TERM; \
	wait

# === MIGRATION COMMANDS ===

# Run all migrations (SQLx manages both data and app schemas)
migrate: migrate-rust

# Run SQLx migrations (data + app schemas) - works with native dev
migrate-rust:
	@echo "🗄️  Running SQLx migrations (data + app schemas)..."
	@if docker ps | grep -q ariata-core; then \
		docker-compose exec core ariata migrate; \
	else \
		cd core && sqlx migrate run --database-url $(DB_URL); \
	fi
	@echo "✅ Migrations complete"

# Regenerate SQLx offline query metadata
# Run this after:
#  - Creating new queries with sqlx::query!()
#  - Modifying existing queries
#  - Running migrations that change table schemas
prepare:
	@echo "🔄 Regenerating SQLx query metadata..."
	@cd core && SQLX_OFFLINE=false cargo sqlx prepare --database-url "$(DB_URL)?options=-csearch_path%3Ddata%2Cpublic"
	@echo "✅ SQLx metadata updated in core/.sqlx/"
	@echo "💡 Remember to commit the updated .sqlx/ files"

# Generate seed configuration files from registry
generate-seeds:
	@echo "🔧 Generating config/seeds/*.json from Rust registry..."
	@echo "   Generates _generated_source_connections.json and _generated_stream_connections.json"
	@cd core && cargo run --bin generate-seeds
	@echo "✅ Seed files generated"
	@echo "💡 Review the generated files in config/seeds/"

# === DATABASE COMMANDS ===

# Seed database with Monday in Rome reference dataset
seed:
	@echo "🇮🇹 Seeding database with Monday in Rome reference dataset..."
	@echo "   This tests the full pipeline: CSV → Archive → Transform → Ontology tables"
	@cd core && SQLX_OFFLINE=false cargo run --bin ariata-seed
	@echo "✅ Database seeding complete"

# Seed production database with defaults (models, agents, tools, sample tags)
prod-seed: generate-seeds
	@echo "🌱 Seeding production database with defaults..."
	@echo "   This seeds: LLM models, agents, tools, and sample axiology tags"
	@cd core && SQLX_OFFLINE=false cargo run --bin ariata-prod-seed
	@echo "✅ Production seeding complete"

# Check database status
db-status:
	@echo "📊 Database Status (ariata):"
	@echo ""
	@echo "Data Schema (data):"
	@docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt data.*" 2>/dev/null || \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt data.*" 2>/dev/null || echo "  ❌ Not accessible"
	@echo ""
	@echo "App Schema (app):"
	@docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt app.*" 2>/dev/null || \
		docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "\\dt app.*" 2>/dev/null || echo "  ❌ Not accessible"

# Reset database (WARNING: destructive)
db-reset:
	@echo "⚠️  WARNING: This will delete ALL data in all schemas!"
	@echo "Database: $(DB_NAME) (schemas: data, app)"
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🗑️  Dropping schemas..."; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS data CASCADE;" 2>/dev/null || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS data CASCADE;" 2>/dev/null || true; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS app CASCADE;" 2>/dev/null || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS app CASCADE;" 2>/dev/null || true; \
		echo "🗑️  Dropping old elt schema (deprecated)..."; \
		docker-compose -f docker-compose.dev.yml exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS elt CASCADE;" 2>/dev/null || \
			docker-compose exec postgres psql -U $(DB_USER) -d $(DB_NAME) -c "DROP SCHEMA IF EXISTS elt CASCADE;" 2>/dev/null || true; \
		echo "📝 Running migrations (will create schemas)..."; \
		$(MAKE) migrate; \
		echo "✨ Database reset complete!"; \
	else \
		echo "❌ Cancelled"; \
	fi

# === MINIO COMMANDS ===

# Setup MinIO bucket (works with dev or prod)
minio-setup:
	@echo "🪣 Setting up MinIO..."
	@docker-compose -f docker-compose.dev.yml exec minio mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null 2>&1 || \
		docker-compose exec minio mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null 2>&1 || true
	@docker-compose -f docker-compose.dev.yml exec minio mc mb local/ariata-data --ignore-existing > /dev/null 2>&1 || \
		docker-compose exec minio mc mb local/ariata-data --ignore-existing > /dev/null 2>&1 || true
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

# Clean everything - backend AND Mac app (for complete dev reset)
clean-all:
	@echo "🧹 Cleaning EVERYTHING (backend + Mac app)..."
	@echo ""
	@echo "This will:"
	@echo "  - Stop and remove all Docker containers + volumes"
	@echo "  - Delete Mac app config, credentials, and queue"
	@echo ""
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "🧹 Cleaning backend..."; \
		$(MAKE) clean || true; \
		echo ""; \
		echo "🧹 Cleaning Mac app..."; \
		$(MAKE) mac-clean || true; \
		echo ""; \
		echo "✅ Full cleanup complete!"; \
		echo ""; \
		echo "📋 Next steps:"; \
		echo "  1. make dev           (start fresh backend)"; \
		echo "  2. open Mac app       (re-pair device)"; \
	else \
		echo "❌ Cancelled"; \
	fi

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

# === MAC APP COMMANDS ===

# Build Mac.app and install to /Applications (stable path for permissions)
mac:
	@echo "🔨 Building Mac.app (Release)..."
	@cd apps/mac && xcodebuild \
		-project mac.xcodeproj \
		-scheme mac \
		-configuration Release \
		-derivedDataPath ./build \
		clean build
	@echo "📦 Installing to /Applications..."
	@sudo rm -rf /Applications/ariata-mac.app
	@sudo cp -R apps/mac/build/Build/Products/Release/mac.app /Applications/ariata-mac.app
	@echo "✅ Installed to /Applications/ariata-mac.app"
	@echo "💡 Launch with: open /Applications/ariata-mac.app"

# Build Mac.app in Debug mode (may have code signing issues with debug dylib)
mac-debug:
	@echo "🔨 Building Mac.app (Debug)..."
	@cd apps/mac && xcodebuild \
		-project mac.xcodeproj \
		-scheme mac \
		-configuration Debug \
		-derivedDataPath ./build \
		clean build
	@echo "📦 Installing to /Applications..."
	@sudo rm -rf /Applications/ariata-mac.app
	@sudo cp -R apps/mac/build/Build/Products/Debug/mac.app /Applications/ariata-mac.app
	@echo "⚠️  Debug build may crash due to code signing - use 'make mac' for Release build"
	@echo "✅ Installed to /Applications/ariata-mac.app"
	@echo "💡 Launch with: open /Applications/ariata-mac.app"

# Show Mac app status (daemon, pairing, queue)
mac-status:
	@echo "📊 Mac App Status:"
	@echo ""
	@echo "App Process:"
	@if ps aux | grep -i "ariata-mac.app/Contents/MacOS/mac" | grep -v grep > /dev/null 2>&1; then \
		echo "  ✅ Running (PID: $$(ps aux | grep -i 'ariata-mac.app/Contents/MacOS/mac' | grep -v grep | awk '{print $$2}' | head -1))"; \
	else \
		echo "  ❌ Not running"; \
	fi
	@echo ""
	@echo "Background Service:"
	@if [ -f ~/.ariata/logs/mac-app.log ]; then \
		if tail -50 ~/.ariata/logs/mac-app.log | grep -q "✅ Daemon started successfully"; then \
			LAST_START=$$(tail -100 ~/.ariata/logs/mac-app.log | grep "Daemon started successfully" | tail -1 | cut -d']' -f1 | tr -d '['); \
			echo "  ✅ Running (started: $$LAST_START)"; \
			if tail -50 ~/.ariata/logs/mac-app.log | grep -q "📤 Upload completed successfully"; then \
				LAST_UPLOAD=$$(tail -100 ~/.ariata/logs/mac-app.log | grep "Upload completed successfully" | tail -1 | cut -d']' -f1 | tr -d '['); \
				echo "  📤 Last upload: $$LAST_UPLOAD"; \
			else \
				echo "  ⏳ No uploads yet"; \
			fi; \
		elif tail -50 ~/.ariata/logs/mac-app.log | grep -q "Cannot start"; then \
			ERROR=$$(tail -100 ~/.ariata/logs/mac-app.log | grep "Cannot start" | tail -1 | cut -d']' -f2- | xargs); \
			echo "  ❌ Failed to start: $$ERROR"; \
		else \
			echo "  ❓ Unknown (check log: ~/.ariata/logs/mac-app.log)"; \
		fi; \
	else \
		echo "  ❓ No log file found"; \
	fi
	@echo ""
	@echo "Pairing:"
	@if [ -f ~/.ariata/config.json ]; then \
		echo "  ✅ Paired"; \
		cat ~/.ariata/config.json | jq -r '"  Device: " + .deviceId, "  API: " + .apiEndpoint' 2>/dev/null || cat ~/.ariata/config.json; \
	else \
		echo "  ❌ Not paired"; \
	fi
	@echo ""
	@echo "Queue:"
	@if [ -f ~/.ariata/activity.db ]; then \
		echo "  Events: $$(sqlite3 ~/.ariata/activity.db 'SELECT COUNT(*) FROM events WHERE uploaded = 0;' 2>/dev/null || echo 'N/A')"; \
		echo "  Messages: $$(sqlite3 ~/.ariata/activity.db 'SELECT COUNT(*) FROM messages WHERE uploaded = 0;' 2>/dev/null || echo 'N/A')"; \
	else \
		echo "  ❌ Queue not initialized"; \
	fi
	@echo ""
	@echo "💡 Tip: View full log with: tail -f ~/.ariata/logs/mac-app.log"

# Clean Mac app config and queue (for fresh testing)
mac-clean:
	@echo "🧹 Cleaning Mac app..."
	@pkill -f "/Applications/ariata-mac.app" 2>/dev/null || true
	@if launchctl list | grep -q "com.ariata.mac" 2>/dev/null; then \
		launchctl unload ~/Library/LaunchAgents/com.ariata.mac.plist 2>/dev/null || true; \
	fi
	@rm -rf ~/.ariata
	@rm -f ~/Library/LaunchAgents/com.ariata.mac.plist
	@security delete-generic-password -s "com.ariata.mac" -a "device-token" 2>/dev/null || true
	@echo "✅ Cleaned (permissions must be manually revoked in System Settings)"

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
