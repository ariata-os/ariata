-- Axiology System: Hierarchy of values and temporal pursuits
--
-- "What you do reveals what you value" - Axiology is the study of values
-- and how they shape your life across different time horizons.
--
-- Architecture:
-- Level 0: VALUES (foundational principles - future)
-- Level 1: TELOS (ultimate purpose - lifetime, singular active)
-- Level 2: TEMPORAL PURSUITS (tasks, initiatives, aspirations)
--   - TASKS: Daily/weekly completables
--   - INITIATIVES: Medium-term commitments (month-quarter)
--   - ASPIRATIONS: Long-term life goals (multi-year)
-- Level 3: PATTERNS (virtues, vices, habits, temperaments - future)
-- Level 4: PREFERENCES (affinities with entities - future)

-- Level 0: Foundational principles
CREATE TABLE IF NOT EXISTS elt.axiology_value (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 1: Ultimate purpose (singular active)
CREATE TABLE IF NOT EXISTS elt.axiology_telos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Singular active constraint
    is_active BOOLEAN DEFAULT true,

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Constraint: Only one active telos at a time
CREATE UNIQUE INDEX IF NOT EXISTS idx_axiology_telos_single_active
    ON elt.axiology_telos(is_active)
    WHERE is_active = true;

-- Level 2a: Tasks (daily/weekly completables)
CREATE TABLE IF NOT EXISTS elt.axiology_task (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Task type
    task_type TEXT,  -- 'work', 'character', 'experiential', 'relational'

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Progress tracking
    status TEXT DEFAULT 'active',  -- 'active', 'on_hold', 'completed', 'abandoned'
    progress_percent INTEGER,  -- 0-100

    -- Time fields
    start_date TIMESTAMPTZ,
    target_date TIMESTAMPTZ,
    completed_date TIMESTAMPTZ,

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 2b: Initiatives (medium-term commitments, month-quarter)
CREATE TABLE IF NOT EXISTS elt.axiology_initiative (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Initiative type
    initiative_type TEXT,  -- 'work', 'character', 'skill_building', 'relational'

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Progress tracking
    status TEXT DEFAULT 'active',  -- 'planning', 'active', 'on_hold', 'completed', 'abandoned'
    progress_percent INTEGER,  -- 0-100

    -- Time fields
    start_date TIMESTAMPTZ,
    target_date TIMESTAMPTZ,
    completed_date TIMESTAMPTZ,

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 2c: Aspirations (long-term life goals, multi-year)
CREATE TABLE IF NOT EXISTS elt.axiology_aspiration (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Aspiration type
    aspiration_type TEXT,  -- 'bucket_list', 'life_goal', 'legacy', 'mastery'

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    status TEXT DEFAULT 'dreaming',  -- 'dreaming', 'planning', 'pursuing', 'achieved', 'let_go'
    target_timeframe TEXT,  -- e.g. "next 5 years", "by age 50", "someday"
    achieved_date TIMESTAMPTZ,

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 3: Virtues (positive character patterns to cultivate)
CREATE TABLE IF NOT EXISTS elt.axiology_virtue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 3: Vices (negative character patterns to resist)
CREATE TABLE IF NOT EXISTS elt.axiology_vice (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 3: Habits (daily practices, neutral, changeable)
CREATE TABLE IF NOT EXISTS elt.axiology_habit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Habit-specific fields
    frequency TEXT,  -- 'daily', 'weekly', 'monthly'
    time_of_day TEXT,  -- 'morning', 'afternoon', 'evening', 'night'

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Tracking
    streak_count INTEGER DEFAULT 0,
    last_completed_date DATE,

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 3: Temperaments (innate, neutral, stable dispositions)
CREATE TABLE IF NOT EXISTS elt.axiology_temperament (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Temperament type
    temperament_type TEXT,  -- 'chronotype', 'neurodivergence', 'personality', 'biological', 'archetypal'

    -- Context
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Level 4: Preferences (affinities with other entities)
CREATE TABLE IF NOT EXISTS elt.axiology_preference (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT,

    -- Preference type
    preference_domain TEXT,  -- 'work_environment', 'people', 'places', 'communication', 'activities'
    valence TEXT,  -- 'strong_preference', 'mild_preference', 'neutral', 'mild_aversion', 'strong_aversion'

    -- Entity references (nullable, one or more may be set)
    person_id UUID REFERENCES elt.entities_person(id),
    place_id UUID REFERENCES elt.entities_place(id),
    topic_id UUID REFERENCES elt.entities_topic(id),

    -- Status
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Comments for documentation
COMMENT ON TABLE elt.axiology_value IS 'Level 0: Foundational principles that inform telos';
COMMENT ON TABLE elt.axiology_telos IS 'Level 1: Ultimate life purpose (singular active)';
COMMENT ON TABLE elt.axiology_task IS 'Level 2a: Daily/weekly completable tasks';
COMMENT ON TABLE elt.axiology_initiative IS 'Level 2b: Medium-term commitments (month-quarter)';
COMMENT ON TABLE elt.axiology_aspiration IS 'Level 2c: Long-term life goals (multi-year)';
COMMENT ON TABLE elt.axiology_virtue IS 'Level 3: Positive character patterns to cultivate';
COMMENT ON TABLE elt.axiology_vice IS 'Level 3: Negative character patterns to resist';
COMMENT ON TABLE elt.axiology_habit IS 'Level 3: Daily practices (neutral, changeable)';
COMMENT ON TABLE elt.axiology_temperament IS 'Level 3: Innate dispositions (neutral, stable)';
COMMENT ON TABLE elt.axiology_preference IS 'Level 4: Affinities with other entities (person/place/topic)';
