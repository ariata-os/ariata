-- Stream Tables for All Data Sources
-- Google, iOS, Mac, Notion stream tables for raw data ingestion

SET search_path TO elt, public;

-- ============================================================================
-- GOOGLE CALENDAR
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_google_calendar (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Google event identifiers
    event_id TEXT NOT NULL,
    calendar_id TEXT NOT NULL,
    etag TEXT,

    -- Core event data
    summary TEXT,
    description TEXT,
    location TEXT,
    status TEXT,

    -- Timing
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    all_day BOOLEAN DEFAULT false,
    timezone TEXT,

    -- People
    organizer_email TEXT,
    organizer_name TEXT,
    creator_email TEXT,
    creator_name TEXT,
    attendee_count INTEGER DEFAULT 0,

    -- Meeting details
    has_conferencing BOOLEAN DEFAULT false,
    conference_type TEXT,
    conference_link TEXT,

    -- Metadata
    created_by_google TIMESTAMPTZ,
    updated_by_google TIMESTAMPTZ,
    is_recurring BOOLEAN DEFAULT false,
    recurring_event_id TEXT,

    -- Full data backup
    raw_json JSONB,

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, event_id)
);

CREATE INDEX idx_gcal_source ON stream_google_calendar(source_id);
CREATE INDEX idx_gcal_event ON stream_google_calendar(event_id);
CREATE INDEX idx_gcal_start ON stream_google_calendar(start_time);
CREATE INDEX idx_gcal_end ON stream_google_calendar(end_time);
CREATE INDEX idx_gcal_calendar ON stream_google_calendar(calendar_id);
CREATE INDEX idx_gcal_organizer ON stream_google_calendar(organizer_email);
CREATE INDEX idx_gcal_time_range ON stream_google_calendar(source_id, start_time, end_time);

CREATE TRIGGER stream_google_calendar_updated_at
    BEFORE UPDATE ON stream_google_calendar
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE stream_google_calendar IS 'Google Calendar events with full fidelity';

-- ============================================================================
-- GOOGLE GMAIL
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_google_gmail (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Gmail identifiers
    message_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    history_id TEXT,

    -- Email headers
    subject TEXT,
    snippet TEXT,
    date TIMESTAMPTZ NOT NULL,

    -- Participants
    from_email TEXT,
    from_name TEXT,
    to_emails TEXT[],
    to_names TEXT[],
    cc_emails TEXT[],
    cc_names TEXT[],
    bcc_emails TEXT[],
    bcc_names TEXT[],
    reply_to TEXT,

    -- Content
    body_plain TEXT,
    body_html TEXT,
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[],
    attachment_names TEXT[],
    attachment_sizes_bytes INTEGER[],

    -- Labels and categories
    labels TEXT[],
    is_unread BOOLEAN DEFAULT false,
    is_important BOOLEAN DEFAULT false,
    is_starred BOOLEAN DEFAULT false,
    is_draft BOOLEAN DEFAULT false,
    is_sent BOOLEAN DEFAULT false,
    is_trash BOOLEAN DEFAULT false,
    is_spam BOOLEAN DEFAULT false,

    -- Threading
    thread_position INTEGER,
    thread_message_count INTEGER,

    -- Metadata
    size_bytes INTEGER,
    internal_date TIMESTAMPTZ,

    -- Full data backup
    raw_json JSONB,
    headers JSONB,

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, message_id)
);

CREATE INDEX idx_gmail_source ON stream_google_gmail(source_id);
CREATE INDEX idx_gmail_message ON stream_google_gmail(message_id);
CREATE INDEX idx_gmail_thread ON stream_google_gmail(thread_id);
CREATE INDEX idx_gmail_date ON stream_google_gmail(date);
CREATE INDEX idx_gmail_from ON stream_google_gmail(from_email);
CREATE INDEX idx_gmail_subject ON stream_google_gmail(subject);
CREATE INDEX idx_gmail_labels ON stream_google_gmail USING GIN(labels);
CREATE INDEX idx_gmail_unread ON stream_google_gmail(is_unread) WHERE is_unread = true;
CREATE INDEX idx_gmail_time_range ON stream_google_gmail(source_id, date DESC);
CREATE INDEX idx_gmail_thread_position ON stream_google_gmail(thread_id, thread_position);
CREATE INDEX idx_gmail_search ON stream_google_gmail USING GIN(to_tsvector('english', coalesce(subject, '') || ' ' || coalesce(snippet, '')));

CREATE TRIGGER stream_google_gmail_updated_at
    BEFORE UPDATE ON stream_google_gmail
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE stream_google_gmail IS 'Gmail messages with full content and metadata';

