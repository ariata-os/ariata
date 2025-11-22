-- Event Boundaries: Temporal skeleton of the day
-- This table stores detected boundaries (transition points) without narrative

CREATE TABLE IF NOT EXISTS data.event_boundaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    timestamp TIMESTAMPTZ NOT NULL,
    boundary_type TEXT NOT NULL CHECK (boundary_type IN ('begin', 'end')),

    -- Source information
    source_ontology TEXT NOT NULL, -- 'health_sleep', 'location_visit', 'praxis_calendar', etc.
    fidelity FLOAT NOT NULL CHECK (fidelity >= 0.0 AND fidelity <= 1.0),

    -- Weight for aggregation (higher = more significant)
    -- location=100, calendar=80, app_usage=60
    weight INTEGER NOT NULL DEFAULT 50,

    -- Aggregation support
    is_primary BOOLEAN DEFAULT FALSE, -- Marks strongest boundary in aggregated group

    -- Source-specific metadata
    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure unique boundaries (no duplicates at same timestamp from same source)
    UNIQUE(timestamp, source_ontology, boundary_type)
);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_event_boundaries_timestamp
ON data.event_boundaries(timestamp);

-- Index for source-based queries
CREATE INDEX IF NOT EXISTS idx_event_boundaries_source
ON data.event_boundaries(source_ontology);

-- Index for finding latest boundary
CREATE INDEX IF NOT EXISTS idx_event_boundaries_timestamp_desc
ON data.event_boundaries(timestamp DESC);

-- Comments
COMMENT ON TABLE data.event_boundaries IS 'Detected temporal boundaries (changepoints) in a person''s day. Stores only WHEN things changed, not WHAT happened.';
COMMENT ON COLUMN data.event_boundaries.fidelity IS 'Confidence score 0.0-1.0. Calendar events = 1.0, sleep = 0.95, location = 0.90, etc.';
COMMENT ON COLUMN data.event_boundaries.metadata IS 'Source-specific context. For sleep: {type: "sleep_start"}. For location: {type: "arrival", place_id: "uuid"}.';
COMMENT ON COLUMN data.event_boundaries.weight IS 'Significance weight for aggregation. Location=100 (master container), calendar=80 (structure), app_usage=60 (substance). Used to determine which boundaries create narrative primitives.';
COMMENT ON COLUMN data.event_boundaries.is_primary IS 'Marks the strongest boundary within an aggregated group (highest weight). Primary boundaries typically trigger new narrative primitives.';
