-- Sync schedule configuration for periodic source syncs
-- Stores cron-based scheduling information for each source

CREATE TABLE IF NOT EXISTS sync_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Cron expression (e.g., "0 */5 * * * *" for every 5 minutes)
    cron_expression TEXT NOT NULL,

    -- Whether this schedule is active
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- Tracking information
    last_run TIMESTAMPTZ,
    next_run TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Only one schedule per source
    UNIQUE(source_id)
);

-- Indexes for performance
CREATE INDEX idx_sync_schedules_enabled ON sync_schedules(enabled);
CREATE INDEX idx_sync_schedules_next_run ON sync_schedules(next_run) WHERE enabled = true;
CREATE INDEX idx_sync_schedules_source ON sync_schedules(source_id);

-- Trigger for updated_at
CREATE TRIGGER sync_schedules_updated_at
    BEFORE UPDATE ON sync_schedules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE sync_schedules IS 'Cron-based scheduling configuration for periodic source syncs';
COMMENT ON COLUMN sync_schedules.cron_expression IS 'Standard cron expression for scheduling (6 fields including seconds)';
COMMENT ON COLUMN sync_schedules.enabled IS 'Whether this schedule is currently active';
COMMENT ON COLUMN sync_schedules.last_run IS 'Timestamp of the last successful sync triggered by this schedule';
COMMENT ON COLUMN sync_schedules.next_run IS 'Calculated timestamp for the next scheduled sync';
