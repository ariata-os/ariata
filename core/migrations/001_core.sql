-- Core Infrastructure for Ariata ELT System
-- Includes: schema, extensions, sources, streams, jobs, sync_schedules

-- Create schema for all ELT operations
CREATE SCHEMA IF NOT EXISTS elt;

-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS postgis;

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- SOURCES: Authentication boundary for all data sources
-- Supports both OAuth (Google, Notion) and Device (iOS, Mac) authentication
-- Provider field identifies the platform (ios, google, notion, mac)
-- ============================================================================

CREATE TABLE IF NOT EXISTS sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,
    name TEXT NOT NULL UNIQUE,

    -- OAuth credentials (null for device sources)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,

    -- Device authentication (null for OAuth sources)
    auth_type TEXT NOT NULL DEFAULT 'oauth2',
    device_id TEXT,
    device_info JSONB,
    device_token TEXT,
    pairing_code TEXT,
    pairing_status TEXT,
    code_expires_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ,

    -- Status tracking
    is_active BOOLEAN DEFAULT true,
    is_internal BOOLEAN DEFAULT false,  -- Marks system/internal sources (e.g., ariata-app)
    error_message TEXT,
    error_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT sources_auth_type_check
      CHECK (auth_type IN ('oauth2', 'device', 'api_key', 'none')),
    CONSTRAINT sources_pairing_status_check
      CHECK (pairing_status IS NULL OR pairing_status IN ('pending', 'active', 'revoked'))
);

-- Device-specific indexes
CREATE UNIQUE INDEX idx_sources_device_id ON sources(device_id)
  WHERE device_id IS NOT NULL;

CREATE UNIQUE INDEX idx_sources_device_token ON sources(device_token)
  WHERE device_token IS NOT NULL;

CREATE UNIQUE INDEX idx_sources_pairing_code ON sources(pairing_code)
  WHERE pairing_code IS NOT NULL AND pairing_status = 'pending';

-- Trigger to update updated_at on any change
CREATE TRIGGER sources_updated_at
    BEFORE UPDATE ON sources
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE sources IS 'Authentication boundary for all data sources (OAuth and device)';

-- ============================================================================
-- STREAMS: Track enabled streams, schedules, and stream-specific config
-- ============================================================================

CREATE TABLE IF NOT EXISTS streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    stream_name TEXT NOT NULL,
    table_name TEXT NOT NULL,

    is_enabled BOOLEAN NOT NULL DEFAULT true,
    cron_schedule TEXT,

    config JSONB NOT NULL DEFAULT '{}',

    last_sync_token TEXT,
    last_sync_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, stream_name)
);

CREATE TRIGGER streams_updated_at
    BEFORE UPDATE ON streams
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE streams IS 'Tracks enabled streams, schedules, and stream-specific config';

-- ============================================================================
-- JOBS: Job queue for async processing (sync jobs, transform jobs, etc.)
-- ============================================================================

CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',

    -- Sync job fields
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE,
    stream_name TEXT,
    sync_mode TEXT,

    -- Transform job fields
    transform_id UUID,
    transform_strategy TEXT,

    -- Job chaining
    parent_job_id UUID REFERENCES jobs(id) ON DELETE CASCADE,
    transform_stage TEXT,

    -- Tracking
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    records_processed BIGINT NOT NULL DEFAULT 0,
    error_message TEXT,
    error_class TEXT,

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT jobs_status_check
        CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled'))
);

CREATE INDEX idx_jobs_pending ON jobs(created_at ASC)
    WHERE status = 'pending';
CREATE INDEX idx_jobs_parent ON jobs(parent_job_id)
    WHERE parent_job_id IS NOT NULL;

CREATE TRIGGER jobs_updated_at
    BEFORE UPDATE ON jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE jobs IS 'Job queue for async processing (sync jobs, transform jobs, etc.)';

-- ============================================================================
-- STREAM OBJECTS: S3/MinIO object storage metadata
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source and stream identification
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
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
CREATE INDEX idx_stream_objects_source_stream ON stream_objects(source_id, stream_name);
CREATE INDEX idx_stream_objects_timestamp_range ON stream_objects(source_id, stream_name, min_timestamp, max_timestamp);
CREATE INDEX idx_stream_objects_created_at ON stream_objects(created_at);

COMMENT ON TABLE stream_objects IS 'Metadata for stream data stored in S3/MinIO object storage. Each row represents a JSONL file containing batched stream records.';
COMMENT ON COLUMN stream_objects.s3_key IS 'S3 object key following pattern: streams/{source_id}/{stream_name}/date={YYYY-MM-DD}/records_{timestamp}.jsonl';
COMMENT ON COLUMN stream_objects.record_count IS 'Number of JSON records in this JSONL file. Used for monitoring and validation.';
COMMENT ON COLUMN stream_objects.min_timestamp IS 'Earliest timestamp of records in this object. Used for efficient time-range queries.';
COMMENT ON COLUMN stream_objects.max_timestamp IS 'Latest timestamp of records in this object. Used for efficient time-range queries.';

-- ============================================================================
-- STREAM TRANSFORM CHECKPOINTS: Track which S3 objects have been transformed
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_transform_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Identifies which transform this checkpoint is for
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    transform_name TEXT NOT NULL,

    -- Checkpoint state
    last_processed_s3_key TEXT,
    last_processed_timestamp TIMESTAMPTZ,
    last_processed_object_id UUID REFERENCES stream_objects(id) ON DELETE SET NULL,

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
CREATE INDEX idx_transform_checkpoints_lookup ON stream_transform_checkpoints(source_id, stream_name, transform_name);
CREATE INDEX idx_transform_checkpoints_last_run ON stream_transform_checkpoints(last_run_at);

