SET search_path TO data, public;

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

CREATE TABLE IF NOT EXISTS location_visit (
    id UUID PRIMARY KEY,

    place_id UUID REFERENCES entities_place(id),

    centroid_coordinates GEOGRAPHY(POINT) NOT NULL,
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL DEFAULT 'location_point',
    source_provider TEXT NOT NULL DEFAULT 'ios',

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_location_visit_centroid ON location_visit USING GIST(centroid_coordinates);
CREATE INDEX idx_location_visit_start_time ON location_visit(start_time DESC);
CREATE INDEX idx_location_visit_end_time ON location_visit(end_time DESC);
CREATE INDEX idx_location_visit_place ON location_visit(place_id) WHERE place_id IS NOT NULL;
CREATE INDEX idx_location_visit_source ON location_visit(source_stream_id);

CREATE TRIGGER location_visit_updated_at
    BEFORE UPDATE ON location_visit
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS social_email (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    message_id TEXT NOT NULL,
    thread_id TEXT,
    subject TEXT,
    snippet TEXT,

    body_plain TEXT,
    body_html TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    from_address TEXT,
    from_name TEXT,

    to_addresses TEXT[] DEFAULT '{}',
    to_names TEXT[] DEFAULT '{}',
    cc_addresses TEXT[] DEFAULT '{}',
    cc_names TEXT[] DEFAULT '{}',
    bcc_addresses TEXT[] DEFAULT '{}',

    from_person_id UUID REFERENCES entities_person(id),
    to_person_ids UUID[] DEFAULT '{}',
    cc_person_ids UUID[] DEFAULT '{}',
    bcc_person_ids UUID[] DEFAULT '{}',

    direction TEXT NOT NULL,
    labels TEXT[] DEFAULT '{}',
    is_read BOOLEAN DEFAULT false,
    is_starred BOOLEAN DEFAULT false,

    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,

    thread_position INTEGER,
    thread_message_count INTEGER,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

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

CREATE TABLE IF NOT EXISTS social_message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    message_id TEXT NOT NULL,
    thread_id TEXT,
    channel TEXT NOT NULL,

    body TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    from_identifier TEXT,
    from_name TEXT,

    to_identifiers TEXT[] DEFAULT '{}',
    to_names TEXT[] DEFAULT '{}',

    from_person_id UUID REFERENCES entities_person(id),
    to_person_ids UUID[] DEFAULT '{}',

    direction TEXT NOT NULL CHECK (direction IN ('sent', 'received')),
    is_read BOOLEAN DEFAULT false,
    is_group_message BOOLEAN DEFAULT false,
    group_name TEXT,

    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    attachment_types TEXT[] DEFAULT '{}',

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

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

CREATE TABLE IF NOT EXISTS activity_app_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    app_name TEXT NOT NULL,
    app_bundle_id TEXT,
    app_category TEXT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    window_title TEXT,
    document_path TEXT,
    url TEXT,

    topic_id UUID REFERENCES entities_topic(id),
    topic_keywords TEXT[] DEFAULT '{}',

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

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

CREATE TABLE IF NOT EXISTS activity_web_browsing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    page_title TEXT,

    visit_duration_seconds INTEGER,
    scroll_depth_percent FLOAT,

    timestamp TIMESTAMPTZ NOT NULL,

    topic_id UUID REFERENCES entities_topic(id),
    topic_keywords TEXT[] DEFAULT '{}',

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT activity_web_browsing_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_activity_web_browsing_domain ON activity_web_browsing(domain);
CREATE INDEX idx_activity_web_browsing_timestamp ON activity_web_browsing(timestamp DESC);
CREATE INDEX idx_activity_web_browsing_topic ON activity_web_browsing(topic_id);
CREATE INDEX idx_activity_web_browsing_url ON activity_web_browsing USING hash(url);

CREATE TRIGGER activity_web_browsing_updated_at
    BEFORE UPDATE ON activity_web_browsing
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

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

CREATE TRIGGER knowledge_document_updated_at
    BEFORE UPDATE ON knowledge_document
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS knowledge_ai_conversation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    conversation_id TEXT NOT NULL,
    message_id TEXT NOT NULL,

    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,

    model TEXT,
    provider TEXT NOT NULL,

    topic_id UUID REFERENCES entities_topic(id),
    tags TEXT[] DEFAULT '{}',

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL DEFAULT 'stream_ariata_ai_chat',
    source_provider TEXT NOT NULL DEFAULT 'ariata',

    metadata JSONB DEFAULT '{}',

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

CREATE TRIGGER knowledge_ai_conversation_updated_at
    BEFORE UPDATE ON knowledge_ai_conversation
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

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

    CONSTRAINT speech_transcription_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_speech_transcription_source ON speech_transcription(source_stream_id);
CREATE INDEX idx_speech_transcription_recorded_at ON speech_transcription(recorded_at DESC);
CREATE INDEX idx_speech_transcription_search ON speech_transcription USING GIN(to_tsvector('english', transcript_text));

CREATE TRIGGER speech_transcription_updated_at
    BEFORE UPDATE ON speech_transcription
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
