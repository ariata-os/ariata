-- Core schema for Ariata
-- This is the minimal foundation - just the sources table

-- Create schema for all ELT operations
CREATE SCHEMA IF NOT EXISTS elt;

-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Enable UUID generation extension
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Sources table: Registered data sources with embedded OAuth tokens
CREATE TABLE IF NOT EXISTS sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type TEXT NOT NULL,  -- 'google', 'notion', 'ios', 'mac'
    name TEXT NOT NULL UNIQUE,  -- User-friendly name like "Personal Gmail"

    -- OAuth credentials (null for device sources)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,

    -- Status tracking
    is_active BOOLEAN DEFAULT true,
    error_message TEXT,
    error_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_sources_type ON sources(type);
CREATE INDEX idx_sources_active ON sources(is_active);
CREATE INDEX idx_sources_token_expires ON sources(token_expires_at) WHERE token_expires_at IS NOT NULL;

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

-- Add helpful comments
COMMENT ON TABLE sources IS 'Authentication boundary for data sources - stores OAuth tokens and device IDs';
COMMENT ON COLUMN sources.type IS 'Source type: google, notion, ios, mac';
COMMENT ON COLUMN sources.name IS 'User-friendly unique name for this source';
COMMENT ON COLUMN sources.access_token IS 'OAuth access token (encrypted in production)';
COMMENT ON COLUMN sources.refresh_token IS 'OAuth refresh token for getting new access tokens';