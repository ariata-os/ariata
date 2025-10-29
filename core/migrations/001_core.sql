-- Core schema for Ariata ELT System
-- Includes: sources (with OAuth + device auth), sync_logs, streams, sync_schedules

-- Create schema for all ELT operations
CREATE SCHEMA IF NOT EXISTS elt;

-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Enable UUID generation extension
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- SOURCES: Authentication boundary for all data sources
-- Supports both OAuth (Google, Notion) and Device (iOS, Mac) authentication
-- ============================================================================

CREATE TABLE IF NOT EXISTS sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type TEXT NOT NULL,
    name TEXT NOT NULL UNIQUE,

    -- OAuth credentials (null for device sources)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,

    -- Device authentication (null for OAuth sources)
    auth_type TEXT NOT NULL DEFAULT 'oauth2',
    device_id TEXT,
    device_info JSONB,
    device_token TEXT,
    pairing_code TEXT,
    pairing_status TEXT,
    code_expires_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ,

    -- Status tracking
    is_active BOOLEAN DEFAULT true,
    error_message TEXT,
    error_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT sources_auth_type_check
      CHECK (auth_type IN ('oauth2', 'device', 'api_key', 'none')),
    CONSTRAINT sources_pairing_status_check
      CHECK (pairing_status IS NULL OR pairing_status IN ('pending', 'active', 'revoked'))
);

-- Indexes for common queries
CREATE INDEX idx_sources_type ON sources(type);
CREATE INDEX idx_sources_active ON sources(is_active);
CREATE INDEX idx_sources_auth_type ON sources(auth_type);
CREATE INDEX idx_sources_token_expires ON sources(token_expires_at) WHERE token_expires_at IS NOT NULL;

-- Device-specific indexes
CREATE UNIQUE INDEX idx_sources_device_id ON sources(device_id)
  WHERE device_id IS NOT NULL;

CREATE UNIQUE INDEX idx_sources_device_token ON sources(device_token)
  WHERE device_token IS NOT NULL;

CREATE UNIQUE INDEX idx_sources_pairing_code ON sources(pairing_code)
  WHERE pairing_code IS NOT NULL AND pairing_status = 'pending';

CREATE INDEX idx_sources_pairing_status ON sources(pairing_status)
  WHERE pairing_status = 'pending';

CREATE INDEX idx_sources_code_expires ON sources(code_expires_at)
  WHERE code_expires_at IS NOT NULL AND pairing_status = 'pending';

CREATE INDEX idx_sources_last_seen ON sources(last_seen_at)
  WHERE last_seen_at IS NOT NULL;

-- Function to automatically update updated_at
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update updated_at on any change
CREATE TRIGGER sources_updated_at
    BEFORE UPDATE ON sources
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- STREAMS: Track enabled streams, schedules, and stream-specific config
-- ============================================================================

CREATE TABLE IF NOT EXISTS streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    stream_name TEXT NOT NULL,
    table_name TEXT NOT NULL,

    is_enabled BOOLEAN NOT NULL DEFAULT true,
    cron_schedule TEXT,

    config JSONB NOT NULL DEFAULT '{}',

    last_sync_token TEXT,
    last_sync_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, stream_name)
);

CREATE INDEX idx_streams_source_id ON streams(source_id);
CREATE INDEX idx_streams_enabled ON streams(is_enabled) WHERE is_enabled = true;
CREATE INDEX idx_streams_scheduled ON streams(cron_schedule) WHERE cron_schedule IS NOT NULL;

CREATE TRIGGER streams_updated_at
    BEFORE UPDATE ON streams
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- SYNC SCHEDULES: Cron-based scheduling for periodic source syncs
-- ============================================================================

CREATE TABLE IF NOT EXISTS sync_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    cron_expression TEXT NOT NULL,

    enabled BOOLEAN NOT NULL DEFAULT true,

    last_run TIMESTAMPTZ,
    next_run TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id)
);

CREATE INDEX idx_sync_schedules_enabled ON sync_schedules(enabled);
CREATE INDEX idx_sync_schedules_next_run ON sync_schedules(next_run) WHERE enabled = true;
CREATE INDEX idx_sync_schedules_source ON sync_schedules(source_id);

CREATE TRIGGER sync_schedules_updated_at
    BEFORE UPDATE ON sync_schedules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
