-- Pipeline tables in data schema
-- Note: search_path is set at database level, so we use qualified names

CREATE TABLE IF NOT EXISTS data.source_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source TEXT NOT NULL,
    name TEXT NOT NULL UNIQUE,
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,
    auth_type TEXT NOT NULL DEFAULT 'oauth2',
    device_id TEXT,
    device_info JSONB,
    device_token TEXT,
    pairing_code TEXT,
    pairing_status TEXT,
    code_expires_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    is_internal BOOLEAN DEFAULT false,
    error_message TEXT,
    error_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_connections_auth_type_check
      CHECK (auth_type IN ('oauth2', 'device', 'api_key', 'none')),
    CONSTRAINT source_connections_pairing_status_check
      CHECK (pairing_status IS NULL OR pairing_status IN ('pending', 'active', 'revoked'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_source_connections_source_device ON data.source_connections(source, device_id)
  WHERE device_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_connections_device_token ON data.source_connections(device_token)
  WHERE device_token IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_connections_pairing_code ON data.source_connections(pairing_code)
  WHERE pairing_code IS NOT NULL AND pairing_status = 'pending';

DROP TRIGGER IF EXISTS source_connections_updated_at ON data.source_connections;
CREATE TRIGGER source_connections_updated_at
    BEFORE UPDATE ON data.source_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS data.stream_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_connection_id UUID NOT NULL REFERENCES data.source_connections(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    table_name TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    -- 6-field cron schedule (sec min hour day month dow). Defaults applied by seeding logic from registry.
    -- The scheduler applies registry defaults at startup for any enabled streams missing schedules.
    cron_schedule TEXT,
    config JSONB NOT NULL DEFAULT '{}',
    last_sync_token TEXT,
    last_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_connection_id, stream_name)
);

DROP TRIGGER IF EXISTS stream_connections_updated_at ON data.stream_connections;
CREATE TRIGGER stream_connections_updated_at
    BEFORE UPDATE ON data.stream_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS data.jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    source_connection_id UUID REFERENCES data.source_connections(id) ON DELETE CASCADE,
    stream_name TEXT,
    sync_mode TEXT,
    transform_id UUID,
    transform_strategy TEXT,
    parent_job_id UUID REFERENCES data.jobs(id) ON DELETE CASCADE,
    transform_stage TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    records_processed BIGINT NOT NULL DEFAULT 0,
    error_message TEXT,
    error_class TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT jobs_status_check
        CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_jobs_pending ON data.jobs(created_at ASC)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_jobs_parent ON data.jobs(parent_job_id)
    WHERE parent_job_id IS NOT NULL;

DROP TRIGGER IF EXISTS jobs_updated_at ON data.jobs;
CREATE TRIGGER jobs_updated_at
    BEFORE UPDATE ON data.jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS data.stream_objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_connection_id UUID NOT NULL REFERENCES data.source_connections(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    s3_key TEXT NOT NULL UNIQUE,
    record_count INTEGER NOT NULL CHECK (record_count > 0),
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
    min_timestamp TIMESTAMPTZ,
    max_timestamp TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (min_timestamp IS NULL OR max_timestamp IS NULL OR min_timestamp <= max_timestamp)
);

CREATE INDEX IF NOT EXISTS idx_stream_objects_source_stream ON data.stream_objects(source_connection_id, stream_name);
CREATE INDEX IF NOT EXISTS idx_stream_objects_timestamp_range ON data.stream_objects(source_connection_id, stream_name, min_timestamp, max_timestamp);
CREATE INDEX IF NOT EXISTS idx_stream_objects_created_at ON data.stream_objects(created_at);

CREATE TABLE IF NOT EXISTS data.stream_transform_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_connection_id UUID NOT NULL REFERENCES data.source_connections(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    transform_name TEXT NOT NULL,
    last_processed_s3_key TEXT,
    last_processed_timestamp TIMESTAMPTZ,
    last_processed_object_id UUID REFERENCES data.stream_objects(id) ON DELETE SET NULL,
    records_processed BIGINT NOT NULL DEFAULT 0,
    objects_processed BIGINT NOT NULL DEFAULT 0,
    last_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_connection_id, stream_name, transform_name)
);

CREATE INDEX IF NOT EXISTS idx_transform_checkpoints_lookup ON data.stream_transform_checkpoints(source_connection_id, stream_name, transform_name);
CREATE INDEX IF NOT EXISTS idx_transform_checkpoints_last_run ON data.stream_transform_checkpoints(last_run_at);

CREATE TABLE IF NOT EXISTS data.archive_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sync_job_id UUID REFERENCES data.jobs(id) ON DELETE CASCADE,
    source_connection_id UUID NOT NULL REFERENCES data.source_connections(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    s3_key TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),
    error_message TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,
    record_count INT NOT NULL DEFAULT 0,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    min_timestamp TIMESTAMPTZ,
    max_timestamp TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    CONSTRAINT valid_status_timestamps CHECK (
        (status = 'in_progress' AND started_at IS NOT NULL) OR
        (status = 'completed' AND started_at IS NOT NULL AND completed_at IS NOT NULL) OR
        (status IN ('pending', 'failed'))
    )
);

CREATE INDEX IF NOT EXISTS idx_archive_jobs_pending ON data.archive_jobs(status, created_at)
    WHERE status IN ('pending', 'failed');
CREATE INDEX IF NOT EXISTS idx_archive_jobs_source ON data.archive_jobs(source_connection_id, stream_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_archive_jobs_sync_job ON data.archive_jobs(sync_job_id)
    WHERE sync_job_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_archive_jobs_stale ON data.archive_jobs(created_at)
    WHERE status IN ('pending', 'in_progress');

ALTER TABLE data.stream_objects
ADD COLUMN IF NOT EXISTS archive_job_id UUID REFERENCES data.archive_jobs(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_stream_objects_archive_job ON data.stream_objects(archive_job_id)
    WHERE archive_job_id IS NOT NULL;
