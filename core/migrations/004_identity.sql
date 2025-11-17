SET search_path TO data, public;

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS data.axiology_value (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    topic_id UUID REFERENCES data.entities_topic(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.axiology_telos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    topic_id UUID REFERENCES data.entities_topic(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_axiology_telos_single_active
    ON data.axiology_telos(is_active)
    WHERE is_active = true;

CREATE TABLE IF NOT EXISTS data.axiology_virtue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    topic_id UUID REFERENCES data.entities_topic(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.axiology_vice (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    topic_id UUID REFERENCES data.entities_topic(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.axiology_habit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    frequency TEXT,
    time_of_day TEXT,
    topic_id UUID REFERENCES data.entities_topic(id),
    streak_count INTEGER DEFAULT 0,
    last_completed_date DATE,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.axiology_temperament (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    temperament_type TEXT,
    topic_id UUID REFERENCES data.entities_topic(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.axiology_preference (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    preference_domain TEXT,
    valence TEXT,
    person_id UUID REFERENCES data.entities_person(id),
    place_id UUID REFERENCES data.entities_place(id),
    topic_id UUID REFERENCES data.entities_topic(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS data.actions_task (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    topic_id UUID REFERENCES data.entities_topic(id),
    status TEXT DEFAULT 'active',
    progress_percent INTEGER,
    start_date TIMESTAMPTZ,
    target_date TIMESTAMPTZ,
    completed_date TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_actions_task_tags ON data.actions_task USING GIN (tags);

CREATE TABLE IF NOT EXISTS data.actions_initiative (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    topic_id UUID REFERENCES data.entities_topic(id),
    status TEXT DEFAULT 'active',
    progress_percent INTEGER,
    start_date TIMESTAMPTZ,
    target_date TIMESTAMPTZ,
    completed_date TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_actions_initiative_tags ON data.actions_initiative USING GIN (tags);

CREATE TABLE IF NOT EXISTS data.actions_aspiration (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    topic_id UUID REFERENCES data.entities_topic(id),
    status TEXT DEFAULT 'dreaming',
    target_timeframe TEXT,
    achieved_date TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_actions_aspiration_tags ON data.actions_aspiration USING GIN (tags);

CREATE TABLE IF NOT EXISTS narrative_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    narrative_text TEXT NOT NULL,
    narrative_type TEXT NOT NULL,
    time_start TIMESTAMPTZ NOT NULL,
    time_end TIMESTAMPTZ NOT NULL,
    time_granularity TEXT NOT NULL,
    parent_narrative_id UUID REFERENCES narrative_chunks(id) ON DELETE CASCADE,
    child_narrative_ids UUID[],
    ontology_primitive_ids JSONB NOT NULL DEFAULT '{}'::jsonb,
    person_ids UUID[],
    place_ids UUID[],
    topic_ids UUID[],
    embedding vector(1536),
    token_count INTEGER,
    confidence_score FLOAT CHECK (confidence_score >= 0 AND confidence_score <= 1),
    generation_model TEXT,
    generated_by TEXT NOT NULL DEFAULT 'narrative_agent_v1',
    generation_prompt_version TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (time_end > time_start),
    CHECK (narrative_type IN ('action', 'event', 'day', 'week', 'chapter', 'telos'))
);

CREATE INDEX idx_narrative_time_range
    ON narrative_chunks(time_start, time_end);

CREATE INDEX idx_narrative_parent
    ON narrative_chunks(parent_narrative_id)
    WHERE parent_narrative_id IS NOT NULL;

CREATE INDEX idx_narrative_type
    ON narrative_chunks(narrative_type);

CREATE INDEX idx_narrative_people
    ON narrative_chunks USING GIN (person_ids);

CREATE INDEX idx_narrative_places
    ON narrative_chunks USING GIN (place_ids);

CREATE INDEX idx_narrative_topics
    ON narrative_chunks USING GIN (topic_ids);

CREATE INDEX idx_narrative_primitives
    ON narrative_chunks USING GIN (ontology_primitive_ids jsonb_path_ops);

CREATE TRIGGER narrative_chunks_updated_at
    BEFORE UPDATE ON narrative_chunks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TABLE IF NOT EXISTS prudent_context_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    context_data JSONB NOT NULL,
    llm_model TEXT,
    token_count INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_prudent_context_expires ON prudent_context_snapshot(expires_at DESC, computed_at DESC);

CREATE OR REPLACE FUNCTION cleanup_expired_context()
RETURNS void AS $$
BEGIN
    DELETE FROM prudent_context_snapshot
    WHERE expires_at < NOW() - INTERVAL '7 days';
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS data.user_profile (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name TEXT,
    preferred_name TEXT,
    birth_date DATE,
    height_cm NUMERIC(5,2),
    weight_kg NUMERIC(5,2),
    ethnicity TEXT,
    home_street TEXT,
    home_city TEXT,
    home_state TEXT,
    home_postal_code TEXT,
    home_country TEXT,
    occupation TEXT,
    employer TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_profile_singleton CHECK (id = '00000000-0000-0000-0000-000000000001'::uuid)
);

INSERT INTO data.user_profile (id)
VALUES ('00000000-0000-0000-0000-000000000001'::uuid)
ON CONFLICT (id) DO NOTHING;

DROP TRIGGER IF EXISTS user_profile_updated_at ON data.user_profile;
CREATE TRIGGER user_profile_updated_at
    BEFORE UPDATE ON data.user_profile
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE INDEX IF NOT EXISTS idx_user_profile_preferred_name ON data.user_profile(preferred_name);
CREATE INDEX IF NOT EXISTS idx_user_profile_full_name ON data.user_profile(full_name);
