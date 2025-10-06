-- Initialize PostgreSQL extensions for Ariata
-- This script runs on first database initialization

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Enable vector similarity search (for embeddings)
CREATE EXTENSION IF NOT EXISTS vector;

-- Enable PostGIS for geospatial queries (for location data)
CREATE EXTENSION IF NOT EXISTS postgis;

-- Log successful initialization
DO $$
BEGIN
    RAISE NOTICE 'Ariata database extensions initialized successfully';
END $$;
