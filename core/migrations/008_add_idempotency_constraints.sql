-- Add unique constraints for idempotent inserts
-- This ensures we can safely re-run syncs without creating duplicates

-- iOS HealthKit: One sample per device per timestamp
ALTER TABLE stream_ios_healthkit
    ADD CONSTRAINT unique_ios_healthkit_sample
    UNIQUE (source_id, timestamp);

-- iOS Location: One location point per device per timestamp
ALTER TABLE stream_ios_location
    ADD CONSTRAINT unique_ios_location_point
    UNIQUE (source_id, timestamp);

-- iOS Microphone: One audio sample per device per timestamp
ALTER TABLE stream_ios_microphone
    ADD CONSTRAINT unique_ios_microphone_sample
    UNIQUE (source_id, timestamp);

-- Mac Apps: One app usage record per device per timestamp per app
-- Using a partial unique index to allow multiple apps at the same timestamp
CREATE UNIQUE INDEX unique_mac_apps_usage
    ON stream_mac_apps(source_id, timestamp, app_name);

-- Mac Browser: One URL visit per device per timestamp per URL
-- This allows revisiting same URL at different times
CREATE UNIQUE INDEX unique_mac_browser_visit
    ON stream_mac_browser(source_id, url, timestamp);

-- Mac iMessage: One message per device per timestamp per contact per direction
-- Using hash of relevant fields to create a unique identifier
-- Note: timestamp alone isn't sufficient because multiple messages can be sent/received in the same second
ALTER TABLE stream_mac_imessage
    ADD CONSTRAINT unique_mac_imessage_message
    UNIQUE (source_id, timestamp, contact_id, is_from_me);

-- Add comments explaining the constraints
COMMENT ON CONSTRAINT unique_ios_healthkit_sample ON stream_ios_healthkit
    IS 'Ensures idempotent inserts: one health sample per device per timestamp';

COMMENT ON CONSTRAINT unique_ios_location_point ON stream_ios_location
    IS 'Ensures idempotent inserts: one location point per device per timestamp';

COMMENT ON CONSTRAINT unique_ios_microphone_sample ON stream_ios_microphone
    IS 'Ensures idempotent inserts: one audio sample per device per timestamp';

COMMENT ON INDEX unique_mac_apps_usage
    IS 'Ensures idempotent inserts: one app usage record per device per timestamp per app';

COMMENT ON INDEX unique_mac_browser_visit
    IS 'Ensures idempotent inserts: one browser visit per device per URL per timestamp';

COMMENT ON CONSTRAINT unique_mac_imessage_message ON stream_mac_imessage
    IS 'Ensures idempotent inserts: one message per device per timestamp per contact per direction';