-- ============================================================================
-- IOS HEALTHKIT
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_ios_healthkit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Cardiovascular metrics
    heart_rate FLOAT,
    hrv FLOAT,
    resting_heart_rate FLOAT,

    -- Activity metrics
    steps INTEGER,
    distance FLOAT,
    active_energy FLOAT,
    basal_energy FLOAT,
    flights_climbed INTEGER,

    -- Sleep metrics
    sleep_stage TEXT,
    sleep_duration INTEGER,

    -- Workout metrics
    workout_type TEXT,
    workout_duration INTEGER,

    -- Body metrics
    weight FLOAT,
    body_fat_percentage FLOAT,

    -- Mindfulness & Recovery
    mindful_minutes INTEGER,

    -- Device information
    device_name TEXT,
    device_model TEXT,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, timestamp)
);

CREATE INDEX idx_ios_healthkit_timestamp ON stream_ios_healthkit(timestamp DESC);
CREATE INDEX idx_ios_healthkit_source_time ON stream_ios_healthkit(source_id, timestamp DESC);
CREATE INDEX idx_ios_healthkit_heart_rate ON stream_ios_healthkit(timestamp) WHERE heart_rate IS NOT NULL;
CREATE INDEX idx_ios_healthkit_steps ON stream_ios_healthkit(timestamp) WHERE steps IS NOT NULL;
CREATE INDEX idx_ios_healthkit_sleep ON stream_ios_healthkit(timestamp) WHERE sleep_stage IS NOT NULL;
CREATE INDEX idx_ios_healthkit_workout ON stream_ios_healthkit(timestamp) WHERE workout_type IS NOT NULL;

COMMENT ON TABLE stream_ios_healthkit IS 'iOS HealthKit data including heart rate, HRV, steps, sleep, and workouts';

-- ============================================================================
-- IOS LOCATION
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_ios_location (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Coordinates
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,
    altitude FLOAT,

    -- Movement metrics
    speed FLOAT,
    course FLOAT,

    -- Accuracy metrics
    horizontal_accuracy FLOAT,
    vertical_accuracy FLOAT,

    -- Activity inference
    activity_type TEXT,
    activity_confidence TEXT,

    -- Floor level
    floor_level INTEGER,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, timestamp)
);

CREATE INDEX idx_ios_location_timestamp ON stream_ios_location(timestamp DESC);
CREATE INDEX idx_ios_location_source_time ON stream_ios_location(source_id, timestamp DESC);

COMMENT ON TABLE stream_ios_location IS 'iOS location data including GPS coordinates, speed, and activity type';

-- ============================================================================
-- IOS MICROPHONE
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_ios_microphone (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Audio level metrics
    decibels FLOAT,
    average_power FLOAT,
    peak_power FLOAT,

    -- Transcription
    transcription TEXT,
    transcription_confidence FLOAT,
    language TEXT,

    -- Recording metadata
    duration_seconds INTEGER,
    sample_rate INTEGER,

    -- Storage reference
    audio_file_key TEXT,
    audio_file_size INTEGER,
    audio_format TEXT,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, timestamp)
);

CREATE INDEX idx_ios_microphone_timestamp ON stream_ios_microphone(timestamp DESC);
CREATE INDEX idx_ios_microphone_source_time ON stream_ios_microphone(source_id, timestamp DESC);
CREATE INDEX idx_ios_microphone_transcription ON stream_ios_microphone USING GIN (to_tsvector('english', transcription))
    WHERE transcription IS NOT NULL;

COMMENT ON TABLE stream_ios_microphone IS 'iOS microphone data including audio levels and transcriptions';

-- ============================================================================
-- MAC APPS
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_mac_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Application identification
    app_name TEXT NOT NULL,
    bundle_id TEXT,
    app_version TEXT,

    -- Window information
    window_title TEXT,
    window_index INTEGER,

    -- Usage metrics
    duration_seconds INTEGER,
    is_frontmost BOOLEAN DEFAULT true,

    -- Activity classification
    category TEXT,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mac_apps_timestamp ON stream_mac_apps(timestamp DESC);
CREATE INDEX idx_mac_apps_source_time ON stream_mac_apps(source_id, timestamp DESC);
CREATE INDEX idx_mac_apps_bundle ON stream_mac_apps(bundle_id, timestamp DESC);
CREATE INDEX idx_mac_apps_name ON stream_mac_apps(app_name);
CREATE UNIQUE INDEX unique_mac_apps_usage ON stream_mac_apps(source_id, timestamp, app_name);

COMMENT ON TABLE stream_mac_apps IS 'macOS application usage tracking including active apps and window titles';

