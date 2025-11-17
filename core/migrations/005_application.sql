SET search_path TO app, public;

CREATE TABLE assistant_profile (
    id UUID PRIMARY KEY DEFAULT '00000000-0000-0000-0000-000000000001'::uuid,
    assistant_name TEXT DEFAULT 'Assistant',
    default_agent_id TEXT DEFAULT 'auto',
    default_model_id TEXT DEFAULT 'openai/gpt-oss-120b',
    enabled_tools JSONB DEFAULT '{"queryLocationMap": true, "queryPursuits": true}',
    pinned_tool_ids TEXT[] DEFAULT ARRAY['queryLocationMap', 'queryPursuits'],
    ui_preferences JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT assistant_profile_singleton CHECK (id = '00000000-0000-0000-0000-000000000001'::uuid)
);

INSERT INTO assistant_profile (id)
VALUES ('00000000-0000-0000-0000-000000000001'::uuid)
ON CONFLICT (id) DO NOTHING;

CREATE OR REPLACE FUNCTION update_assistant_profile_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS assistant_profile_updated_at ON assistant_profile;
CREATE TRIGGER assistant_profile_updated_at
    BEFORE UPDATE ON assistant_profile
    FOR EACH ROW
    EXECUTE FUNCTION update_assistant_profile_updated_at();

CREATE TABLE chat_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    messages JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_chat_sessions_updated_at
    ON chat_sessions(updated_at DESC);

CREATE INDEX idx_chat_sessions_message_count
    ON chat_sessions(message_count);

CREATE TABLE models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES data.entities_person(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    context_window INTEGER,
    max_output_tokens INTEGER,
    supports_tools BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_models_user_model ON models(user_id, model_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX idx_models_system_model ON models(model_id) WHERE user_id IS NULL;
CREATE INDEX idx_models_user_id ON models(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_models_enabled ON models(enabled) WHERE enabled = true;

CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES data.entities_person(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT,
    icon TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_agents_user_agent ON agents(user_id, agent_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX idx_agents_system_agent ON agents(agent_id) WHERE user_id IS NULL;
CREATE INDEX idx_agents_user_id ON agents(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_agents_enabled ON agents(enabled) WHERE enabled = true;

CREATE TABLE tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT,
    icon TEXT,
    is_pinnable BOOLEAN DEFAULT false,
    default_params JSONB,
    display_order INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tools_pinnable ON tools(is_pinnable) WHERE is_pinnable = true;
CREATE INDEX idx_tools_category ON tools(category);

CREATE TABLE api_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint TEXT NOT NULL,
    day_bucket DATE NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    token_count INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd DECIMAL(10, 4) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_endpoint_day UNIQUE(endpoint, day_bucket),
    CONSTRAINT positive_request_count CHECK (request_count >= 0),
    CONSTRAINT positive_token_count CHECK (token_count >= 0),
    CONSTRAINT positive_input_tokens CHECK (input_tokens >= 0),
    CONSTRAINT positive_output_tokens CHECK (output_tokens >= 0),
    CONSTRAINT positive_cost CHECK (estimated_cost_usd >= 0)
);

CREATE INDEX idx_usage_day ON api_usage(day_bucket DESC);
CREATE INDEX idx_usage_endpoint ON api_usage(endpoint);
CREATE INDEX idx_usage_endpoint_day ON api_usage(endpoint, day_bucket);
