-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Add stream_name column to sync_logs for per-stream filtering
-- This allows filtering sync history by specific streams instead of just by source

ALTER TABLE sync_logs
ADD COLUMN stream_name TEXT;

-- Add index for efficient filtering
CREATE INDEX idx_sync_logs_stream_name ON sync_logs(stream_name);

-- Add composite index for common query pattern (source_id + stream_name)
CREATE INDEX idx_sync_logs_source_stream ON sync_logs(source_id, stream_name, started_at DESC);
