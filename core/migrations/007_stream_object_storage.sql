-- Migration 007: Stream Object Storage Metadata
--
-- This migration adds metadata tables to support storing stream data in object storage (S3/MinIO)
-- instead of Postgres. This enables:
-- - Lower storage costs (S3 is ~10x cheaper than Postgres block storage)
-- - Better scalability for high-volume time-series data
-- - Encryption at rest with per-source/stream/date keys
-- - Permanent retention without database bloat
--
-- Architecture:
-- - stream_objects: Tracks S3 objects containing stream data (JSONL files)
-- - stream_transform_checkpoints: Tracks which S3 objects have been transformed
-- - Stream data moved from Postgres tables to S3/MinIO
-- - Ontology data remains in Postgres for fast queries

-- =============================================================================
-- Stream Object Metadata
-- =============================================================================

-- Tracks metadata for stream data stored in S3/MinIO
CREATE TABLE IF NOT EXISTS elt.stream_objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source and stream identification
    source_id UUID NOT NULL REFERENCES elt.sources(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,

    -- S3 object location
    s3_key TEXT NOT NULL UNIQUE,

    -- Object metadata
    record_count INTEGER NOT NULL CHECK (record_count > 0),
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),

    -- Time range of records in this object (for efficient querying)
    min_timestamp TIMESTAMPTZ,
    max_timestamp TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CHECK (min_timestamp IS NULL OR max_timestamp IS NULL OR min_timestamp <= max_timestamp)
);

-- Indexes for efficient lookups
CREATE INDEX idx_stream_objects_source_stream ON elt.stream_objects(source_id, stream_name);
CREATE INDEX idx_stream_objects_timestamp_range ON elt.stream_objects(source_id, stream_name, min_timestamp, max_timestamp);
CREATE INDEX idx_stream_objects_created_at ON elt.stream_objects(created_at);

-- =============================================================================
-- Transform Checkpoints
-- =============================================================================

-- Tracks which stream objects have been processed by transform jobs
-- This replaces the LEFT JOIN pattern that queries Postgres stream tables
CREATE TABLE IF NOT EXISTS elt.stream_transform_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Identifies which transform this checkpoint is for
    source_id UUID NOT NULL REFERENCES elt.sources(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    transform_name TEXT NOT NULL,

    -- Checkpoint state
    last_processed_s3_key TEXT,
    last_processed_timestamp TIMESTAMPTZ,
    last_processed_object_id UUID REFERENCES elt.stream_objects(id) ON DELETE SET NULL,

    -- Statistics
    records_processed BIGINT NOT NULL DEFAULT 0,
    objects_processed BIGINT NOT NULL DEFAULT 0,

    -- Timestamps
    last_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one checkpoint per source/stream/transform combination
    UNIQUE(source_id, stream_name, transform_name)
);

-- Index for checkpoint queries
CREATE INDEX idx_transform_checkpoints_lookup ON elt.stream_transform_checkpoints(source_id, stream_name, transform_name);
CREATE INDEX idx_transform_checkpoints_last_run ON elt.stream_transform_checkpoints(last_run_at);

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON TABLE elt.stream_objects IS
'Metadata for stream data stored in S3/MinIO object storage. Each row represents a JSONL file containing batched stream records.';

COMMENT ON COLUMN elt.stream_objects.s3_key IS
'S3 object key following pattern: streams/{source_id}/{stream_name}/date={YYYY-MM-DD}/records_{timestamp}.jsonl';

COMMENT ON COLUMN elt.stream_objects.record_count IS
'Number of JSON records in this JSONL file. Used for monitoring and validation.';

COMMENT ON COLUMN elt.stream_objects.min_timestamp IS
'Earliest timestamp of records in this object. Used for efficient time-range queries.';

COMMENT ON COLUMN elt.stream_objects.max_timestamp IS
'Latest timestamp of records in this object. Used for efficient time-range queries.';

COMMENT ON TABLE elt.stream_transform_checkpoints IS
'Tracks transform job progress when reading from S3. Replaces LEFT JOIN pattern used with Postgres stream tables.';

COMMENT ON COLUMN elt.stream_transform_checkpoints.last_processed_s3_key IS
'Last S3 object key that was fully processed by this transform. Resume from next object on restart.';

COMMENT ON COLUMN elt.stream_transform_checkpoints.last_processed_timestamp IS
'Timestamp of last record processed. Used to avoid reprocessing and for monitoring.';

COMMENT ON COLUMN elt.stream_transform_checkpoints.records_processed IS
'Total count of stream records transformed. Used for monitoring and analytics.';
