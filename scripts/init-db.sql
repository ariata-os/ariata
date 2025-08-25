-- Install extensions
CREATE EXTENSION IF NOT EXISTS postgis;
-- Note: pgvector would need to be installed separately if needed for embeddings

-- Grant permissions
GRANT ALL ON SCHEMA public TO ariata_user;