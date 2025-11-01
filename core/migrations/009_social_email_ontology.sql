-- Social Email Ontology Table
-- Normalized email schema that combines emails from Gmail, Outlook, and other sources

SET search_path TO elt, public;

-- ============================================================================
-- SOCIAL_EMAIL: Normalized email ontology
-- Transforms source-specific email tables into a unified schema
-- ============================================================================

CREATE TABLE IF NOT EXISTS social_email (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Core email fields
    message_id TEXT NOT NULL,           -- Original provider message ID
    thread_id TEXT,                     -- Conversation thread ID
    subject TEXT,
    body_plain TEXT,
    body_html TEXT,
    snippet TEXT,                       -- Preview text (first ~150 chars)

    -- Timing
    timestamp TIMESTAMPTZ NOT NULL,     -- When email was sent/received

    -- Participants
    from_address TEXT,
    from_name TEXT,
    to_addresses TEXT[],
    to_names TEXT[],
    cc_addresses TEXT[],
    cc_names TEXT[],

    -- Email metadata
    direction TEXT NOT NULL,            -- 'sent', 'received'
    labels TEXT[],                      -- Tags/labels/folders
    is_read BOOLEAN DEFAULT false,
    is_starred BOOLEAN DEFAULT false,
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,

    -- Threading context
    thread_position INTEGER,            -- Position in conversation (1-based)
    thread_message_count INTEGER,       -- Total messages in thread

    -- Source tracking (traceability back to raw stream)
    source_stream_id UUID NOT NULL,     -- FK to source stream table row
    source_table TEXT NOT NULL,         -- Which stream table: 'stream_google_gmail', 'stream_outlook_mail', etc.
    source_provider TEXT NOT NULL,      -- Provider: 'google', 'microsoft', etc.

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT social_email_direction_check
        CHECK (direction IN ('sent', 'received')),

    -- Prevent duplicate emails from same source
    UNIQUE(source_table, message_id)
);

-- Indexes for common queries
CREATE INDEX idx_social_email_timestamp ON social_email(timestamp DESC);
CREATE INDEX idx_social_email_from ON social_email(from_address);
CREATE INDEX idx_social_email_thread ON social_email(thread_id);
CREATE INDEX idx_social_email_source ON social_email(source_stream_id);
CREATE INDEX idx_social_email_provider ON social_email(source_provider);
CREATE INDEX idx_social_email_direction ON social_email(direction);
CREATE INDEX idx_social_email_labels ON social_email USING GIN(labels);
CREATE INDEX idx_social_email_unread ON social_email(is_read) WHERE is_read = false;

-- Combined index for filtering by provider and time
CREATE INDEX idx_social_email_provider_timestamp ON social_email(source_provider, timestamp DESC);

-- Full text search on email content
CREATE INDEX idx_social_email_search ON social_email
    USING GIN(to_tsvector('english',
        coalesce(subject, '') || ' ' ||
        coalesce(body_plain, '') || ' ' ||
        coalesce(snippet, '')
    ));

-- Auto-update updated_at timestamp
CREATE TRIGGER social_email_updated_at
    BEFORE UPDATE ON social_email
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE social_email IS 'Normalized email ontology combining emails from all sources (Gmail, Outlook, etc.)';
COMMENT ON COLUMN social_email.message_id IS 'Original message ID from email provider';
COMMENT ON COLUMN social_email.thread_id IS 'Thread/conversation ID for grouping related emails';
COMMENT ON COLUMN social_email.direction IS 'Whether email was sent or received by user';
COMMENT ON COLUMN social_email.source_stream_id IS 'References the source stream table row (e.g., stream_google_gmail.id)';
COMMENT ON COLUMN social_email.source_table IS 'Name of source stream table this was transformed from';
COMMENT ON COLUMN social_email.source_provider IS 'Email provider: google, microsoft, apple, etc.';
