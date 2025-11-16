-- Fix Gmail multi-account deduplication bug
--
-- Problem: Current UNIQUE constraint on (source_table, message_id) breaks when users
-- add multiple Gmail accounts (e.g., personal@gmail.com + work@gmail.com) because
-- all Gmail sources use the same source_table value ("stream_google_gmail").
--
-- Solution: Change to UNIQUE (source_stream_id) which aligns with Calendar pattern
-- and allows the same message to exist across different Gmail accounts.
--
-- Each Gmail account has its own unique source_stream_id, so the same email
-- appearing in both personal and work accounts will have different source_stream_id
-- values and won't conflict.

-- Drop the existing problematic constraint
ALTER TABLE elt.social_email
DROP CONSTRAINT IF EXISTS social_email_unique_source;

-- Add new constraint using source_stream_id (aligns with Calendar pattern)
ALTER TABLE elt.social_email
ADD CONSTRAINT social_email_unique_source UNIQUE (source_stream_id);

-- Note: This allows the same message_id to appear multiple times in the table
-- (once per Gmail account), but ensures each stream record only appears once.
-- This is the correct behavior for multi-account Gmail support.
