-- ============================================================================
-- 002_app_schema.sql
-- Application Schema - Operational tables for web UI
-- ============================================================================
-- Created: 2025-11-13
-- Description: Creates the `app` schema for operational data used by the web
--              application (chat sessions, preferences, dashboards, etc.)
--              Separate from `elt` schema (data warehouse/analytics).
-- ============================================================================

-- Create app schema
CREATE SCHEMA IF NOT EXISTS app;

-- ============================================================================
-- Chat Sessions Table
-- ============================================================================
-- Stores chat conversations with denormalized JSONB messages array
-- Optimized for fast session list queries and single-query conversation loading

CREATE TABLE IF NOT EXISTS app.chat_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    messages JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_count INTEGER NOT NULL DEFAULT 0
);

-- Index for listing sessions by recency
CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_at
    ON app.chat_sessions(updated_at DESC);

-- Index for message count filtering/sorting
CREATE INDEX IF NOT EXISTS idx_chat_sessions_message_count
    ON app.chat_sessions(message_count);

-- ============================================================================
-- Preferences Table
-- ============================================================================
-- Key-value store for user settings (name, system prompt, etc.)

CREATE TABLE IF NOT EXISTS app.preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Dashboards Table
-- ============================================================================
-- Saved dashboards and visualizations

CREATE TABLE IF NOT EXISTS app.dashboards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT,
    layout TEXT NOT NULL, -- JSON string with widget positions
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for default dashboard lookup
CREATE INDEX IF NOT EXISTS idx_dashboards_is_default
    ON app.dashboards(is_default)
    WHERE is_default = true;

-- ============================================================================
-- Saved Queries Table
-- ============================================================================
-- Saved queries for exploring data

CREATE TABLE IF NOT EXISTS app.saved_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT,
    query TEXT NOT NULL, -- SQL query string
    source_id TEXT, -- Optional: associated source from elt schema
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for source_id lookups
CREATE INDEX IF NOT EXISTS idx_saved_queries_source_id
    ON app.saved_queries(source_id)
    WHERE source_id IS NOT NULL;

-- ============================================================================
-- Recently Viewed Table
-- ============================================================================
-- Recently viewed sources (for quick access in UI)

CREATE TABLE IF NOT EXISTS app.recently_viewed (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id TEXT NOT NULL, -- References elt.sources.id (soft reference)
    source_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    viewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for recent views lookup (sorted by recency)
CREATE INDEX IF NOT EXISTS idx_recently_viewed_viewed_at
    ON app.recently_viewed(viewed_at DESC);

-- Index for deduplication queries
CREATE INDEX IF NOT EXISTS idx_recently_viewed_source_id
    ON app.recently_viewed(source_id);

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON SCHEMA app IS 'Application operational schema - fast queries for web UI';
COMMENT ON TABLE app.chat_sessions IS 'Chat conversation history with denormalized messages array';
COMMENT ON TABLE app.preferences IS 'User preferences key-value store';
COMMENT ON TABLE app.dashboards IS 'Saved dashboard configurations';
COMMENT ON TABLE app.saved_queries IS 'User-saved SQL queries for data exploration';
COMMENT ON TABLE app.recently_viewed IS 'Recently accessed sources for quick navigation';
