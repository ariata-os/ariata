# Ariata TODOs

## High Priority

### Location Visit Transformation (Spatial Clustering)

**Status**: Schema exists, transformation not implemented

**Problem**:
- We currently transform `stream_ios_location` → `location_point` (raw GPS pings)
- We need to transform `location_point` → `location_visit` (meaningful place visits)
- The `location_visit` table exists in the schema but is never populated from real data
- Narrative seeding and changepoint detection expect `location_visit` to exist

**What's Needed**:

1. **Implement HDBSCAN Clustering Algorithm**
   - Use [HDBSCAN](https://hdbscan.readthedocs.io/) for density-based spatial clustering
   - Better than DBSCAN for varying density clusters (home vs. brief stops)
   - Rust implementation: Consider `linfa-clustering` crate or Python bridge

2. **Create Transform: `LocationVisitTransform`**
   - Input: `location_point` primitives (raw GPS coordinates)
   - Output: `location_visit` primitives (clustered stays)
   - Location: `core/src/sources/ios/location/visit_transform.rs` (new file)

3. **Algorithm Parameters**:
   ```rust
   // Suggested starting values (tune based on real data)
   min_cluster_size: 5 points         // Minimum points to form a visit
   min_samples: 3 points              // Core point threshold
   distance_threshold: 100 meters     // Max distance within a cluster
   time_gap_threshold: 5 minutes      // Max time gap within a visit
   min_duration: 5 minutes            // Minimum visit duration to keep
   ```

4. **Transform Logic**:
   ```
   For each user:
     1. Fetch location_points ordered by timestamp
     2. Run HDBSCAN clustering on (lat, lon, time)
     3. For each cluster:
        - Calculate centroid coordinates
        - Determine start_time (earliest point) and end_time (latest point)
        - Filter out visits < min_duration
        - Optionally resolve place_id via entities_place lookup
     4. Batch insert to location_visit table
   ```

5. **Update Transform Registry**:
   - File: `core/src/transforms/registry.rs`
   - Change `"stream_ios_location"` target_tables from `vec!["location_point"]`
     to `vec!["location_point", "location_visit"]`

6. **Job/Trigger Configuration**:
   - Option A: Run as secondary transform after `location_point` transform completes
   - Option B: Run as periodic batch job (every hour/day) on accumulated points
   - Needs checkpoint tracking to avoid reprocessing

**References**:
- Schema definition: `core/migrations/003_ontologies.sql` lines 245-252
- Documentation: `ONTOLOGIES.md` lines 246-252
- Changepoint detection usage: `CHANGEPOINT.md` lines 119-121, 261-262
- Current location_point transform: `core/src/sources/ios/location/transform.rs`

**Related Work**:
- Consider implementing `entities_place` clustering/resolution
- Add visualization of visits in map tool
- Integrate with narrative layer for place-based storytelling

---

## Additional TODOs

*(Add other TODO items here as needed)*
