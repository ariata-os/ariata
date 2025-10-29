-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- macOS stream tables for device data ingestion
-- These tables store data from the Mac companion app

-- Mac Applications stream (app usage tracking)
CREATE TABLE IF NOT EXISTS stream_mac_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp when the app was active
    timestamp TIMESTAMPTZ NOT NULL,

    -- Application identification
    app_name TEXT NOT NULL,              -- user-facing app name (e.g., "Chrome")
    bundle_id TEXT,                      -- bundle identifier (e.g., "com.google.Chrome")
    app_version TEXT,                    -- application version

    -- Window information
    window_title TEXT,                   -- current window title
    window_index INTEGER,                -- window number for multi-window apps

    -- Usage metrics
    duration_seconds INTEGER,            -- how long the app was active
    is_frontmost BOOLEAN DEFAULT true,   -- whether app was in foreground

    -- Activity classification
    category TEXT,                       -- productivity, communication, entertainment, development, etc.

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_mac_apps_timestamp ON stream_mac_apps(timestamp DESC);
CREATE INDEX idx_mac_apps_source_time ON stream_mac_apps(source_id, timestamp DESC);
CREATE INDEX idx_mac_apps_bundle ON stream_mac_apps(bundle_id, timestamp DESC);
CREATE INDEX idx_mac_apps_name ON stream_mac_apps(app_name);

-- Idempotency constraint: one app usage record per device per timestamp per app
CREATE UNIQUE INDEX unique_mac_apps_usage
    ON stream_mac_apps(source_id, timestamp, app_name);

-- Comments
COMMENT ON TABLE stream_mac_apps IS 'macOS application usage tracking including active apps and window titles';
COMMENT ON COLUMN stream_mac_apps.app_name IS 'User-facing application name';
COMMENT ON COLUMN stream_mac_apps.bundle_id IS 'macOS bundle identifier for the application';
COMMENT ON COLUMN stream_mac_apps.window_title IS 'Title of the active window (may contain sensitive info)';
COMMENT ON COLUMN stream_mac_apps.duration_seconds IS 'Duration the app was active in seconds';
COMMENT ON COLUMN stream_mac_apps.is_frontmost IS 'Whether the app was in the foreground';
COMMENT ON INDEX unique_mac_apps_usage
    IS 'Ensures idempotent inserts: one app usage record per device per timestamp per app';

-----------------------------------------------------------

