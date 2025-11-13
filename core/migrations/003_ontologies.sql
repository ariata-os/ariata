-- Ontology Layer: Domain and Entity Primitives
-- Normalized facts (domain primitives) + Canonical identities (entity primitives)
-- Single source of truth for all life logging data
--
-- MVP SCHEMA: Only includes tables with active data sources/transforms

SET search_path TO elt, public;

-- ============================================================================
-- ENTITY PRIMITIVES
-- Canonical identities that persist across all sources
-- ============================================================================

-- entities_person: People you directly interact with
CREATE TABLE IF NOT EXISTS entities_person (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    canonical_name TEXT NOT NULL,
    email_addresses TEXT[] DEFAULT '{}',
    phone_numbers TEXT[] DEFAULT '{}',
    display_names TEXT[] DEFAULT '{}',

    relationship_category TEXT,

    first_interaction TIMESTAMPTZ,
    last_interaction TIMESTAMPTZ,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_person_name ON entities_person(canonical_name);
CREATE INDEX idx_entities_person_emails ON entities_person USING GIN(email_addresses);
CREATE INDEX idx_entities_person_phones ON entities_person USING GIN(phone_numbers);

CREATE TRIGGER entities_person_updated_at
    BEFORE UPDATE ON entities_person
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE entities_person IS 'Canonical identities of people you directly interact with';

-- entities_place: Canonical locations you physically visit
CREATE TABLE IF NOT EXISTS entities_place (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    canonical_name TEXT NOT NULL,
    category TEXT,

    geo_center GEOGRAPHY(POINT),
    bounding_box GEOGRAPHY(POLYGON),
    cluster_radius_meters FLOAT,

    visit_count INTEGER DEFAULT 0,
    total_time_minutes INTEGER DEFAULT 0,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_place_name ON entities_place(canonical_name);
CREATE INDEX idx_entities_place_geo ON entities_place USING GIST(geo_center);

CREATE TRIGGER entities_place_updated_at
    BEFORE UPDATE ON entities_place
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE entities_place IS 'Canonical locations you physically visit';

-- entities_topic: Projects, interests, and themes
CREATE TABLE IF NOT EXISTS entities_topic (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    name TEXT NOT NULL,
    category TEXT,
    keywords TEXT[] DEFAULT '{}',

    first_mentioned TIMESTAMPTZ,
    last_mentioned TIMESTAMPTZ,
    mention_count INTEGER DEFAULT 0,

    sources JSONB DEFAULT '{}',
    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_topic_name ON entities_topic(name);

CREATE TRIGGER entities_topic_updated_at
    BEFORE UPDATE ON entities_topic
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE entities_topic IS 'Projects, interests, and themes in your life';

-- ============================================================================
-- HEALTH DOMAIN PRIMITIVES
-- Physiological measurements and bodily states
-- ============================================================================

-- health_heart_rate (signal)
CREATE TABLE IF NOT EXISTS health_heart_rate (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    bpm INTEGER NOT NULL,
    measurement_context TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_heart_rate_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_health_heart_rate_timestamp ON health_heart_rate(timestamp DESC);

-- health_hrv (signal)
CREATE TABLE IF NOT EXISTS health_hrv (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    hrv_ms FLOAT NOT NULL,
    measurement_type TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_hrv_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_health_hrv_timestamp ON health_hrv(timestamp DESC);

-- health_steps (signal)
CREATE TABLE IF NOT EXISTS health_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    step_count INTEGER NOT NULL,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_steps_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_health_steps_timestamp ON health_steps(timestamp DESC);

-- health_sleep (temporal)
CREATE TABLE IF NOT EXISTS health_sleep (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    sleep_stages JSONB,
    total_duration_minutes INTEGER NOT NULL,
    sleep_quality_score FLOAT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_sleep_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_health_sleep_start_time ON health_sleep(start_time DESC);

-- health_workout (temporal)
CREATE TABLE IF NOT EXISTS health_workout (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    activity_type TEXT NOT NULL,
    intensity TEXT,

    calories_burned INTEGER,
    average_heart_rate INTEGER,
    max_heart_rate INTEGER,
    distance_meters FLOAT,

    place_id UUID REFERENCES entities_place(id),

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_workout_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_health_workout_start_time ON health_workout(start_time DESC);
CREATE INDEX idx_health_workout_place ON health_workout(place_id);

-- ============================================================================
-- LOCATION DOMAIN PRIMITIVES
-- Geographic position and movement data
-- ============================================================================

-- location_point (signal)
CREATE TABLE IF NOT EXISTS location_point (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    coordinates GEOGRAPHY(POINT) NOT NULL,
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,
    altitude_meters FLOAT,

    accuracy_meters FLOAT,
    speed_meters_per_second FLOAT,
    course_degrees FLOAT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT location_point_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_location_point_coords ON location_point USING GIST(coordinates);
CREATE INDEX idx_location_point_timestamp ON location_point(timestamp DESC);

-- ============================================================================
-- SOCIAL DOMAIN PRIMITIVES
-- Communication and interpersonal interactions
-- ============================================================================

-- social_email (temporal)
-- Email communications from various providers (Gmail, Outlook, etc.)
CREATE TABLE IF NOT EXISTS social_email (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Email identifiers
    message_id TEXT NOT NULL,
    thread_id TEXT,
    subject TEXT,
    snippet TEXT,

    -- Email content
    body_plain TEXT,
    body_html TEXT,

    -- Timestamp
    timestamp TIMESTAMPTZ NOT NULL,

    -- Sender information
    from_address TEXT,
    from_name TEXT,

    -- Recipient information
    to_addresses TEXT[] DEFAULT '{}',
    to_names TEXT[] DEFAULT '{}',
    cc_addresses TEXT[] DEFAULT '{}',
    cc_names TEXT[] DEFAULT '{}',
    bcc_addresses TEXT[] DEFAULT '{}',

    -- Entity references (for person resolution)
    from_person_id UUID REFERENCES entities_person(id),
    to_person_ids UUID[] DEFAULT '{}',
    cc_person_ids UUID[] DEFAULT '{}',
    bcc_person_ids UUID[] DEFAULT '{}',

    -- Email metadata
    direction TEXT NOT NULL,
    labels TEXT[] DEFAULT '{}',
    is_read BOOLEAN DEFAULT false,
    is_starred BOOLEAN DEFAULT false,

    -- Attachment information
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,

    -- Thread information
    thread_position INTEGER,
    thread_message_count INTEGER,

    -- Source tracking
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_email_direction_check CHECK (direction IN ('sent', 'received')),
    CONSTRAINT social_email_unique_source UNIQUE (source_table, message_id)
);

CREATE INDEX idx_social_email_from_person ON social_email(from_person_id);
CREATE INDEX idx_social_email_timestamp ON social_email(timestamp DESC);

CREATE TRIGGER social_email_updated_at
    BEFORE UPDATE ON social_email
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- social_message (temporal)
-- Text messages from SMS, iMessage, WhatsApp, Slack, etc.
CREATE TABLE IF NOT EXISTS social_message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Message identifiers
    message_id TEXT NOT NULL,
    thread_id TEXT,
    channel TEXT NOT NULL,

    -- Message content
    body TEXT,

    -- Timestamp
    timestamp TIMESTAMPTZ NOT NULL,

    -- Sender information
    from_identifier TEXT,
    from_name TEXT,

    -- Recipient information
    to_identifiers TEXT[] DEFAULT '{}',
    to_names TEXT[] DEFAULT '{}',

    -- Entity references (for person resolution)
    from_person_id UUID REFERENCES entities_person(id),
    to_person_ids UUID[] DEFAULT '{}',

    -- Message metadata
    direction TEXT NOT NULL CHECK (direction IN ('sent', 'received')),
    is_read BOOLEAN DEFAULT false,
    is_group_message BOOLEAN DEFAULT false,
    group_name TEXT,

    -- Attachment information
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[] DEFAULT '{}',

    -- Source tracking
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    -- Metadata
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_message_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_social_message_from_person ON social_message(from_person_id);
CREATE INDEX idx_social_message_timestamp ON social_message(timestamp DESC);
CREATE INDEX idx_social_message_channel ON social_message(channel);
CREATE INDEX idx_social_message_thread ON social_message(thread_id);

CREATE TRIGGER social_message_updated_at
    BEFORE UPDATE ON social_message
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- social_call (temporal)
-- Voice and video calls from phone, FaceTime, Zoom, Teams, etc.
CREATE TABLE IF NOT EXISTS social_call (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Call identifiers
    call_id TEXT,

    -- Call type and direction
    call_type TEXT NOT NULL CHECK (call_type IN ('voice', 'video', 'voip')),
    direction TEXT NOT NULL CHECK (direction IN ('incoming', 'outgoing')),
    call_status TEXT NOT NULL CHECK (call_status IN ('answered', 'missed', 'declined', 'voicemail', 'cancelled')),

    -- Temporal (bounded duration)
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    duration_seconds INTEGER,

    -- Caller information
    caller_identifier TEXT,
    caller_name TEXT,

    -- Callee information
    callee_identifiers TEXT[] DEFAULT '{}',
    callee_names TEXT[] DEFAULT '{}',

    -- Entity references (for person resolution)
    caller_person_id UUID REFERENCES entities_person(id),
    callee_person_ids UUID[] DEFAULT '{}',

    -- Call metadata
    is_group_call BOOLEAN DEFAULT false,
    platform TEXT,

    -- Source tracking
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    -- Metadata
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_call_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_social_call_caller_person ON social_call(caller_person_id);
CREATE INDEX idx_social_call_start_time ON social_call(start_time DESC);
CREATE INDEX idx_social_call_call_type ON social_call(call_type);
CREATE INDEX idx_social_call_platform ON social_call(platform);

CREATE TRIGGER social_call_updated_at
    BEFORE UPDATE ON social_call
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- ACTIVITY DOMAIN PRIMITIVES
-- Time allocation and attention tracking
-- ============================================================================

-- activity_calendar_entry (temporal)
CREATE TABLE IF NOT EXISTS activity_calendar_entry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT,
    description TEXT,
    calendar_name TEXT,
    event_type TEXT,

    organizer_identifier TEXT,
    attendee_identifiers TEXT[] DEFAULT '{}',

    organizer_person_id UUID REFERENCES entities_person(id),
    attendee_person_ids UUID[] DEFAULT '{}',

    topic_id UUID REFERENCES entities_topic(id),
    topic_keywords TEXT[] DEFAULT '{}',

    location_name TEXT,
    place_id UUID REFERENCES entities_place(id),

    conference_url TEXT,
    conference_platform TEXT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    is_all_day BOOLEAN DEFAULT false,

    status TEXT,
    response_status TEXT,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT activity_calendar_entry_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_activity_calendar_topic ON activity_calendar_entry(topic_id);
CREATE INDEX idx_activity_calendar_place ON activity_calendar_entry(place_id);
CREATE INDEX idx_activity_calendar_start_time ON activity_calendar_entry(start_time DESC);

CREATE TRIGGER activity_calendar_entry_updated_at
    BEFORE UPDATE ON activity_calendar_entry
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- activity_app_usage (temporal)
-- Application usage tracking from Mac, Windows, etc.
CREATE TABLE IF NOT EXISTS activity_app_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- App identifiers
    app_name TEXT NOT NULL,
    app_bundle_id TEXT,
    app_category TEXT,

    -- Temporal (bounded duration)
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    -- Context details
    window_title TEXT,
    document_path TEXT,
    url TEXT,

    -- Entity references
    topic_id UUID REFERENCES entities_topic(id),
    topic_keywords TEXT[] DEFAULT '{}',

    -- Source tracking
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    -- Metadata
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT activity_app_usage_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_activity_app_usage_app_name ON activity_app_usage(app_name);
CREATE INDEX idx_activity_app_usage_app_category ON activity_app_usage(app_category);
CREATE INDEX idx_activity_app_usage_start_time ON activity_app_usage(start_time DESC);
CREATE INDEX idx_activity_app_usage_topic ON activity_app_usage(topic_id);

CREATE TRIGGER activity_app_usage_updated_at
    BEFORE UPDATE ON activity_app_usage
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- AMBIENT DOMAIN PRIMITIVES
-- External environmental conditions
-- ============================================================================

-- ambient_audio_captioning (temporal)
-- Free-form semantic descriptions of audio content
-- Examples: "Two people conversing in a restaurant with background music and dishes"
--           "Wind blowing through trees with distant traffic noise"
--           "Keyboard typing in a quiet office with air conditioning hum"
CREATE TABLE IF NOT EXISTS ambient_audio_captioning (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Free-form audio description (natural language)
    caption TEXT NOT NULL,

    -- Optional structured fields extracted from caption
    primary_sounds TEXT[],        -- ["conversation", "dishes", "music"]
    acoustic_environment TEXT,     -- "indoor_restaurant", "outdoor_park", "vehicle_interior"
    estimated_participants INT,    -- Number of speakers/people detected
    ambient_noise_level TEXT,      -- "quiet", "moderate", "loud"

    -- Confidence and timing
    confidence FLOAT,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    -- Place reference
    place_id UUID REFERENCES entities_place(id),

    -- Source tracking
    audio_file_path TEXT,          -- Path to source audio file
    audio_duration_seconds INT,    -- Duration of captioned audio
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    -- Metadata
    metadata JSONB DEFAULT '{}',   -- Model info, processing params, etc.

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint for transform idempotency
    CONSTRAINT ambient_audio_captioning_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_ambient_audio_captioning_source ON ambient_audio_captioning(source_stream_id);
CREATE INDEX idx_ambient_audio_captioning_start_time ON ambient_audio_captioning(start_time DESC);
CREATE INDEX idx_ambient_audio_captioning_environment ON ambient_audio_captioning(acoustic_environment) WHERE acoustic_environment IS NOT NULL;
CREATE INDEX idx_ambient_audio_captioning_place ON ambient_audio_captioning(place_id);
CREATE INDEX idx_ambient_audio_captioning_caption_search ON ambient_audio_captioning USING GIN (to_tsvector('english', caption));
CREATE TRIGGER ambient_audio_captioning_updated_at BEFORE UPDATE ON ambient_audio_captioning FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- KNOWLEDGE DOMAIN PRIMITIVES
-- Semantic artifacts and documents
-- ============================================================================

-- knowledge_document (temporal)
CREATE TABLE IF NOT EXISTS knowledge_document (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT,
    content TEXT,
    content_summary TEXT,
    document_type TEXT,

    external_id TEXT,
    external_url TEXT,

    topic_id UUID REFERENCES entities_topic(id),
    tags TEXT[] DEFAULT '{}',

    is_authored BOOLEAN DEFAULT false,

    created_time TIMESTAMPTZ,
    last_modified_time TIMESTAMPTZ,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT knowledge_document_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_knowledge_document_title ON knowledge_document(title);
CREATE INDEX idx_knowledge_document_topic ON knowledge_document(topic_id);
CREATE INDEX idx_knowledge_document_tags ON knowledge_document USING GIN(tags);
CREATE INDEX idx_knowledge_document_source ON knowledge_document(source_stream_id);
CREATE INDEX idx_knowledge_document_search ON knowledge_document USING GIN(to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, '')));
CREATE TRIGGER knowledge_document_updated_at BEFORE UPDATE ON knowledge_document FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- knowledge_ai_conversation (temporal)
CREATE TABLE IF NOT EXISTS knowledge_ai_conversation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Conversation/Message identification
    conversation_id TEXT NOT NULL,
    message_id TEXT NOT NULL,

    -- Message content
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,

    -- Model information
    model TEXT,
    provider TEXT NOT NULL,

    -- Entity relationships (for future linking)
    topic_id UUID REFERENCES entities_topic(id),
    tags TEXT[] DEFAULT '{}',

    -- Timing
    timestamp TIMESTAMPTZ NOT NULL,

    -- Source tracking (standard ontology pattern)
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL DEFAULT 'stream_ariata_ai_chat',
    source_provider TEXT NOT NULL DEFAULT 'ariata',

    -- Additional metadata
    metadata JSONB DEFAULT '{}',

    -- Standard audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT knowledge_ai_conversation_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_knowledge_ai_conversation_conversation
    ON knowledge_ai_conversation(conversation_id, timestamp);
CREATE INDEX idx_knowledge_ai_conversation_timestamp
    ON knowledge_ai_conversation(timestamp DESC);
CREATE INDEX idx_knowledge_ai_conversation_topic
    ON knowledge_ai_conversation(topic_id) WHERE topic_id IS NOT NULL;
CREATE INDEX idx_knowledge_ai_conversation_provider
    ON knowledge_ai_conversation(provider, timestamp DESC);
CREATE TRIGGER knowledge_ai_conversation_updated_at BEFORE UPDATE ON knowledge_ai_conversation FOR EACH ROW EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE knowledge_ai_conversation IS
'Normalized AI conversations in the knowledge domain. Transformed from stream_ariata_ai_chat.';
COMMENT ON COLUMN knowledge_ai_conversation.source_stream_id IS
'UUID linking back to the stream table record (stream_ariata_ai_chat.id).';

-- ============================================================================
-- SPEECH DOMAIN PRIMITIVES
-- Transcribed spoken audio (intermediate primitive)
-- ============================================================================

-- speech_transcription (temporal)
CREATE TABLE IF NOT EXISTS speech_transcription (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    audio_file_path TEXT,
    audio_duration_seconds INTEGER,

    transcript_text TEXT NOT NULL,
    language TEXT,
    confidence_score FLOAT,

    speaker_count INTEGER,
    speaker_labels JSONB,

    recorded_at TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint for transform idempotency
    CONSTRAINT speech_transcription_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_speech_transcription_source ON speech_transcription(source_stream_id);
CREATE INDEX idx_speech_transcription_recorded_at ON speech_transcription(recorded_at DESC);
CREATE INDEX idx_speech_transcription_search ON speech_transcription USING GIN(to_tsvector('english', transcript_text));
CREATE TRIGGER speech_transcription_updated_at BEFORE UPDATE ON speech_transcription FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON SCHEMA elt IS 'Ariata ELT system: streams (raw) + ontologies (normalized)';
