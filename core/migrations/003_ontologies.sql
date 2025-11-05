-- Ontology Layer: Domain and Entity Primitives
-- Normalized facts (domain primitives) + Canonical identities (entity primitives)
-- Single source of truth for all life logging data

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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT entities_person_relationship_check
        CHECK (relationship_category IS NULL OR relationship_category IN (
            'colleague', 'friend', 'family', 'service_provider', 'acquaintance', 'other'
        ))
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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT entities_place_category_check
        CHECK (category IS NULL OR category IN (
            'home', 'work', 'gym', 'restaurant', 'park', 'shop', 'transit', 'other'
        ))
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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT entities_topic_category_check
        CHECK (category IS NULL OR category IN (
            'project', 'skill', 'interest', 'goal', 'other'
        ))
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

    CONSTRAINT health_heart_rate_context_check
        CHECK (measurement_context IS NULL OR measurement_context IN (
            'resting', 'active', 'workout', 'sleep', 'recovery'
        ))
);


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

    CONSTRAINT health_hrv_type_check
        CHECK (measurement_type IS NULL OR measurement_type IN ('rmssd', 'sdnn', 'pnn50'))
);


-- health_blood_oxygen (signal)
CREATE TABLE IF NOT EXISTS health_blood_oxygen (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    spo2_percent FLOAT NOT NULL,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- health_blood_pressure (signal)
CREATE TABLE IF NOT EXISTS health_blood_pressure (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    systolic_mmhg INTEGER NOT NULL,
    diastolic_mmhg INTEGER NOT NULL,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- health_blood_glucose (signal)
CREATE TABLE IF NOT EXISTS health_blood_glucose (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    glucose_mg_dl FLOAT NOT NULL,
    meal_context TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_blood_glucose_context_check
        CHECK (meal_context IS NULL OR meal_context IN ('fasting', 'pre_meal', 'post_meal', 'random'))
);


-- health_body_temperature (signal)
CREATE TABLE IF NOT EXISTS health_body_temperature (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    temperature_celsius FLOAT NOT NULL,
    measurement_location TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_body_temperature_location_check
        CHECK (measurement_location IS NULL OR measurement_location IN ('oral', 'forehead', 'wrist', 'ear', 'axillary'))
);


-- health_respiratory_rate (signal)
CREATE TABLE IF NOT EXISTS health_respiratory_rate (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    breaths_per_minute INTEGER NOT NULL,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


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

    CONSTRAINT health_workout_intensity_check
        CHECK (intensity IS NULL OR intensity IN ('low', 'moderate', 'high', 'max'))
);


-- health_meal (temporal)
CREATE TABLE IF NOT EXISTS health_meal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    meal_type TEXT,
    foods JSONB DEFAULT '[]',

    total_calories INTEGER,
    protein_grams FLOAT,
    carbs_grams FLOAT,
    fat_grams FLOAT,

    place_id UUID REFERENCES entities_place(id),

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_meal_type_check
        CHECK (meal_type IS NULL OR meal_type IN ('breakfast', 'lunch', 'dinner', 'snack'))
);


-- health_medication (temporal)
CREATE TABLE IF NOT EXISTS health_medication (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    medication_name TEXT NOT NULL,
    dosage TEXT NOT NULL,
    route TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- health_symptom (temporal)
CREATE TABLE IF NOT EXISTS health_symptom (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    symptom_name TEXT NOT NULL,
    severity TEXT,
    body_location TEXT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_symptom_severity_check
        CHECK (severity IS NULL OR severity IN ('mild', 'moderate', 'severe'))
);


-- health_mood (temporal)
CREATE TABLE IF NOT EXISTS health_mood (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    valence FLOAT NOT NULL,
    arousal FLOAT NOT NULL,
    mood_category TEXT,
    measurement_method TEXT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT health_mood_valence_check CHECK (valence BETWEEN -1.0 AND 1.0),
    CONSTRAINT health_mood_arousal_check CHECK (arousal BETWEEN -1.0 AND 1.0),
    CONSTRAINT health_mood_category_check
        CHECK (mood_category IS NULL OR mood_category IN (
            'happy', 'sad', 'anxious', 'calm', 'stressed', 'energized', 'tired', 'neutral'
        )),
    CONSTRAINT health_mood_method_check
        CHECK (measurement_method IS NULL OR measurement_method IN (
            'self_reported', 'hrv_derived', 'activity_inferred'
        ))
);


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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_location_point_coords ON location_point USING GIST(coordinates);

-- location_visit (temporal)
CREATE TABLE IF NOT EXISTS location_visit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    place_id UUID REFERENCES entities_place(id),

    centroid_coordinates GEOGRAPHY(POINT) NOT NULL,
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_location_visit_place ON location_visit(place_id);

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

-- social_message (temporal)
CREATE TABLE IF NOT EXISTS social_message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    message_id TEXT NOT NULL,
    thread_id TEXT,
    channel TEXT NOT NULL,

    body TEXT,

    from_identifier TEXT,
    to_identifiers TEXT[] DEFAULT '{}',

    from_person_id UUID REFERENCES entities_person(id),
    to_person_ids UUID[] DEFAULT '{}',

    direction TEXT NOT NULL,
    is_read BOOLEAN DEFAULT false,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_message_direction_check CHECK (direction IN ('sent', 'received')),
    CONSTRAINT social_message_channel_check
        CHECK (channel IN ('sms', 'imessage', 'slack', 'whatsapp', 'discord', 'telegram'))
);

CREATE INDEX idx_social_message_from_person ON social_message(from_person_id);

-- social_call (temporal)
CREATE TABLE IF NOT EXISTS social_call (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    call_type TEXT NOT NULL,
    direction TEXT NOT NULL,
    call_status TEXT NOT NULL,

    caller_identifier TEXT,
    callee_identifiers TEXT[] DEFAULT '{}',

    caller_person_id UUID REFERENCES entities_person(id),
    callee_person_ids UUID[] DEFAULT '{}',

    duration_seconds INTEGER,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_call_type_check CHECK (call_type IN ('voice', 'video')),
    CONSTRAINT social_call_direction_check CHECK (direction IN ('incoming', 'outgoing')),
    CONSTRAINT social_call_status_check CHECK (call_status IN ('answered', 'missed', 'declined', 'voicemail'))
);

CREATE INDEX idx_social_call_caller_person ON social_call(caller_person_id);

-- social_interaction (temporal)
CREATE TABLE IF NOT EXISTS social_interaction (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    interaction_type TEXT NOT NULL,
    title TEXT,
    description TEXT,

    participant_identifiers TEXT[] DEFAULT '{}',
    participant_person_ids UUID[] DEFAULT '{}',

    place_id UUID REFERENCES entities_place(id),
    location_name TEXT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_interaction_type_check
        CHECK (interaction_type IN ('meeting', 'gathering', 'event', 'casual_encounter'))
);

CREATE INDEX idx_social_interaction_place ON social_interaction(place_id);

-- social_post (temporal)
CREATE TABLE IF NOT EXISTS social_post (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    platform TEXT NOT NULL,
    post_id TEXT NOT NULL,
    post_type TEXT NOT NULL,

    content TEXT,
    media_urls TEXT[] DEFAULT '{}',

    like_count INTEGER DEFAULT 0,
    repost_count INTEGER DEFAULT 0,
    comment_count INTEGER DEFAULT 0,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT social_post_platform_check
        CHECK (platform IN ('twitter', 'instagram', 'facebook', 'linkedin', 'mastodon')),
    CONSTRAINT social_post_type_check
        CHECK (post_type IN ('original', 'repost', 'reply', 'quote'))
);


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

    CONSTRAINT activity_calendar_event_type_check
        CHECK (event_type IS NULL OR event_type IN ('meeting', 'appointment', 'reminder', 'focus_block')),
    CONSTRAINT activity_calendar_status_check
        CHECK (status IS NULL OR status IN ('confirmed', 'tentative', 'cancelled')),
    CONSTRAINT activity_calendar_entry_unique_source UNIQUE (source_stream_id)
);

