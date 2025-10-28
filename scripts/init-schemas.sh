#!/bin/bash
set -e

echo "Creating PostgreSQL schemas for Ariata..."

# Connect to the ariata database and create schemas
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create schemas
    CREATE SCHEMA IF NOT EXISTS elt;
    CREATE SCHEMA IF NOT EXISTS app;

    -- Grant permissions
    GRANT ALL ON SCHEMA elt TO $POSTGRES_USER;
    GRANT ALL ON SCHEMA app TO $POSTGRES_USER;

    -- Enable pgvector extension (must be in public or specific schema)
    CREATE EXTENSION IF NOT EXISTS vector;

    -- Set default search_path to include all schemas
    ALTER DATABASE $POSTGRES_DB SET search_path TO elt, app, public;
EOSQL

echo "✅ Schemas created successfully:"
echo "   - elt (ELT data - managed by Rust)"
echo "   - app (Application state - managed by SvelteKit)"
echo "   - Future: transform (Transformed data - managed by Python)"
