-- OAuth and scheduler tables for managing cloud source syncs

-- OAuth credentials storage
CREATE TABLE IF NOT EXISTS oauth_credentials (
    id SERIAL PRIMARY KEY,
    provider TEXT NOT NULL UNIQUE,
    credentials JSONB NOT NULL,  -- Encrypted OAuth tokens
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_provider ON oauth_credentials(provider);

-- Sync schedules for sources
CREATE TABLE IF NOT EXISTS sync_schedules (
    id SERIAL PRIMARY KEY,
    source_name TEXT NOT NULL UNIQUE,
    cron_expression TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    last_run TIMESTAMPTZ,
    next_run TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_schedules_source ON sync_schedules(source_name);
CREATE INDEX idx_sync_schedules_enabled ON sync_schedules(enabled);

-- Sync history for tracking
CREATE TABLE IF NOT EXISTS sync_history (
    id BIGSERIAL PRIMARY KEY,
    source TEXT NOT NULL,
    records_synced INTEGER NOT NULL DEFAULT 0,
    duration_ms BIGINT NOT NULL,
    success BOOLEAN NOT NULL DEFAULT true,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_history_source ON sync_history(source);
CREATE INDEX idx_sync_history_created ON sync_history(created_at DESC);

-- Sync state for incremental syncs
CREATE TABLE IF NOT EXISTS sync_state (
    id SERIAL PRIMARY KEY,
    source TEXT NOT NULL UNIQUE,
    last_sync TIMESTAMPTZ,
    sync_token TEXT,
    cursor TEXT,
    checkpoint JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_state_source ON sync_state(source);

-- Google Calendar specific
CREATE TABLE IF NOT EXISTS stream_google_calendar (
    id BIGSERIAL PRIMARY KEY,
    event_id TEXT NOT NULL,
    calendar_id TEXT NOT NULL,
    summary TEXT,
    description TEXT,
    location TEXT,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    all_day BOOLEAN DEFAULT FALSE,
    status TEXT,
    organizer_email TEXT,
    attendees JSONB,
    recurrence TEXT[],
    html_link TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(event_id, calendar_id)
);

CREATE INDEX idx_google_calendar_event ON stream_google_calendar(event_id);
CREATE INDEX idx_google_calendar_time ON stream_google_calendar(start_time, end_time);
CREATE INDEX idx_google_calendar_calendar ON stream_google_calendar(calendar_id);

-- Strava activities
CREATE TABLE IF NOT EXISTS stream_strava_activities (
    id BIGSERIAL PRIMARY KEY,
    activity_id BIGINT NOT NULL UNIQUE,
    athlete_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    sport_type TEXT,
    start_date TIMESTAMPTZ NOT NULL,
    distance DOUBLE PRECISION,
    moving_time INTEGER,
    elapsed_time INTEGER,
    total_elevation_gain DOUBLE PRECISION,
    average_speed DOUBLE PRECISION,
    max_speed DOUBLE PRECISION,
    average_heartrate DOUBLE PRECISION,
    max_heartrate DOUBLE PRECISION,
    start_latlng DOUBLE PRECISION[],
    end_latlng DOUBLE PRECISION[],
    map_polyline TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_strava_activity ON stream_strava_activities(activity_id);
CREATE INDEX idx_strava_athlete ON stream_strava_activities(athlete_id);
CREATE INDEX idx_strava_date ON stream_strava_activities(start_date DESC);

-- Notion pages
CREATE TABLE IF NOT EXISTS stream_notion_pages (
    id BIGSERIAL PRIMARY KEY,
    page_id TEXT NOT NULL UNIQUE,
    parent_type TEXT,
    parent_id TEXT,
    title TEXT,
    url TEXT,
    icon JSONB,
    cover JSONB,
    properties JSONB,
    content TEXT,
    last_edited_time TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notion_page ON stream_notion_pages(page_id);
CREATE INDEX idx_notion_parent ON stream_notion_pages(parent_id);
CREATE INDEX idx_notion_edited ON stream_notion_pages(last_edited_time DESC);