-- Google Calendar stream table
-- Stores calendar events with full fidelity

CREATE TABLE IF NOT EXISTS stream_google_calendar (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Google event identifiers
    event_id TEXT NOT NULL,  -- Google's event ID
    calendar_id TEXT NOT NULL,  -- Which calendar this came from
    etag TEXT,  -- For change detection

    -- Core event data
    summary TEXT,  -- Event title
    description TEXT,  -- Event description
    location TEXT,  -- Physical or virtual location
    status TEXT,  -- confirmed, tentative, cancelled

    -- Timing
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    all_day BOOLEAN DEFAULT false,
    timezone TEXT,  -- Event timezone if different from start/end

    -- People
    organizer_email TEXT,
    organizer_name TEXT,
    creator_email TEXT,
    creator_name TEXT,
    attendee_count INTEGER DEFAULT 0,

    -- Meeting details
    has_conferencing BOOLEAN DEFAULT false,
    conference_type TEXT,  -- meet, zoom, teams, etc
    conference_link TEXT,

    -- Metadata
    created_by_google TIMESTAMPTZ,  -- When Google created it
    updated_by_google TIMESTAMPTZ,  -- Last Google update
    is_recurring BOOLEAN DEFAULT false,
    recurring_event_id TEXT,  -- Parent event for recurring

    -- Full data backup
    raw_json JSONB,  -- Complete event from Google API

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Prevent duplicate events
    UNIQUE(source_id, event_id)
);

-- Indexes for common queries
CREATE INDEX idx_gcal_source ON stream_google_calendar(source_id);
CREATE INDEX idx_gcal_event ON stream_google_calendar(event_id);
CREATE INDEX idx_gcal_start ON stream_google_calendar(start_time);
CREATE INDEX idx_gcal_end ON stream_google_calendar(end_time);
CREATE INDEX idx_gcal_calendar ON stream_google_calendar(calendar_id);
CREATE INDEX idx_gcal_organizer ON stream_google_calendar(organizer_email);

-- Index for finding events in a time range
CREATE INDEX idx_gcal_time_range ON stream_google_calendar(source_id, start_time, end_time);

-- Trigger for updated_at
CREATE TRIGGER stream_google_calendar_updated_at
    BEFORE UPDATE ON stream_google_calendar
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE stream_google_calendar IS 'Google Calendar events with full fidelity';
COMMENT ON COLUMN stream_google_calendar.event_id IS 'Google event ID - unique per calendar';
COMMENT ON COLUMN stream_google_calendar.raw_json IS 'Complete event object from Google API for data recovery';