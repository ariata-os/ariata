-- Stream Checkpoints for Transform Job Progress Tracking
--
-- This migration creates the stream_checkpoints table for tracking transform job progress.
-- Each transform maintains a checkpoint indicating the last timestamp it successfully processed.

-- Create stream_checkpoints table
CREATE TABLE IF NOT EXISTS elt.stream_checkpoints (
    -- Composite primary key on source_id + stream_name + checkpoint_key
    source_id UUID NOT NULL REFERENCES elt.sources(id) ON DELETE CASCADE,
    stream_name TEXT NOT NULL,
    checkpoint_key TEXT NOT NULL,

    -- Checkpoint state
    last_processed_at TIMESTAMPTZ NOT NULL,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (source_id, stream_name, checkpoint_key)
);

-- Index for finding checkpoints by stream
CREATE INDEX idx_stream_checkpoints_stream
    ON elt.stream_checkpoints(stream_name, source_id);

-- Index for finding stale checkpoints (monitoring)
CREATE INDEX idx_stream_checkpoints_updated
    ON elt.stream_checkpoints(updated_at DESC);

-- Comments
COMMENT ON TABLE elt.stream_checkpoints IS 'Transform job checkpoint tracking for incremental processing';
COMMENT ON COLUMN elt.stream_checkpoints.source_id IS 'Source UUID this checkpoint belongs to';
COMMENT ON COLUMN elt.stream_checkpoints.stream_name IS 'Stream name (e.g., "healthkit", "gmail")';
COMMENT ON COLUMN elt.stream_checkpoints.checkpoint_key IS 'Unique identifier for this transform (e.g., "healthkit_to_heart_rate")';
COMMENT ON COLUMN elt.stream_checkpoints.last_processed_at IS 'Maximum timestamp of records successfully processed by this transform';
