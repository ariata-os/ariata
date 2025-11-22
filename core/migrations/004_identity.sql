-- Axiology and identity tables in data schema
-- Note: search_path is set at database level, so we use qualified names

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

-- Removed axiology_habit table - habits are now recurring tasks in praxis_task

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

CREATE TABLE IF NOT EXISTS data.praxis_task (
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

    -- Habit fields (for recurring tasks that are habits)
    recurrence_rule TEXT,              -- 'daily', 'weekly', 'monthly', or RRULE
    is_habit BOOLEAN DEFAULT FALSE,    -- marks this as a habit
    current_streak INTEGER DEFAULT 0,  -- habit streak tracking
    best_streak INTEGER DEFAULT 0,     -- best ever streak
    last_completed_date DATE,          -- for streak calculation

    -- Hierarchical relations
    initiative_id UUID,                -- References praxis_initiative (will add FK after table creation)
    parent_task_id UUID,               -- References praxis_task (self-referential)

    -- Source integration
    source_provider TEXT DEFAULT 'internal',  -- 'internal', 'todoist', 'notion', etc.
    external_id TEXT,                  -- ID in external system
    external_url TEXT,                 -- Deep link to external system

    -- Axiological links
    virtue_ids UUID[],                 -- Links to axiology_virtue
    vice_ids UUID[],                   -- Links to axiology_vice
    value_ids UUID[],                  -- Links to axiology_value
    purpose TEXT,                      -- Why am I doing this?

    -- Context
    context_energy TEXT,               -- 'deep_focus', 'shallow', 'creative'
    context_location TEXT,             -- '@home', '@office', '@anywhere'
    estimated_minutes INTEGER,
    actual_minutes INTEGER,

    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_praxis_task_tags ON data.praxis_task USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_praxis_task_status ON data.praxis_task(status) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_praxis_task_habits ON data.praxis_task(is_habit) WHERE is_habit = true;
CREATE INDEX IF NOT EXISTS idx_praxis_task_recurrence ON data.praxis_task(recurrence_rule) WHERE recurrence_rule IS NOT NULL;

CREATE TABLE IF NOT EXISTS data.praxis_initiative (
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

    -- Hierarchical relations
    parent_initiative_id UUID,         -- References praxis_initiative (self-referential)

    -- Source integration
    source_provider TEXT DEFAULT 'internal',  -- 'internal', 'asana', 'notion', etc.
    external_id TEXT,
    external_url TEXT,

    -- Axiological links (initiatives often have explicit purpose)
    virtue_ids UUID[],
    vice_ids UUID[],
    value_ids UUID[],
    purpose TEXT,

    -- For commitment-level initiatives (overcoming addiction, etc.)
    is_commitment BOOLEAN DEFAULT FALSE,
    success_metrics JSONB,             -- {"days_clean": 60}
    current_metrics JSONB,             -- {"days_clean": 12}

    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_praxis_initiative_tags ON data.praxis_initiative USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_praxis_initiative_status ON data.praxis_initiative(status) WHERE is_active = true;

CREATE TABLE IF NOT EXISTS data.praxis_aspiration (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    topic_id UUID REFERENCES data.entities_topic(id),
    status TEXT DEFAULT 'dreaming',
    target_timeframe TEXT,              -- 'this_year', 'next_5_years', 'lifetime', 'undefined'
    achieved_date TIMESTAMPTZ,

    -- When activated
    activated_date DATE,                -- When moved from someday to active
    activated_as_initiative_id UUID,    -- References praxis_initiative (will add FK below)

    -- Axiological links (aspirations are usually values-aligned)
    virtue_ids UUID[],
    value_ids UUID[],
    purpose TEXT,

    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_praxis_aspiration_tags ON data.praxis_aspiration USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_praxis_aspiration_status ON data.praxis_aspiration(status) WHERE is_active = true;

-- Add foreign key constraints after all tables are created (with existence checks)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_task_initiative'
        AND conrelid = 'data.praxis_task'::regclass
    ) THEN
        ALTER TABLE data.praxis_task ADD CONSTRAINT fk_praxis_task_initiative
            FOREIGN KEY (initiative_id) REFERENCES data.praxis_initiative(id) ON DELETE SET NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_task_parent'
        AND conrelid = 'data.praxis_task'::regclass
    ) THEN
        ALTER TABLE data.praxis_task ADD CONSTRAINT fk_praxis_task_parent
            FOREIGN KEY (parent_task_id) REFERENCES data.praxis_task(id) ON DELETE CASCADE;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_initiative_parent'
        AND conrelid = 'data.praxis_initiative'::regclass
    ) THEN
        ALTER TABLE data.praxis_initiative ADD CONSTRAINT fk_praxis_initiative_parent
            FOREIGN KEY (parent_initiative_id) REFERENCES data.praxis_initiative(id) ON DELETE CASCADE;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_aspiration_initiative'
        AND conrelid = 'data.praxis_aspiration'::regclass
    ) THEN
        ALTER TABLE data.praxis_aspiration ADD CONSTRAINT fk_praxis_aspiration_initiative
            FOREIGN KEY (activated_as_initiative_id) REFERENCES data.praxis_initiative(id) ON DELETE SET NULL;
    END IF;
END $$;

-- Add foreign key constraints for praxis_calendar (table defined in 003_entities_and_ontology.sql)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_calendar_task'
        AND conrelid = 'data.praxis_calendar'::regclass
    ) THEN
        ALTER TABLE data.praxis_calendar ADD CONSTRAINT fk_praxis_calendar_task
            FOREIGN KEY (task_id) REFERENCES data.praxis_task(id) ON DELETE SET NULL;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_praxis_calendar_initiative'
        AND conrelid = 'data.praxis_calendar'::regclass
    ) THEN
        ALTER TABLE data.praxis_calendar ADD CONSTRAINT fk_praxis_calendar_initiative
            FOREIGN KEY (initiative_id) REFERENCES data.praxis_initiative(id) ON DELETE SET NULL;
    END IF;
END $$;

-- Note: narrative_chunks table removed - superseded by narrative_primitive in migration 006

CREATE TABLE IF NOT EXISTS data.prudent_context_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    context_data JSONB NOT NULL,
    llm_model TEXT,
    token_count INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_prudent_context_expires ON data.prudent_context_snapshot(expires_at DESC, computed_at DESC);

CREATE OR REPLACE FUNCTION cleanup_expired_context()
RETURNS void AS $$
BEGIN
    DELETE FROM data.prudent_context_snapshot
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
    is_onboarding BOOLEAN NOT NULL DEFAULT true,
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
