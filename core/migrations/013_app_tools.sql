-- Create app.tools table for tool metadata and configuration
CREATE TABLE app.tools (
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

-- Add pinned_tool_ids to assistant_profile
ALTER TABLE elt.assistant_profile
ADD COLUMN pinned_tool_ids TEXT[] DEFAULT ARRAY['queryLocationMap', 'queryPursuits'];

-- Create index for faster lookups
CREATE INDEX idx_tools_pinnable ON app.tools(is_pinnable) WHERE is_pinnable = true;
CREATE INDEX idx_tools_category ON app.tools(category);
