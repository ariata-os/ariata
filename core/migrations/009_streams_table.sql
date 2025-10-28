-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Streams table: Track enabled streams, schedules, and stream-specific config
-- Separates source authentication (sources table) from stream configuration

CREATE TABLE IF NOT EXISTS streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Reference to parent source
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Stream identification
    stream_name TEXT NOT NULL,  -- 'calendar', 'gmail', 'healthkit', 'location'
    table_name TEXT NOT NULL,   -- 'stream_google_calendar', 'stream_ios_healthkit'

    -- Stream enablement and scheduling
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    cron_schedule TEXT,  -- e.g., '0 */6 * * *' for every 6 hours (null = manual only)

    -- Stream-specific configuration
    config JSONB NOT NULL DEFAULT '{}',  -- Stream-specific settings (calendars to sync, etc)

    -- Sync state tracking
    last_sync_token TEXT,  -- Incremental sync cursor/token
    last_sync_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(source_id, stream_name)
);

-- Indexes for common queries
CREATE INDEX idx_streams_source_id ON streams(source_id);
CREATE INDEX idx_streams_enabled ON streams(is_enabled) WHERE is_enabled = true;
CREATE INDEX idx_streams_scheduled ON streams(cron_schedule) WHERE cron_schedule IS NOT NULL;

-- Trigger to update updated_at
CREATE TRIGGER streams_updated_at
    BEFORE UPDATE ON streams
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments for documentation
COMMENT ON TABLE streams IS 'Enabled streams per source with scheduling and configuration';
COMMENT ON COLUMN streams.stream_name IS 'Stream identifier (e.g., calendar, gmail, healthkit)';
COMMENT ON COLUMN streams.table_name IS 'Database table name following stream_{source}_{stream} pattern';
COMMENT ON COLUMN streams.cron_schedule IS 'Cron expression for automatic sync (null = manual sync only)';
COMMENT ON COLUMN streams.config IS 'Stream-specific configuration (calendars to sync, filters, etc)';
COMMENT ON COLUMN streams.last_sync_token IS 'Token/cursor for incremental sync (provider-specific format)';
