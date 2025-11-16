-- Location Visit Table: Semantic location clustering
-- Transforms raw location_point primitives into meaningful visits
--
-- This table represents clustered location points that indicate a user
-- stayed at a specific location for a meaningful duration (e.g., 5+ minutes).
--
-- Populated by: LocationVisitTransform (density-adaptive HDBSCAN clustering)
-- Source: location_point primitives
-- Processing: Hourly cron job with 12-hour rolling window
-- Idempotency: Deterministic visit IDs based on hash(source_id, centroid, start_time)

SET search_path TO elt, public;

-- ============================================================================
-- LOCATION DOMAIN PRIMITIVES (CONTINUED)
-- ============================================================================

-- location_visit (temporal)
CREATE TABLE IF NOT EXISTS location_visit (
    id UUID PRIMARY KEY,  -- Deterministic UUID (not auto-generated)

    -- Place reference (nullable until place resolution implemented)
    place_id UUID REFERENCES entities_place(id),

    -- Centroid of clustered points (weighted by GPS accuracy)
    centroid_coordinates GEOGRAPHY(POINT) NOT NULL,
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,

    -- Visit temporal bounds
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,

    -- Source tracking (standard ontology pattern)
    source_stream_id UUID NOT NULL,  -- First location_point ID in cluster
    source_table TEXT NOT NULL DEFAULT 'location_point',
    source_provider TEXT NOT NULL DEFAULT 'ios',

    -- Clustering metadata
    metadata JSONB DEFAULT '{}',
    -- Expected metadata fields:
    --   - point_count: Number of location_point records in cluster
    --   - radius_meters: Max distance from centroid to any point
    --   - sampling_rate: Detected points per minute
    --   - cluster_algorithm: "density_adaptive_dbscan"

    -- Standard audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_location_visit_centroid ON location_visit USING GIST(centroid_coordinates);
CREATE INDEX idx_location_visit_start_time ON location_visit(start_time DESC);
CREATE INDEX idx_location_visit_end_time ON location_visit(end_time DESC);
CREATE INDEX idx_location_visit_place ON location_visit(place_id) WHERE place_id IS NOT NULL;
CREATE INDEX idx_location_visit_source ON location_visit(source_stream_id);

-- Timestamp trigger
CREATE TRIGGER location_visit_updated_at
    BEFORE UPDATE ON location_visit
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Table comments
COMMENT ON TABLE location_visit IS
'Clustered location visits representing meaningful stays at specific locations.
Derived from location_point using density-adaptive HDBSCAN clustering.';

COMMENT ON COLUMN location_visit.id IS
'Deterministic UUIDv5 based on hash(source_id, centroid_rounded, start_time_rounded) for idempotent writes.';

COMMENT ON COLUMN location_visit.centroid_coordinates IS
'Weighted centroid of all location points in cluster (weighted by GPS accuracy).';

COMMENT ON COLUMN location_visit.place_id IS
'Reference to canonical place entity. NULL until place resolution is implemented.';

COMMENT ON COLUMN location_visit.start_time IS
'Timestamp of first location point in cluster.';

COMMENT ON COLUMN location_visit.end_time IS
'Timestamp of last location point in cluster. Updated across multiple clustering runs for ongoing visits.';

COMMENT ON COLUMN location_visit.metadata IS
'Clustering metadata: point_count, radius_meters, sampling_rate, cluster_algorithm.';
