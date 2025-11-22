-- Add idempotency constraint to narrative_primitive table
--
-- This migration adds a unique constraint on (start_time, end_time) to prevent
-- duplicate narrative primitives from being created when the pipeline re-runs
-- for overlapping time windows.
--
-- The constraint enables ON CONFLICT DO UPDATE semantics for safe re-runs and backfills.

-- Add unique constraint on temporal boundaries
ALTER TABLE data.narrative_primitive
DROP CONSTRAINT IF EXISTS unique_narrative_primitive_time_range;

ALTER TABLE data.narrative_primitive
ADD CONSTRAINT unique_narrative_primitive_time_range
UNIQUE (start_time, end_time);

-- Create index to support efficient lookups by time range
-- (The unique constraint automatically creates an index, but we make it explicit here)
CREATE INDEX IF NOT EXISTS idx_narrative_primitive_time_range
ON data.narrative_primitive(start_time, end_time);
