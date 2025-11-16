-- Replace rigid type fields with flexible tags in axiology tables
--
-- This migration removes prescriptive type columns (task_type, initiative_type, aspiration_type)
-- and replaces them with a flexible tags array that users can populate however they want.

-- Add tags column to tasks
ALTER TABLE elt.axiology_task
    DROP COLUMN IF EXISTS task_type,
    ADD COLUMN tags TEXT[] DEFAULT '{}';

-- Add GIN index for efficient tag filtering
CREATE INDEX IF NOT EXISTS idx_axiology_task_tags ON elt.axiology_task USING GIN (tags);

-- Add tags column to initiatives
ALTER TABLE elt.axiology_initiative
    DROP COLUMN IF EXISTS initiative_type,
    ADD COLUMN tags TEXT[] DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_axiology_initiative_tags ON elt.axiology_initiative USING GIN (tags);

-- Add tags column to aspirations
ALTER TABLE elt.axiology_aspiration
    DROP COLUMN IF EXISTS aspiration_type,
    ADD COLUMN tags TEXT[] DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_axiology_aspiration_tags ON elt.axiology_aspiration USING GIN (tags);

-- Update comments
COMMENT ON COLUMN elt.axiology_task.tags IS 'User-defined tags for categorizing tasks (e.g., work, creative, spiritual, family)';
COMMENT ON COLUMN elt.axiology_initiative.tags IS 'User-defined tags for categorizing initiatives';
COMMENT ON COLUMN elt.axiology_aspiration.tags IS 'User-defined tags for categorizing aspirations';
