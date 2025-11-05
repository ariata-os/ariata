-- Migration: App Schema for Chat Sessions
--
-- Creates operational schema for fast chat UI queries
-- Separates operational (app) from analytical (elt) concerns

-- Create app schema
CREATE SCHEMA IF NOT EXISTS app;

-- ============================================================================
-- APP SCHEMA: Operational Tables
-- ============================================================================

-- Chat sessions with JSONB messages array
-- This is optimized for UI queries (session list, conversation history)
CREATE TABLE app.chat_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    messages JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_count INT NOT NULL DEFAULT 0
);

-- Index for session list (sorted by most recent activity)
CREATE INDEX idx_chat_sessions_updated ON app.chat_sessions(updated_at DESC);

-- Index for GIN queries on JSONB messages (for future full-text search)
CREATE INDEX idx_chat_sessions_messages_gin ON app.chat_sessions USING gin(messages);

-- JSONB message structure example:
-- [
--   {
--     "role": "user",
--     "content": "What is my heart rate trend?",
--     "timestamp": "2025-01-05T12:34:56Z",
--     "model": null
--   },
--   {
--     "role": "assistant",
--     "content": "Based on your health data...",
--     "timestamp": "2025-01-05T12:34:58Z",
--     "model": "claude-sonnet-4-20250514",
--     "provider": "anthropic",
--     "tool_calls": [...]  -- Future: record tool usage
--   }
-- ]

COMMENT ON SCHEMA app IS 'Operational schema for fast UI queries';
COMMENT ON TABLE app.chat_sessions IS 'Chat sessions with denormalized JSONB messages for instant UI loading';
COMMENT ON COLUMN app.chat_sessions.messages IS 'JSONB array of message objects (role, content, timestamp, model, provider)';
COMMENT ON COLUMN app.chat_sessions.message_count IS 'Cached count of messages for performance';

-- ============================================================================
-- ELT SCHEMA: Register Internal Source and Export Stream
-- ============================================================================

-- Register ariata_app as internal source
-- This represents the web application itself as a data source
INSERT INTO elt.sources (id, provider, name, auth_type, is_active, is_internal, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'ariata_app',
    'Ariata Web App',
    'none',
    true,
    true,  -- Mark as internal source
    NOW(),
    NOW()
)
ON CONFLICT (id) DO UPDATE
SET provider = EXCLUDED.provider,
    name = EXCLUDED.name,
    auth_type = EXCLUDED.auth_type,
    is_internal = EXCLUDED.is_internal,
    updated_at = NOW();

-- Register app_export stream
-- This stream exports from app.chat_sessions → elt.stream_ariata_ai_chat
-- Uses cursor-based incremental sync (updated_at timestamp)
INSERT INTO elt.streams (
    source_id,
    stream_name,
    table_name,
    is_enabled,
    cron_schedule,
    config,
    last_sync_token,
    created_at,
    updated_at
)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'app_export',
    'stream_ariata_ai_chat',
    true,
    '*/5 * * * *',  -- Every 5 minutes
    '{}'::jsonb,
    '1970-01-01T00:00:00Z',  -- Initial cursor (epoch timestamp)
    NOW(),
    NOW()
)
ON CONFLICT (source_id, stream_name) DO UPDATE
SET is_enabled = EXCLUDED.is_enabled,
    cron_schedule = EXCLUDED.cron_schedule,
    updated_at = NOW();

-- ============================================================================
-- DATA MIGRATION (if needed)
-- ============================================================================

-- Note: If you have existing chat data in elt.knowledge_ai_conversation,
-- you may want to backfill it into app.chat_sessions.
--
-- This can be done in a follow-up migration or manual script.
-- For now, we start fresh with the new architecture.
