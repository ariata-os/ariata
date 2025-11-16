-- Assistant Profile Settings
-- Singleton table for storing user's AI assistant preferences
CREATE TABLE IF NOT EXISTS elt.assistant_profile (
    id UUID PRIMARY KEY DEFAULT '00000000-0000-0000-0000-000000000001'::uuid,
    assistant_name TEXT DEFAULT 'Assistant',
    default_agent_id TEXT DEFAULT 'auto',
    default_model_id TEXT DEFAULT 'openai/gpt-oss-120b',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT assistant_profile_singleton CHECK (id = '00000000-0000-0000-0000-000000000001'::uuid)
);

-- Insert default row
INSERT INTO elt.assistant_profile (id)
VALUES ('00000000-0000-0000-0000-000000000001'::uuid)
ON CONFLICT (id) DO NOTHING;

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_assistant_profile_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS assistant_profile_updated_at ON elt.assistant_profile;
CREATE TRIGGER assistant_profile_updated_at
    BEFORE UPDATE ON elt.assistant_profile
    FOR EACH ROW
    EXECUTE FUNCTION update_assistant_profile_updated_at();
