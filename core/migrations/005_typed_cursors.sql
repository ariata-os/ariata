-- Add typed cursor support to sources table
-- Breaking change: removes backwards compatibility with string cursors

-- Add cursor_type and cursor_metadata columns
ALTER TABLE sources ADD COLUMN IF NOT EXISTS cursor_type TEXT;
ALTER TABLE sources ADD COLUMN IF NOT EXISTS cursor_metadata JSONB;

-- Drop the old last_sync_token column (BREAKING CHANGE)
-- Since we're in "build and break" mode, we're not preserving old data
ALTER TABLE sources DROP COLUMN IF EXISTS last_sync_token;

-- Add index for querying by cursor type
CREATE INDEX IF NOT EXISTS idx_sources_cursor_type ON sources(cursor_type) WHERE cursor_type IS NOT NULL;

-- Add helpful comments
COMMENT ON COLUMN sources.cursor_type IS 'Type of sync cursor: google_sync_token, notion_start_cursor, strava_timestamp, etc';
COMMENT ON COLUMN sources.cursor_metadata IS 'Typed cursor with metadata: {cursor_type: string, value: string, created_at: timestamp}';