CREATE INDEX idx_activity_calendar_topic ON activity_calendar_entry(topic_id);
CREATE INDEX idx_activity_calendar_place ON activity_calendar_entry(place_id);

-- activity_app_usage (temporal)
CREATE TABLE IF NOT EXISTS activity_app_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    app_name TEXT NOT NULL,
    app_bundle_id TEXT,
    app_category TEXT,

    window_title TEXT,
    document_path TEXT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- activity_screen_time (temporal)
CREATE TABLE IF NOT EXISTS activity_screen_time (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    device_name TEXT,
    device_type TEXT,

    total_screen_time_seconds INTEGER NOT NULL,
    unlock_count INTEGER,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- activity_web_browsing (temporal)
CREATE TABLE IF NOT EXISTS activity_web_browsing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    url TEXT NOT NULL,
    domain TEXT,
    page_title TEXT,

    visit_duration_seconds INTEGER,
    scroll_depth_percent FLOAT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- activity_focus_session (temporal)
CREATE TABLE IF NOT EXISTS activity_focus_session (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    session_type TEXT NOT NULL,
    task_description TEXT,

    topic_id UUID REFERENCES entities_topic(id),

    distraction_count INTEGER DEFAULT 0,
    focus_score FLOAT,

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT activity_focus_session_type_check
        CHECK (session_type IN ('deep_work', 'pomodoro', 'flow_state'))
);

CREATE INDEX idx_activity_focus_session_topic ON activity_focus_session(topic_id);

-- ============================================================================
-- FINANCE DOMAIN PRIMITIVES
-- Monetary transactions and account balances
-- ============================================================================

-- finance_balance (signal)
CREATE TABLE IF NOT EXISTS finance_balance (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    account_name TEXT NOT NULL,
    account_type TEXT,
    institution_name TEXT,

    balance_cents BIGINT NOT NULL,
    currency TEXT DEFAULT 'USD',

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- finance_transaction (temporal)
CREATE TABLE IF NOT EXISTS finance_transaction (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    transaction_id TEXT NOT NULL,
    transaction_type TEXT,

    description TEXT,
    merchant_name TEXT,

    amount_cents BIGINT NOT NULL,
    currency TEXT DEFAULT 'USD',

    account_name TEXT,
    account_type TEXT,

    category TEXT,
    subcategory TEXT,

    place_id UUID REFERENCES entities_place(id),

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_finance_transaction_place ON finance_transaction(place_id);

-- finance_subscription (temporal)
CREATE TABLE IF NOT EXISTS finance_subscription (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    service_name TEXT NOT NULL,
    subscription_type TEXT,

    amount_cents BIGINT NOT NULL,
    currency TEXT DEFAULT 'USD',
    billing_period_days INTEGER,

    status TEXT NOT NULL,

    start_date DATE,
    end_date DATE,
    next_billing_date DATE,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT finance_subscription_status_check
        CHECK (status IN ('active', 'cancelled', 'paused', 'trial'))
);

CREATE INDEX idx_finance_subscription_status ON finance_subscription(status);
CREATE INDEX idx_finance_subscription_next_billing ON finance_subscription(next_billing_date);

-- ============================================================================
-- AMBIENT DOMAIN PRIMITIVES
-- External environmental conditions
-- ============================================================================

-- ambient_weather (signal)
CREATE TABLE IF NOT EXISTS ambient_weather (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    temperature_celsius FLOAT,
    feels_like_celsius FLOAT,
    humidity_percent FLOAT,

    precipitation_mm FLOAT,
    wind_speed_kmh FLOAT,
    wind_direction_degrees FLOAT,

    condition_category TEXT,
    condition_description TEXT,

    pressure_hpa FLOAT,
    uv_index FLOAT,

    place_id UUID REFERENCES entities_place(id),
    latitude FLOAT,
    longitude FLOAT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ambient_weather_place ON ambient_weather(place_id);

-- ambient_air_quality (signal)
CREATE TABLE IF NOT EXISTS ambient_air_quality (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    aqi INTEGER,
    aqi_category TEXT,

    pm25 FLOAT,
    pm10 FLOAT,
    ozone FLOAT,
    no2 FLOAT,
    co FLOAT,
    so2 FLOAT,

    place_id UUID REFERENCES entities_place(id),
    latitude FLOAT,
    longitude FLOAT,

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ambient_air_quality_place ON ambient_air_quality(place_id);
CREATE INDEX idx_ambient_air_quality_source ON ambient_air_quality(source_stream_id);
CREATE TRIGGER ambient_air_quality_updated_at BEFORE UPDATE ON ambient_air_quality FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ambient_noise_level (signal)
CREATE TABLE IF NOT EXISTS ambient_noise_level (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    decibels FLOAT NOT NULL,
    noise_category TEXT,

    place_id UUID REFERENCES entities_place(id),

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ambient_noise_level_place ON ambient_noise_level(place_id);
CREATE INDEX idx_ambient_noise_level_source ON ambient_noise_level(source_stream_id);
CREATE TRIGGER ambient_noise_level_updated_at BEFORE UPDATE ON ambient_noise_level FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ambient_light_level (signal)
CREATE TABLE IF NOT EXISTS ambient_light_level (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    lux FLOAT NOT NULL,
    light_category TEXT,

    place_id UUID REFERENCES entities_place(id),

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ambient_light_level_place ON ambient_light_level(place_id);
CREATE INDEX idx_ambient_light_level_source ON ambient_light_level(source_stream_id);
CREATE TRIGGER ambient_light_level_updated_at BEFORE UPDATE ON ambient_light_level FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ambient_audio_classification (temporal)
CREATE TABLE IF NOT EXISTS ambient_audio_classification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    audio_class TEXT NOT NULL,
    confidence FLOAT,
    audio_subclass TEXT,
    volume_level_db FLOAT,

    place_id UUID REFERENCES entities_place(id),

    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ambient_audio_classification_start ON ambient_audio_classification(start_time DESC);
CREATE INDEX idx_ambient_audio_classification_class ON ambient_audio_classification(audio_class);
CREATE INDEX idx_ambient_audio_classification_place ON ambient_audio_classification(place_id);
CREATE INDEX idx_ambient_audio_classification_source ON ambient_audio_classification(source_stream_id);
CREATE TRIGGER ambient_audio_classification_updated_at BEFORE UPDATE ON ambient_audio_classification FOR EACH ROW EXECUTE FUNCTION update_updated_at();

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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_knowledge_document_title ON knowledge_document(title);
CREATE INDEX idx_knowledge_document_topic ON knowledge_document(topic_id);
CREATE INDEX idx_knowledge_document_tags ON knowledge_document USING GIN(tags);
CREATE INDEX idx_knowledge_document_source ON knowledge_document(source_stream_id);
CREATE INDEX idx_knowledge_document_search ON knowledge_document USING GIN(to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, '')));
CREATE TRIGGER knowledge_document_updated_at BEFORE UPDATE ON knowledge_document FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- knowledge_playlist (temporal)
CREATE TABLE IF NOT EXISTS knowledge_playlist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    name TEXT NOT NULL,
    description TEXT,
    playlist_type TEXT,

    external_id TEXT,
    external_url TEXT,

    item_count INTEGER DEFAULT 0,
    items JSONB DEFAULT '[]',

    is_public BOOLEAN DEFAULT false,

    created_time TIMESTAMPTZ,
    last_modified_time TIMESTAMPTZ,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT knowledge_playlist_type_check
        CHECK (playlist_type IS NULL OR playlist_type IN (
            'music', 'video', 'podcast', 'reading_list', 'watch_later'
        ))
);

CREATE INDEX idx_knowledge_playlist_name ON knowledge_playlist(name);
CREATE INDEX idx_knowledge_playlist_type ON knowledge_playlist(playlist_type);
CREATE INDEX idx_knowledge_playlist_source ON knowledge_playlist(source_stream_id);
CREATE TRIGGER knowledge_playlist_updated_at BEFORE UPDATE ON knowledge_playlist FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- knowledge_bookmark (temporal)
CREATE TABLE IF NOT EXISTS knowledge_bookmark (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    url TEXT NOT NULL,
    title TEXT,
    description TEXT,
    page_content TEXT,

    topic_id UUID REFERENCES entities_topic(id),
    tags TEXT[] DEFAULT '{}',

    saved_at TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_knowledge_bookmark_url ON knowledge_bookmark(url);
CREATE INDEX idx_knowledge_bookmark_topic ON knowledge_bookmark(topic_id);
CREATE INDEX idx_knowledge_bookmark_tags ON knowledge_bookmark USING GIN(tags);
CREATE INDEX idx_knowledge_bookmark_source ON knowledge_bookmark(source_stream_id);
CREATE TRIGGER knowledge_bookmark_updated_at BEFORE UPDATE ON knowledge_bookmark FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- knowledge_search (temporal)
CREATE TABLE IF NOT EXISTS knowledge_search (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    query TEXT NOT NULL,
    search_engine TEXT NOT NULL,

    result_count INTEGER,
    clicked_result_url TEXT,

    topic_id UUID REFERENCES entities_topic(id),
    inferred_keywords TEXT[] DEFAULT '{}',

    timestamp TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT knowledge_search_engine_check
        CHECK (search_engine IN ('google', 'chatgpt', 'perplexity', 'notion', 'github', 'other'))
);

CREATE INDEX idx_knowledge_search_query ON knowledge_search(query);
CREATE INDEX idx_knowledge_search_engine ON knowledge_search(search_engine);
CREATE INDEX idx_knowledge_search_topic ON knowledge_search(topic_id);
CREATE INDEX idx_knowledge_search_source ON knowledge_search(source_stream_id);
CREATE TRIGGER knowledge_search_updated_at BEFORE UPDATE ON knowledge_search FOR EACH ROW EXECUTE FUNCTION update_updated_at();

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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_speech_transcription_source ON speech_transcription(source_stream_id);
CREATE INDEX idx_speech_transcription_search ON speech_transcription USING GIN(to_tsvector('english', transcript_text));
CREATE TRIGGER speech_transcription_updated_at BEFORE UPDATE ON speech_transcription FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- INTROSPECTION DOMAIN PRIMITIVES
-- Self-reflection and metacognition
-- ============================================================================

-- introspection_journal (temporal)
CREATE TABLE IF NOT EXISTS introspection_journal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT,
    content TEXT NOT NULL,

    sentiment_score FLOAT,

    topic_ids UUID[] DEFAULT '{}',
    tags TEXT[] DEFAULT '{}',

    entry_type TEXT,
    entry_date DATE NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT introspection_journal_sentiment_check CHECK (sentiment_score IS NULL OR sentiment_score BETWEEN -1.0 AND 1.0),
    CONSTRAINT introspection_journal_entry_type_check
        CHECK (entry_type IS NULL OR entry_type IN ('written', 'voice_transcribed', 'prompted'))
);

CREATE INDEX idx_introspection_journal_entry_date ON introspection_journal(entry_date DESC);
CREATE INDEX idx_introspection_journal_topics ON introspection_journal USING GIN(topic_ids);
CREATE INDEX idx_introspection_journal_tags ON introspection_journal USING GIN(tags);
CREATE INDEX idx_introspection_journal_source ON introspection_journal(source_stream_id);
CREATE INDEX idx_introspection_journal_search ON introspection_journal USING GIN(to_tsvector('english', coalesce(title, '') || ' ' || content));
CREATE TRIGGER introspection_journal_updated_at BEFORE UPDATE ON introspection_journal FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- introspection_goal (temporal)
CREATE TABLE IF NOT EXISTS introspection_goal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT NOT NULL,
    description TEXT,
    goal_type TEXT,

    topic_id UUID REFERENCES entities_topic(id),

    status TEXT NOT NULL,
    progress_percent FLOAT DEFAULT 0,

    created_date DATE NOT NULL,
    target_date DATE,
    completed_date DATE,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT introspection_goal_status_check
        CHECK (status IN ('not_started', 'in_progress', 'completed', 'abandoned')),
    CONSTRAINT introspection_goal_progress_check CHECK (progress_percent BETWEEN 0 AND 100)
);

CREATE INDEX idx_introspection_goal_status ON introspection_goal(status);
CREATE INDEX idx_introspection_goal_topic ON introspection_goal(topic_id);
CREATE INDEX idx_introspection_goal_target ON introspection_goal(target_date);
CREATE INDEX idx_introspection_goal_source ON introspection_goal(source_stream_id);
CREATE TRIGGER introspection_goal_updated_at BEFORE UPDATE ON introspection_goal FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- introspection_gratitude (temporal)
CREATE TABLE IF NOT EXISTS introspection_gratitude (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    content TEXT NOT NULL,
    gratitude_category TEXT,

    person_ids UUID[] DEFAULT '{}',
    place_ids UUID[] DEFAULT '{}',

    entry_date DATE NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT introspection_gratitude_category_check
        CHECK (gratitude_category IS NULL OR gratitude_category IN (
            'people', 'experiences', 'achievements', 'material', 'health'
        ))
);

CREATE INDEX idx_introspection_gratitude_entry_date ON introspection_gratitude(entry_date DESC);
CREATE INDEX idx_introspection_gratitude_category ON introspection_gratitude(gratitude_category);
CREATE INDEX idx_introspection_gratitude_persons ON introspection_gratitude USING GIN(person_ids);
CREATE INDEX idx_introspection_gratitude_source ON introspection_gratitude(source_stream_id);
CREATE TRIGGER introspection_gratitude_updated_at BEFORE UPDATE ON introspection_gratitude FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- introspection_reflection (temporal)
CREATE TABLE IF NOT EXISTS introspection_reflection (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT,
    content TEXT NOT NULL,

    reflection_type TEXT,

    topic_ids UUID[] DEFAULT '{}',
    tags TEXT[] DEFAULT '{}',

    reflection_date DATE NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT introspection_reflection_type_check
        CHECK (reflection_type IS NULL OR reflection_type IN (
            'daily', 'weekly', 'event', 'decision', 'lesson_learned'
        ))
);

CREATE INDEX idx_introspection_reflection_reflection_date ON introspection_reflection(reflection_date DESC);
CREATE INDEX idx_introspection_reflection_type ON introspection_reflection(reflection_type);
CREATE INDEX idx_introspection_reflection_topics ON introspection_reflection USING GIN(topic_ids);
CREATE INDEX idx_introspection_reflection_source ON introspection_reflection(source_stream_id);
CREATE TRIGGER introspection_reflection_updated_at BEFORE UPDATE ON introspection_reflection FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- introspection_dream (temporal)
CREATE TABLE IF NOT EXISTS introspection_dream (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title TEXT,
    description TEXT NOT NULL,

    vividness TEXT,
    emotional_tone TEXT,

    tags TEXT[] DEFAULT '{}',

    dream_date DATE NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,

    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,

    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT introspection_dream_vividness_check
        CHECK (vividness IS NULL OR vividness IN ('low', 'medium', 'high')),
    CONSTRAINT introspection_dream_tone_check
        CHECK (emotional_tone IS NULL OR emotional_tone IN (
            'positive', 'negative', 'neutral', 'mixed'
        ))
);

CREATE INDEX idx_introspection_dream_dream_date ON introspection_dream(dream_date DESC);
CREATE INDEX idx_introspection_dream_tags ON introspection_dream USING GIN(tags);
CREATE INDEX idx_introspection_dream_source ON introspection_dream(source_stream_id);
CREATE TRIGGER introspection_dream_updated_at BEFORE UPDATE ON introspection_dream FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON SCHEMA elt IS 'Ariata ELT system: streams (raw) + ontologies (normalized)';
