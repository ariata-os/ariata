-- Rename sync_strategy to sync_mode in jobs table
-- This aligns with existing terminology and removes redundant type

-- Use the elt schema
SET search_path TO elt, public;

-- Rename the column
ALTER TABLE jobs RENAME COLUMN sync_strategy TO sync_mode;

-- Update the constraint to match the new column name
ALTER TABLE jobs DROP CONSTRAINT IF EXISTS jobs_sync_strategy_check;
ALTER TABLE jobs ADD CONSTRAINT jobs_sync_mode_check
    CHECK (sync_mode IS NULL OR sync_mode IN ('full_refresh', 'incremental'));

-- No need to migrate data since the values remain the same ('full_refresh', 'incremental')
