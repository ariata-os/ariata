-- Google Gmail stream table
-- Stores email messages and threads with full fidelity

CREATE TABLE IF NOT EXISTS stream_google_gmail (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Gmail identifiers
    message_id TEXT NOT NULL,  -- Gmail's message ID
    thread_id TEXT NOT NULL,    -- Conversation thread ID
    history_id TEXT,            -- For incremental sync

    -- Email headers
    subject TEXT,
    snippet TEXT,               -- First 100-200 chars preview
    date TIMESTAMPTZ NOT NULL,  -- Email date from headers

    -- Participants
    from_email TEXT,
    from_name TEXT,
    to_emails TEXT[],           -- Array of recipient emails
    to_names TEXT[],            -- Array of recipient names
    cc_emails TEXT[],
    cc_names TEXT[],
    bcc_emails TEXT[],
    bcc_names TEXT[],
    reply_to TEXT,

    -- Content
    body_plain TEXT,            -- Plain text version
    body_html TEXT,             -- HTML version
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[],    -- MIME types of attachments
    attachment_names TEXT[],    -- Filenames of attachments
    attachment_sizes_bytes INTEGER[], -- Sizes of each attachment

    -- Labels and categories
    labels TEXT[],              -- Gmail labels (INBOX, SENT, etc)
    is_unread BOOLEAN DEFAULT false,
    is_important BOOLEAN DEFAULT false,
    is_starred BOOLEAN DEFAULT false,
    is_draft BOOLEAN DEFAULT false,
    is_sent BOOLEAN DEFAULT false,
    is_trash BOOLEAN DEFAULT false,
    is_spam BOOLEAN DEFAULT false,

    -- Threading
    thread_position INTEGER,    -- Position in thread (1-based)
    thread_message_count INTEGER, -- Total messages in thread

    -- Metadata
    size_bytes INTEGER,         -- Total message size
    internal_date TIMESTAMPTZ,  -- Gmail internal date

    -- Full data backup
    raw_json JSONB,            -- Complete message from Gmail API
    headers JSONB,             -- All email headers as key-value pairs

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Prevent duplicate messages
    UNIQUE(source_id, message_id)
);

-- Indexes for common queries
CREATE INDEX idx_gmail_source ON stream_google_gmail(source_id);
CREATE INDEX idx_gmail_message ON stream_google_gmail(message_id);
CREATE INDEX idx_gmail_thread ON stream_google_gmail(thread_id);
CREATE INDEX idx_gmail_date ON stream_google_gmail(date);
CREATE INDEX idx_gmail_from ON stream_google_gmail(from_email);
CREATE INDEX idx_gmail_subject ON stream_google_gmail(subject);
CREATE INDEX idx_gmail_labels ON stream_google_gmail USING GIN(labels);
CREATE INDEX idx_gmail_unread ON stream_google_gmail(is_unread) WHERE is_unread = true;

-- Index for finding messages in a time range
CREATE INDEX idx_gmail_time_range ON stream_google_gmail(source_id, date DESC);

-- Index for thread reconstruction
CREATE INDEX idx_gmail_thread_position ON stream_google_gmail(thread_id, thread_position);

-- Full text search on subject and snippet
CREATE INDEX idx_gmail_search ON stream_google_gmail USING GIN(to_tsvector('english', coalesce(subject, '') || ' ' || coalesce(snippet, '')));

-- Trigger for updated_at
CREATE TRIGGER stream_google_gmail_updated_at
    BEFORE UPDATE ON stream_google_gmail
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE stream_google_gmail IS 'Gmail messages with full content and metadata';
COMMENT ON COLUMN stream_google_gmail.message_id IS 'Gmail message ID - globally unique';
COMMENT ON COLUMN stream_google_gmail.thread_id IS 'Gmail thread ID for conversation grouping';
COMMENT ON COLUMN stream_google_gmail.history_id IS 'Gmail history ID for incremental sync';
COMMENT ON COLUMN stream_google_gmail.snippet IS 'Preview text from message body';
COMMENT ON COLUMN stream_google_gmail.raw_json IS 'Complete message object from Gmail API for data recovery';
COMMENT ON COLUMN stream_google_gmail.headers IS 'All email headers as JSON object';