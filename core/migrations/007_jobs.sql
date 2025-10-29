-- Jobs table for async job tracking
-- Replaces sync_logs for active job tracking
-- sync_logs remains as immutable audit trail

-- Use the elt schema
SET search_path TO elt, public;

-- ============================================================================
-- JOBS: Unified async job tracking system
-- Supports sync jobs, transform jobs, and future job types
-- ============================================================================

CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Job classification
    job_type TEXT NOT NULL,  -- 'sync', 'transform'
    status TEXT NOT NULL,     -- 'pending', 'running', 'succeeded', 'failed', 'cancelled'

    -- Sync job fields
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE,
    stream_name TEXT,
    sync_strategy TEXT,  -- 'full_refresh', 'incremental'

    -- Transform job fields (future use)
    transform_id UUID,
    transform_strategy TEXT,

    -- Job tracking
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    records_processed BIGINT DEFAULT 0,
    error_message TEXT,
    error_class TEXT,

    -- Flexible metadata storage for job-specific data
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT jobs_type_check
        CHECK (job_type IN ('sync', 'transform')),
    CONSTRAINT jobs_status_check
        CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')),
    CONSTRAINT jobs_sync_strategy_check
        CHECK (sync_strategy IS NULL OR sync_strategy IN ('full_refresh', 'incremental'))
);

-- Prevent concurrent syncs of the same stream
-- This is a partial unique index that only applies to active jobs
CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_active_stream
ON jobs(source_id, stream_name)
WHERE status IN ('pending', 'running') AND job_type = 'sync';

-- Fast lookups for active jobs
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_source_id ON jobs(source_id);
CREATE INDEX IF NOT EXISTS idx_jobs_source_stream ON jobs(source_id, stream_name);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_type_status ON jobs(job_type, status);

-- Query by source and status (for job history)
CREATE INDEX IF NOT EXISTS idx_jobs_source_status_created
ON jobs(source_id, status, created_at DESC);

-- Auto-update updated_at timestamp
CREATE TRIGGER jobs_updated_at
    BEFORE UPDATE ON jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- Migration Notes:
-- - sync_logs table is kept for historical audit trail
-- - jobs table is for active job tracking and recent history
-- - On job completion, data is also written to sync_logs for audit purposes
-- ============================================================================
