-- Create app schema for application-level data
-- This includes user preferences, configurations, and app state
-- Distinct from elt schema which contains transformed event data

CREATE SCHEMA IF NOT EXISTS app;

-- Models configuration
-- Stores available LLM models and user preferences
CREATE TABLE app.models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES elt.entities_person(id) ON DELETE CASCADE, -- NULL = system default
    model_id TEXT NOT NULL, -- e.g., 'anthropic/claude-sonnet-4.5'
    display_name TEXT NOT NULL,
    provider TEXT NOT NULL, -- e.g., 'Anthropic', 'OpenAI'
    enabled BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint for user-specific models (user_id + model_id)
CREATE UNIQUE INDEX idx_models_user_model ON app.models(user_id, model_id) WHERE user_id IS NOT NULL;

-- Unique constraint for system defaults (model_id only when user_id is NULL)
CREATE UNIQUE INDEX idx_models_system_model ON app.models(model_id) WHERE user_id IS NULL;

CREATE INDEX idx_models_user_id ON app.models(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_models_enabled ON app.models(enabled) WHERE enabled = true;

COMMENT ON TABLE app.models IS 'LLM model configurations - system defaults (user_id=NULL) and user-specific overrides';
COMMENT ON COLUMN app.models.user_id IS 'NULL for system defaults, UUID for user-specific models';

-- Agents configuration
-- Stores available agents and their display metadata
CREATE TABLE app.agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES elt.entities_person(id) ON DELETE CASCADE, -- NULL = system default
    agent_id TEXT NOT NULL, -- e.g., 'analytics', 'research'
    name TEXT NOT NULL,
    description TEXT,
    color TEXT, -- Hex color for UI (e.g., '#3b82f6')
    icon TEXT, -- Icon identifier (e.g., 'ri:bar-chart-line')
    enabled BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint for user-specific agents (user_id + agent_id)
CREATE UNIQUE INDEX idx_agents_user_agent ON app.agents(user_id, agent_id) WHERE user_id IS NOT NULL;

-- Unique constraint for system defaults (agent_id only when user_id is NULL)
CREATE UNIQUE INDEX idx_agents_system_agent ON app.agents(agent_id) WHERE user_id IS NULL;

CREATE INDEX idx_agents_user_id ON app.agents(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_agents_enabled ON app.agents(enabled) WHERE enabled = true;

COMMENT ON TABLE app.agents IS 'Agent configurations - system defaults (user_id=NULL) and user-specific overrides';
COMMENT ON COLUMN app.agents.user_id IS 'NULL for system defaults, UUID for user-specific agents';
