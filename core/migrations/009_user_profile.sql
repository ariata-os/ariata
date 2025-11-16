-- ============================================================================
-- USER PROFILE: Non-ephemeral biographical metadata
-- ============================================================================
-- Lives in elt schema (data warehouse) for durability
-- Single-row table (enforced by singleton constraint)
-- Used by LLM system prompts and context generation
-- ============================================================================

CREATE TABLE IF NOT EXISTS elt.user_profile (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Identity
    full_name TEXT,
    preferred_name TEXT,
    birth_date DATE,

    -- Physical/Biometric
    height_cm NUMERIC(5,2),           -- e.g., 175.50 cm
    weight_kg NUMERIC(5,2),           -- e.g., 70.50 kg
    ethnicity TEXT,

    -- Home Address
    home_street TEXT,
    home_city TEXT,
    home_state TEXT,
    home_postal_code TEXT,
    home_country TEXT,

    -- Work/Occupation
    occupation TEXT,                  -- Job title/profession (e.g., "Software Engineer", "Student")
    employer TEXT,                    -- Company name (nullable for freelance/student/retired)

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Singleton constraint (only one user profile allowed)
    CONSTRAINT user_profile_singleton CHECK (id = '00000000-0000-0000-0000-000000000001'::uuid)
);

-- Insert default empty profile (idempotent)
INSERT INTO elt.user_profile (id)
VALUES ('00000000-0000-0000-0000-000000000001'::uuid)
ON CONFLICT (id) DO NOTHING;

-- Auto-update timestamp trigger
DROP TRIGGER IF EXISTS user_profile_updated_at ON elt.user_profile;
CREATE TRIGGER user_profile_updated_at
    BEFORE UPDATE ON elt.user_profile
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_profile_preferred_name ON elt.user_profile(preferred_name);
CREATE INDEX IF NOT EXISTS idx_user_profile_full_name ON elt.user_profile(full_name);

-- Comments
COMMENT ON TABLE elt.user_profile IS 'Biographical metadata about the user. Single-row table used for system prompts and context generation.';
COMMENT ON COLUMN elt.user_profile.preferred_name IS 'How the AI assistant should address the user (overrides full_name if set)';
COMMENT ON COLUMN elt.user_profile.occupation IS 'Job title or profession (e.g., "Software Engineer", "Student", "Retired Teacher")';
COMMENT ON COLUMN elt.user_profile.employer IS 'Company or organization name (optional for freelance/student/retired)';
