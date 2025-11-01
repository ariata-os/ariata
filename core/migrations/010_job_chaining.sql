-- Add job chaining support for multi-stage transforms
-- Enables workflows like: audio → transcription → structured primitives

-- Use the elt schema
SET search_path TO elt, public;

-- ============================================================================
-- ALTER JOBS TABLE: Add chaining fields
-- ============================================================================

-- Add parent_job_id to track job chains
ALTER TABLE jobs
ADD COLUMN IF NOT EXISTS parent_job_id UUID REFERENCES jobs(id) ON DELETE CASCADE;

-- Add transform_stage to identify stage in multi-stage transforms
-- Examples: 'transcription', 'structuring', 'entity_resolution'
ALTER TABLE jobs
ADD COLUMN IF NOT EXISTS transform_stage TEXT;

-- Add index for finding child jobs
CREATE INDEX IF NOT EXISTS idx_jobs_parent_id ON jobs(parent_job_id);

-- Add composite index for job chains
CREATE INDEX IF NOT EXISTS idx_jobs_parent_stage ON jobs(parent_job_id, transform_stage);

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON COLUMN jobs.parent_job_id IS 'Reference to parent job in a chain (e.g., transcription job spawns structuring job)';
COMMENT ON COLUMN jobs.transform_stage IS 'Stage identifier for multi-stage transforms (e.g., ''transcription'', ''structuring'', ''entity_resolution'')';

-- ============================================================================
-- Migration Notes:
-- - parent_job_id enables chaining: Job A completes → creates Job B with parent_job_id = A.id
-- - transform_stage helps identify which stage of a multi-stage pipeline a job belongs to
-- - Example chain:
--   Job 1 (stage='transcription'): audio → content_transcription
--   Job 2 (stage='structuring', parent_job_id=Job1.id): content_transcription → multiple primitives
-- - Cascade delete ensures child jobs are removed if parent is deleted
-- ============================================================================
