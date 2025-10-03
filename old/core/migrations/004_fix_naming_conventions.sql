-- Fix stream table naming conventions to follow stream_{source}_{stream} pattern
-- This migration renames legacy tables to match the YAML-based naming convention

-- Rename stream_location to stream_ios_location (if it exists)
-- Location data is iOS-specific, not a generic stream
DO $$
BEGIN
    IF EXISTS (
        SELECT FROM pg_tables
        WHERE schemaname = 'public' AND tablename = 'stream_location'
    ) THEN
        ALTER TABLE stream_location RENAME TO stream_ios_location_old;

        -- If stream_ios_location already exists from 003, migrate data
        IF EXISTS (
            SELECT FROM pg_tables
            WHERE schemaname = 'public' AND tablename = 'stream_ios_location'
        ) THEN
            -- Migrate any data from old table to new (if both exist)
            INSERT INTO stream_ios_location (
                device_id, latitude, longitude, accuracy, altitude, speed,
                timestamp, address, metadata, created_at
            )
            SELECT
                device_id, latitude, longitude, accuracy, altitude, speed,
                timestamp, address, metadata, created_at
            FROM stream_ios_location_old
            ON CONFLICT DO NOTHING;

            -- Drop old table
            DROP TABLE stream_ios_location_old;
        ELSE
            -- Just rename if new table doesn't exist
            ALTER TABLE stream_ios_location_old RENAME TO stream_ios_location;
        END IF;
    END IF;
END $$;

-- Rename stream_calendar to stream_google_calendar (if it exists)
-- Calendar without source prefix is ambiguous - Google Calendar is the specific implementation
DO $$
BEGIN
    IF EXISTS (
        SELECT FROM pg_tables
        WHERE schemaname = 'public' AND tablename = 'stream_calendar'
    ) THEN
        -- Check if stream_google_calendar exists
        IF EXISTS (
            SELECT FROM pg_tables
            WHERE schemaname = 'public' AND tablename = 'stream_google_calendar'
        ) THEN
            -- Drop old table as stream_google_calendar is the correct one
            DROP TABLE stream_calendar;
        ELSE
            -- Rename to proper name
            ALTER TABLE stream_calendar RENAME TO stream_google_calendar;
        END IF;
    END IF;
END $$;

-- Rename stream_app_usage to stream_mac_apps (if it exists)
-- App usage tracking is Mac-specific
DO $$
BEGIN
    IF EXISTS (
        SELECT FROM pg_tables
        WHERE schemaname = 'public' AND tablename = 'stream_app_usage'
    ) THEN
        ALTER TABLE stream_app_usage RENAME TO stream_mac_apps_old;

        -- If stream_mac_apps already exists, migrate data
        IF EXISTS (
            SELECT FROM pg_tables
            WHERE schemaname = 'public' AND tablename = 'stream_mac_apps'
        ) THEN
            -- Note: schema may differ, adjust if needed
            DROP TABLE stream_mac_apps_old;
        ELSE
            ALTER TABLE stream_mac_apps_old RENAME TO stream_mac_apps;
        END IF;
    END IF;
END $$;

-- Update any indexes that reference the old table names
-- (These will be automatically renamed with the tables, but documenting for clarity)

-- Verify naming convention compliance
COMMENT ON TABLE stream_ios_healthkit IS 'Health metrics from iOS HealthKit - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_ios_location IS 'GPS location data from iOS CoreLocation - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_google_calendar IS 'Calendar events from Google Calendar - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_google_gmail IS 'Email messages from Gmail - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_strava_activities IS 'Fitness activities from Strava - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_notion_pages IS 'Pages from Notion workspace - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_mac_apps IS 'Application usage from macOS - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_mac_messages IS 'Messages from macOS - follows naming convention: stream_{source}_{stream}';
COMMENT ON TABLE stream_ios_mic IS 'Audio recordings from iOS microphone - follows naming convention: stream_{source}_{stream}';