-- Mac Browser History stream (browsing activity)
CREATE TABLE IF NOT EXISTS stream_mac_browser (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp of the page visit
    timestamp TIMESTAMPTZ NOT NULL,

    -- Page information
    url TEXT NOT NULL,
    title TEXT,
    domain TEXT,                         -- extracted domain for easier querying

    -- Browser information
    browser TEXT,                        -- safari, chrome, firefox, etc.

    -- Visit metrics
    visit_duration INTEGER,              -- time spent on page in seconds
    transition_type TEXT,                -- link, typed, reload, etc.

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_mac_browser_timestamp ON stream_mac_browser(timestamp DESC);
CREATE INDEX idx_mac_browser_source_time ON stream_mac_browser(source_id, timestamp DESC);
CREATE INDEX idx_mac_browser_domain ON stream_mac_browser(domain, timestamp DESC);
CREATE INDEX idx_mac_browser_url ON stream_mac_browser USING hash(url);

-- Idempotency constraint: one URL visit per device per timestamp per URL
CREATE UNIQUE INDEX unique_mac_browser_visit
    ON stream_mac_browser(source_id, url, timestamp);

-- Comments
COMMENT ON TABLE stream_mac_browser IS 'macOS browser history from Safari, Chrome, Firefox, etc.';
COMMENT ON COLUMN stream_mac_browser.url IS 'Full URL visited';
COMMENT ON COLUMN stream_mac_browser.domain IS 'Extracted domain name for easier filtering';
COMMENT ON COLUMN stream_mac_browser.visit_duration IS 'Time spent on the page in seconds';
COMMENT ON INDEX unique_mac_browser_visit
    IS 'Ensures idempotent inserts: one browser visit per device per URL per timestamp';

-----------------------------------------------------------

-- Mac iMessage stream (message history)
CREATE TABLE IF NOT EXISTS stream_mac_imessage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp of the message
    timestamp TIMESTAMPTZ NOT NULL,

    -- Message content
    message_text TEXT,                   -- message body
    is_from_me BOOLEAN NOT NULL,         -- true if sent by user, false if received

    -- Contact information
    contact_id TEXT,                     -- contact identifier
    contact_name TEXT,                   -- display name
    phone_number TEXT,                   -- phone number or email
    is_group_chat BOOLEAN DEFAULT false,

    -- Message metadata
    is_read BOOLEAN DEFAULT false,
    has_attachment BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[],             -- array of attachment types (image, video, etc.)

    -- Service information
    service TEXT DEFAULT 'iMessage',     -- iMessage or SMS

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_mac_imessage_timestamp ON stream_mac_imessage(timestamp DESC);
CREATE INDEX idx_mac_imessage_source_time ON stream_mac_imessage(source_id, timestamp DESC);
CREATE INDEX idx_mac_imessage_contact ON stream_mac_imessage(contact_id, timestamp DESC);
CREATE INDEX idx_mac_imessage_search ON stream_mac_imessage USING GIN (to_tsvector('english', message_text))
    WHERE message_text IS NOT NULL;

-- Idempotency constraint: one message per device per timestamp per contact per direction
ALTER TABLE stream_mac_imessage
    ADD CONSTRAINT unique_mac_imessage_message
    UNIQUE (source_id, timestamp, contact_id, is_from_me);

-- Comments
COMMENT ON TABLE stream_mac_imessage IS 'macOS iMessage and SMS history';
COMMENT ON CONSTRAINT unique_mac_imessage_message ON stream_mac_imessage
    IS 'Ensures idempotent inserts: one message per device per timestamp per contact per direction';
COMMENT ON COLUMN stream_mac_imessage.message_text IS 'Message body text (may be null for attachment-only messages)';
COMMENT ON COLUMN stream_mac_imessage.is_from_me IS 'True if message was sent by user, false if received';
COMMENT ON COLUMN stream_mac_imessage.contact_name IS 'Display name of contact';

-----------------------------------------------------------

-- Mac Screen Time stream (overall system usage)
CREATE TABLE IF NOT EXISTS stream_mac_screentime (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp (typically start of time period)
    timestamp TIMESTAMPTZ NOT NULL,

    -- Period information
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    period_type TEXT DEFAULT 'hourly',   -- hourly, daily, weekly

    -- Usage metrics
    total_screen_time INTEGER,           -- total active time in seconds
    productive_time INTEGER,             -- productive app usage in seconds
    entertainment_time INTEGER,          -- entertainment app usage in seconds
    communication_time INTEGER,          -- communication app usage in seconds
    unclassified_time INTEGER,           -- unclassified usage in seconds

    -- App breakdown (top apps)
    top_apps JSONB,                      -- array of {app, duration} objects

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Prevent duplicate periods
    UNIQUE(source_id, period_start, period_end)
);

-- Indexes for performance
CREATE INDEX idx_mac_screentime_timestamp ON stream_mac_screentime(timestamp DESC);
CREATE INDEX idx_mac_screentime_source_time ON stream_mac_screentime(source_id, timestamp DESC);
CREATE INDEX idx_mac_screentime_period ON stream_mac_screentime(period_start, period_end);

-- Comments
COMMENT ON TABLE stream_mac_screentime IS 'macOS screen time summaries aggregated by time period';
COMMENT ON COLUMN stream_mac_screentime.total_screen_time IS 'Total active computer usage in seconds';
COMMENT ON COLUMN stream_mac_screentime.productive_time IS 'Time spent in productivity apps (code editors, office apps, etc.)';
COMMENT ON COLUMN stream_mac_screentime.top_apps IS 'JSON array of top apps used during this period';