-- ============================================================================
-- MAC BROWSER
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_mac_browser (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Page information
    url TEXT NOT NULL,
    title TEXT,
    domain TEXT,

    -- Browser information
    browser TEXT,

    -- Visit metrics
    visit_duration INTEGER,
    transition_type TEXT,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mac_browser_timestamp ON stream_mac_browser(timestamp DESC);
CREATE INDEX idx_mac_browser_source_time ON stream_mac_browser(source_id, timestamp DESC);
CREATE INDEX idx_mac_browser_domain ON stream_mac_browser(domain, timestamp DESC);
CREATE INDEX idx_mac_browser_url ON stream_mac_browser USING hash(url);
CREATE UNIQUE INDEX unique_mac_browser_visit ON stream_mac_browser(source_id, url, timestamp);

COMMENT ON TABLE stream_mac_browser IS 'macOS browser history from Safari, Chrome, Firefox, etc.';

-- ============================================================================
-- MAC IMESSAGE
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_mac_imessage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Message content
    message_text TEXT,
    is_from_me BOOLEAN NOT NULL,

    -- Contact information
    contact_id TEXT,
    contact_name TEXT,
    phone_number TEXT,
    is_group_chat BOOLEAN DEFAULT false,

    -- Message metadata
    is_read BOOLEAN DEFAULT false,
    has_attachment BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[],

    -- Service information
    service TEXT DEFAULT 'iMessage',

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, timestamp, contact_id, is_from_me)
);

CREATE INDEX idx_mac_imessage_timestamp ON stream_mac_imessage(timestamp DESC);
CREATE INDEX idx_mac_imessage_source_time ON stream_mac_imessage(source_id, timestamp DESC);
CREATE INDEX idx_mac_imessage_contact ON stream_mac_imessage(contact_id, timestamp DESC);
CREATE INDEX idx_mac_imessage_search ON stream_mac_imessage USING GIN (to_tsvector('english', message_text))
    WHERE message_text IS NOT NULL;

COMMENT ON TABLE stream_mac_imessage IS 'macOS iMessage and SMS history';

-- ============================================================================
-- MAC SCREENTIME
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_mac_screentime (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    timestamp TIMESTAMPTZ NOT NULL,

    -- Period information
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    period_type TEXT DEFAULT 'hourly',

    -- Usage metrics
    total_screen_time INTEGER,
    productive_time INTEGER,
    entertainment_time INTEGER,
    communication_time INTEGER,
    unclassified_time INTEGER,

    -- App breakdown
    top_apps JSONB,

    -- Raw data backup
    raw_data JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, period_start, period_end)
);

CREATE INDEX idx_mac_screentime_timestamp ON stream_mac_screentime(timestamp DESC);
CREATE INDEX idx_mac_screentime_source_time ON stream_mac_screentime(source_id, timestamp DESC);
CREATE INDEX idx_mac_screentime_period ON stream_mac_screentime(period_start, period_end);

COMMENT ON TABLE stream_mac_screentime IS 'macOS screen time summaries aggregated by time period';

-- ============================================================================
-- NOTION PAGES
-- ============================================================================

CREATE TABLE IF NOT EXISTS stream_notion_pages (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Notion page identifiers
    page_id TEXT NOT NULL,
    url TEXT NOT NULL,

    -- Timing
    created_time TIMESTAMPTZ NOT NULL,
    last_edited_time TIMESTAMPTZ NOT NULL,

    -- People
    created_by_id TEXT NOT NULL,
    created_by_name TEXT,
    last_edited_by_id TEXT NOT NULL,
    last_edited_by_name TEXT,

    -- Parent information
    parent_type TEXT NOT NULL,
    parent_id TEXT,

    -- Status
    archived BOOLEAN DEFAULT false,

    -- Properties and metadata
    properties JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Full data backup
    raw_json JSONB,

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(source_id, page_id)
);

CREATE INDEX idx_notion_pages_source ON stream_notion_pages(source_id);
CREATE INDEX idx_notion_pages_page_id ON stream_notion_pages(page_id);
CREATE INDEX idx_notion_pages_last_edited ON stream_notion_pages(last_edited_time DESC);
CREATE INDEX idx_notion_pages_created ON stream_notion_pages(created_time DESC);
CREATE INDEX idx_notion_pages_archived ON stream_notion_pages(archived) WHERE archived = false;
CREATE INDEX idx_notion_pages_parent ON stream_notion_pages(parent_type, parent_id);
CREATE INDEX idx_notion_pages_sync_time ON stream_notion_pages(source_id, synced_at DESC);
CREATE INDEX idx_notion_pages_properties_search ON stream_notion_pages USING GIN (properties jsonb_path_ops);

CREATE TRIGGER stream_notion_pages_updated_at
    BEFORE UPDATE ON stream_notion_pages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE stream_notion_pages IS 'Notion pages with metadata, properties, and relationships';
