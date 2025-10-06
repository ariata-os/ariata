-- Sync logs table for observability and audit trail
-- Stores the history of every sync operation across all sources

CREATE TABLE IF NOT EXISTS sync_logs (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Sync metadata
    sync_mode TEXT NOT NULL,  -- 'full_refresh' or 'incremental'
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,

    -- Results
    status TEXT NOT NULL,  -- 'success', 'failed', 'partial'
    records_fetched INTEGER DEFAULT 0,
    records_written INTEGER DEFAULT 0,
    records_failed INTEGER DEFAULT 0,

    -- Error tracking
    error_message TEXT,
    error_class TEXT,  -- 'auth_error', 'rate_limit', 'sync_token_error', 'server_error', 'client_error', 'network_error'

    -- Cursor/token management for incremental syncs
    sync_cursor_before TEXT,  -- Token/cursor before this sync
    sync_cursor_after TEXT,   -- Token/cursor after this sync (for next run)

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_sync_logs_source ON sync_logs(source_id, started_at DESC);
CREATE INDEX idx_sync_logs_status ON sync_logs(status);
CREATE INDEX idx_sync_logs_started ON sync_logs(started_at DESC);
CREATE INDEX idx_sync_logs_source_status ON sync_logs(source_id, status, started_at DESC);

-- Helpful comments
COMMENT ON TABLE sync_logs IS 'Audit trail of all sync operations for observability and debugging';
COMMENT ON COLUMN sync_logs.sync_mode IS 'full_refresh (replace all) or incremental (fetch only new/changed)';
COMMENT ON COLUMN sync_logs.status IS 'success (completed successfully), failed (error occurred), partial (some records failed)';
COMMENT ON COLUMN sync_logs.duration_ms IS 'Total sync duration in milliseconds';
COMMENT ON COLUMN sync_logs.error_class IS 'Classification of error for monitoring: auth_error, rate_limit, sync_token_error, server_error, client_error, network_error';
COMMENT ON COLUMN sync_logs.sync_cursor_before IS 'Sync token/cursor at start (null for full refresh)';
COMMENT ON COLUMN sync_logs.sync_cursor_after IS 'Sync token/cursor returned from API (null if not available)';