COMMENT ON TABLE stream_transform_checkpoints IS 'Tracks transform job progress when reading from S3. Replaces LEFT JOIN pattern used with Postgres stream tables.';
COMMENT ON COLUMN stream_transform_checkpoints.last_processed_s3_key IS 'Last S3 object key that was fully processed by this transform. Resume from next object on restart.';
COMMENT ON COLUMN stream_transform_checkpoints.last_processed_timestamp IS 'Timestamp of last record processed. Used to avoid reprocessing and for monitoring.';
COMMENT ON COLUMN stream_transform_checkpoints.records_processed IS 'Total count of stream records transformed. Used for monitoring and analytics.';

-- ============================================================================
-- ARCHIVE JOBS: Async S3 archival tracking
-- ============================================================================

CREATE TABLE IF NOT EXISTS archive_jobs (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- References (sync_job_id nullable to support device ingests)
    sync_job_id UUID REFERENCES jobs(id) ON DELETE CASCADE,
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Stream metadata
    stream_name TEXT NOT NULL,
    s3_key TEXT NOT NULL,

    -- Job status
    status TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),
    error_message TEXT,

    -- Retry tracking
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,

    -- Record metadata
    record_count INT NOT NULL DEFAULT 0,
    size_bytes BIGINT NOT NULL DEFAULT 0,

    -- Timestamps for archival window
    min_timestamp TIMESTAMPTZ,
    max_timestamp TIMESTAMPTZ,

    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- Constraints
    CONSTRAINT valid_status_timestamps CHECK (
        (status = 'in_progress' AND started_at IS NOT NULL) OR
        (status = 'completed' AND started_at IS NOT NULL AND completed_at IS NOT NULL) OR
        (status IN ('pending', 'failed'))
    )
);

-- Index for finding pending jobs (job executor query)
CREATE INDEX idx_archive_jobs_pending
    ON archive_jobs(status, created_at)
    WHERE status IN ('pending', 'failed');

-- Index for monitoring by source
CREATE INDEX idx_archive_jobs_source
    ON archive_jobs(source_id, stream_name, created_at DESC);

-- Index for finding jobs by sync_job_id
CREATE INDEX idx_archive_jobs_sync_job
    ON archive_jobs(sync_job_id)
    WHERE sync_job_id IS NOT NULL;

-- Index for finding stale jobs (alerting)
CREATE INDEX idx_archive_jobs_stale
    ON archive_jobs(created_at)
    WHERE status IN ('pending', 'in_progress');

-- Add archive_job_id reference to stream_objects
ALTER TABLE stream_objects
ADD COLUMN IF NOT EXISTS archive_job_id UUID REFERENCES archive_jobs(id) ON DELETE SET NULL;

-- Index for finding stream objects by archive job
CREATE INDEX IF NOT EXISTS idx_stream_objects_archive_job
    ON stream_objects(archive_job_id)
    WHERE archive_job_id IS NOT NULL;

COMMENT ON TABLE archive_jobs IS 'Async S3 archival job tracking for direct transform pipeline';
COMMENT ON COLUMN archive_jobs.id IS 'Unique archive job identifier';
COMMENT ON COLUMN archive_jobs.sync_job_id IS 'Parent sync job that created this archive job (NULL for device ingests)';
COMMENT ON COLUMN archive_jobs.source_id IS 'Source UUID for this archive';
COMMENT ON COLUMN archive_jobs.stream_name IS 'Stream name (e.g., "app_export", "healthkit")';
COMMENT ON COLUMN archive_jobs.s3_key IS 'S3 object key where data will be archived';
COMMENT ON COLUMN archive_jobs.status IS 'Job status: pending, in_progress, completed, failed';
COMMENT ON COLUMN archive_jobs.retry_count IS 'Number of retry attempts for this job';
COMMENT ON COLUMN archive_jobs.record_count IS 'Number of records to archive';
COMMENT ON COLUMN archive_jobs.size_bytes IS 'Total size of records in bytes';
COMMENT ON COLUMN archive_jobs.min_timestamp IS 'Earliest record timestamp in batch';
COMMENT ON COLUMN archive_jobs.max_timestamp IS 'Latest record timestamp in batch';
COMMENT ON COLUMN stream_objects.archive_job_id IS 'Archive job that created this S3 object (NULL for legacy synchronous writes)';

-- ============================================================================
-- BOOTSTRAP DATA: Internal sources
-- ============================================================================

-- Insert ariata source (idempotent)
-- Using a deterministic UUID so it's consistent across all instances
INSERT INTO sources (
    id,
    provider,
    name,
    auth_type,
    is_active,
    is_internal,
    created_at,
    updated_at
)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'ariata',
    'ariata-app',
    'none',
    true,
    true,
    NOW(),
    NOW()
)
ON CONFLICT (id) DO NOTHING;

-- Insert ai_chat stream (idempotent)
INSERT INTO streams (
    id,
    source_id,
    stream_name,
    table_name,
    is_enabled,
    created_at,
    updated_at
)
SELECT
    gen_random_uuid(),
    '00000000-0000-0000-0000-000000000001'::uuid,
    'ai_chat',
    'stream_ariata_ai_chat',
    true,
    NOW(),
    NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM streams
    WHERE source_id = '00000000-0000-0000-0000-000000000001'::uuid
    AND stream_name = 'ai_chat'
);
