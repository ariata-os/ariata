-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- iOS stream tables for device data ingestion
-- These tables store data from the iOS companion app

-- iOS HealthKit stream (heart rate, HRV, steps, sleep, workouts, etc.)
CREATE TABLE IF NOT EXISTS stream_ios_healthkit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp of the health measurement
    timestamp TIMESTAMPTZ NOT NULL,

    -- Cardiovascular metrics
    heart_rate FLOAT,                    -- beats per minute
    hrv FLOAT,                           -- heart rate variability in milliseconds
    resting_heart_rate FLOAT,            -- resting heart rate

    -- Activity metrics
    steps INTEGER,                       -- step count
    distance FLOAT,                      -- distance in meters
    active_energy FLOAT,                 -- active energy burned in kcal
    basal_energy FLOAT,                  -- basal energy burned in kcal
    flights_climbed INTEGER,             -- stairs climbed

    -- Sleep metrics
    sleep_stage TEXT,                    -- awake, light, deep, rem, asleep
    sleep_duration INTEGER,              -- duration in seconds

    -- Workout metrics
    workout_type TEXT,                   -- running, cycling, swimming, etc.
    workout_duration INTEGER,            -- workout duration in seconds

    -- Body metrics
    weight FLOAT,                        -- weight in kg
    body_fat_percentage FLOAT,           -- body fat %

    -- Mindfulness & Recovery
    mindful_minutes INTEGER,             -- mindfulness duration in minutes

    -- Device information
    device_name TEXT,                    -- e.g., "iPhone 15 Pro", "Apple Watch Series 9"
    device_model TEXT,

    -- Raw data backup (for fields not mapped to columns)
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_ios_healthkit_timestamp ON stream_ios_healthkit(timestamp DESC);
CREATE INDEX idx_ios_healthkit_source_time ON stream_ios_healthkit(source_id, timestamp DESC);
CREATE INDEX idx_ios_healthkit_heart_rate ON stream_ios_healthkit(timestamp) WHERE heart_rate IS NOT NULL;
CREATE INDEX idx_ios_healthkit_steps ON stream_ios_healthkit(timestamp) WHERE steps IS NOT NULL;
CREATE INDEX idx_ios_healthkit_sleep ON stream_ios_healthkit(timestamp) WHERE sleep_stage IS NOT NULL;
CREATE INDEX idx_ios_healthkit_workout ON stream_ios_healthkit(timestamp) WHERE workout_type IS NOT NULL;

-- Comments
COMMENT ON TABLE stream_ios_healthkit IS 'iOS HealthKit data including heart rate, HRV, steps, sleep, and workouts';
COMMENT ON COLUMN stream_ios_healthkit.heart_rate IS 'Heart rate in beats per minute';
COMMENT ON COLUMN stream_ios_healthkit.hrv IS 'Heart rate variability in milliseconds (SDNN)';
COMMENT ON COLUMN stream_ios_healthkit.steps IS 'Step count for the time period';
COMMENT ON COLUMN stream_ios_healthkit.sleep_stage IS 'Sleep stage: awake, light, deep, rem, asleep';
COMMENT ON COLUMN stream_ios_healthkit.workout_type IS 'Workout activity type from HealthKit';
COMMENT ON COLUMN stream_ios_healthkit.raw_data IS 'Complete HealthKit sample data for fields not mapped to columns';

-----------------------------------------------------------

-- iOS Location stream (GPS tracking)
CREATE TABLE IF NOT EXISTS stream_ios_location (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp of the location measurement
    timestamp TIMESTAMPTZ NOT NULL,

    -- Coordinates
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,
    altitude FLOAT,                      -- meters above sea level

    -- Movement metrics
    speed FLOAT,                         -- speed in m/s
    course FLOAT,                        -- direction in degrees (0-360)

    -- Accuracy metrics
    horizontal_accuracy FLOAT,           -- horizontal accuracy in meters
    vertical_accuracy FLOAT,             -- vertical accuracy in meters

    -- Activity inference
    activity_type TEXT,                  -- stationary, walking, running, automotive, cycling
    activity_confidence TEXT,            -- low, medium, high

    -- Floor level (for indoor positioning)
    floor_level INTEGER,

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_ios_location_timestamp ON stream_ios_location(timestamp DESC);
CREATE INDEX idx_ios_location_source_time ON stream_ios_location(source_id, timestamp DESC);

-- Geospatial index (requires PostGIS extension)
-- Uncomment if PostGIS is available:
-- CREATE INDEX idx_ios_location_coords ON stream_ios_location USING GIST (
--     ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)
-- );

-- Comments
COMMENT ON TABLE stream_ios_location IS 'iOS location data including GPS coordinates, speed, and activity type';
COMMENT ON COLUMN stream_ios_location.latitude IS 'Latitude in decimal degrees';
COMMENT ON COLUMN stream_ios_location.longitude IS 'Longitude in decimal degrees';
COMMENT ON COLUMN stream_ios_location.altitude IS 'Altitude in meters above sea level';
COMMENT ON COLUMN stream_ios_location.speed IS 'Speed in meters per second';
COMMENT ON COLUMN stream_ios_location.horizontal_accuracy IS 'Location accuracy radius in meters';
COMMENT ON COLUMN stream_ios_location.activity_type IS 'Inferred activity: stationary, walking, running, automotive, cycling';

-----------------------------------------------------------

-- iOS Microphone stream (audio levels and transcriptions)
CREATE TABLE IF NOT EXISTS stream_ios_microphone (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Timestamp of the audio sample
    timestamp TIMESTAMPTZ NOT NULL,

    -- Audio level metrics
    decibels FLOAT,                      -- sound level in dB
    average_power FLOAT,                 -- average power level
    peak_power FLOAT,                    -- peak power level

    -- Transcription (if available)
    transcription TEXT,                  -- transcribed text from audio
    transcription_confidence FLOAT,      -- confidence score 0-1
    language TEXT,                       -- detected language code (e.g., "en-US")

    -- Recording metadata
    duration_seconds INTEGER,            -- duration of audio sample
    sample_rate INTEGER,                 -- sample rate in Hz

    -- Storage reference (if audio stored in MinIO)
    audio_file_key TEXT,                 -- S3/MinIO object key for audio file
    audio_file_size INTEGER,             -- file size in bytes
    audio_format TEXT,                   -- format: m4a, wav, etc.

    -- Raw data backup
    raw_data JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_ios_microphone_timestamp ON stream_ios_microphone(timestamp DESC);
CREATE INDEX idx_ios_microphone_source_time ON stream_ios_microphone(source_id, timestamp DESC);
CREATE INDEX idx_ios_microphone_transcription ON stream_ios_microphone USING GIN (to_tsvector('english', transcription))
    WHERE transcription IS NOT NULL;

-- Comments
COMMENT ON TABLE stream_ios_microphone IS 'iOS microphone data including audio levels and transcriptions';
COMMENT ON COLUMN stream_ios_microphone.decibels IS 'Sound level in decibels';
COMMENT ON COLUMN stream_ios_microphone.transcription IS 'Transcribed text from audio recording';
COMMENT ON COLUMN stream_ios_microphone.audio_file_key IS 'MinIO/S3 object key for the full audio file';
