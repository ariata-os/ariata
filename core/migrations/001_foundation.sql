CREATE SCHEMA IF NOT EXISTS data;
CREATE SCHEMA IF NOT EXISTS app;

-- Note: search_path is set at database level via init-schemas.sh
-- Do not use SET search_path in migrations - it breaks SQLx metadata operations

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS vector;

CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
