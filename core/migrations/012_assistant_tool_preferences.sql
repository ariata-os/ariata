-- Add tool preferences to assistant profile
-- This allows users to enable/disable specific tools and widgets

ALTER TABLE elt.assistant_profile
ADD COLUMN IF NOT EXISTS enabled_tools JSONB DEFAULT '{}';

COMMENT ON COLUMN elt.assistant_profile.enabled_tools IS
'User preferences for enabling/disabling specific tools.
Empty object {} means all tools are enabled by default.
Example: {"queryLocationMap": true, "queryPursuits": false}';
