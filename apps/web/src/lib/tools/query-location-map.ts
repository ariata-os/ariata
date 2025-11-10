import { tool } from 'ai';
import { z } from 'zod';
import type { Pool } from 'pg';

interface LocationPoint {
	latitude: number;
	longitude: number;
	timestamp: string;
	accuracy_meters?: number;
	speed_meters_per_second?: number;
}

interface LocationVisit {
	latitude: number;
	longitude: number;
	start_time: string;
	end_time: string;
	canonical_name?: string;
	category?: string;
	duration_minutes: number;
}

interface MapBounds {
	minLat: number;
	maxLat: number;
	minLon: number;
	maxLon: number;
}

interface MapData {
	points: LocationPoint[];
	visits: LocationVisit[];
	bounds: MapBounds | null;
	metadata: {
		startTime: string;
		endTime: string;
		pointCount: number;
		visitCount: number;
	};
}

/**
 * Calculate bounding box for all location data
 */
function calculateBounds(
	points: LocationPoint[],
	visits: LocationVisit[]
): MapBounds | null {
	const allLats = [
		...points.map((p) => p.latitude),
		...visits.map((v) => v.latitude)
	];
	const allLons = [
		...points.map((p) => p.longitude),
		...visits.map((v) => v.longitude)
	];

	if (allLats.length === 0) return null;

	return {
		minLat: Math.min(...allLats),
		maxLat: Math.max(...allLats),
		minLon: Math.min(...allLons),
		maxLon: Math.max(...allLons)
	};
}

/**
 * Create a location map visualization tool
 *
 * This tool queries location data from the database and returns it in a format
 * optimized for map rendering. It supports querying both raw location points
 * and aggregated place visits within a specified time range.
 */
export async function createLocationMapTool(pool: Pool) {
	return tool({
		description: `PREFERRED TOOL for visualizing location data on an interactive map.

Use this tool DIRECTLY when the user asks to:
- See where they've been
- View their location history or movement patterns
- Show places they've visited
- Display their travels or journeys
- "show me on a map"
- Visualize geographic activity

DO NOT use queryOntologies first - this tool handles querying automatically.
The tool will return an error message if no data exists for the time range.

Returns location data formatted for interactive map rendering:
- Raw location points (GPS traces shown as a path)
- Place visits with names and durations (shown as markers)
- Automatic map bounds for optimal viewing

This provides a visual, geographic view rather than tabular data.`,

		inputSchema: z.object({
			startTime: z
				.string()
				.describe(
					'Start of time range in ISO 8601 format (e.g., "2024-01-01T00:00:00Z"). Required.'
				),
			endTime: z
				.string()
				.describe(
					'End of time range in ISO 8601 format (e.g., "2024-01-31T23:59:59Z"). Required.'
				),
			includePoints: z
				.boolean()
				.default(true)
				.describe(
					'Include raw GPS location points (shown as a path). Default: true. Set false for cleaner visit-only view.'
				),
			includeVisits: z
				.boolean()
				.default(true)
				.describe(
					'Include place visits with names and durations (shown as markers). Default: true.'
				),
			maxPoints: z
				.number()
				.default(1000)
				.describe('Maximum number of location points to return. Default: 1000.')
		}),

		execute: async ({ startTime, endTime, includePoints, includeVisits, maxPoints }) => {
			console.log('[locationMapTool] Querying location data:', {
				startTime,
				endTime,
				includePoints,
				includeVisits,
				maxPoints
			});

			const mapData: MapData = {
				points: [],
				visits: [],
				bounds: null,
				metadata: {
					startTime,
					endTime,
					pointCount: 0,
					visitCount: 0
				}
			};

			try {
				const client = await pool.connect();
				try {
					await client.query('BEGIN TRANSACTION READ ONLY');
					await client.query('SET search_path TO elt, public');

					// Query location points
					if (includePoints) {
						const pointsQuery = `
							SELECT
								latitude,
								longitude,
								timestamp,
								accuracy_meters,
								speed_meters_per_second
							FROM location_point
							WHERE timestamp >= $1 AND timestamp <= $2
							ORDER BY timestamp ASC
							LIMIT $3
						`;

						const pointsResult = await client.query(pointsQuery, [
							startTime,
							endTime,
							maxPoints
						]);

						mapData.points = pointsResult.rows.map((row) => ({
							latitude: row.latitude,
							longitude: row.longitude,
							timestamp: row.timestamp,
							accuracy_meters: row.accuracy_meters,
							speed_meters_per_second: row.speed_meters_per_second
						}));

						console.log(`[locationMapTool] Found ${mapData.points.length} location points`);
					}

					// Query location visits with place information
					if (includeVisits) {
						const visitsQuery = `
							SELECT
								lv.latitude,
								lv.longitude,
								lv.start_time,
								lv.end_time,
								ep.canonical_name,
								ep.category,
								EXTRACT(EPOCH FROM (lv.end_time - lv.start_time)) / 60 as duration_minutes
							FROM location_visit lv
							LEFT JOIN entities_place ep ON lv.place_id = ep.id
							WHERE lv.start_time >= $1 AND lv.end_time <= $2
							ORDER BY lv.start_time ASC
							LIMIT 200
						`;

						const visitsResult = await client.query(visitsQuery, [startTime, endTime]);

						mapData.visits = visitsResult.rows.map((row) => ({
							latitude: row.latitude,
							longitude: row.longitude,
							start_time: row.start_time,
							end_time: row.end_time,
							canonical_name: row.canonical_name,
							category: row.category,
							duration_minutes: Math.round(row.duration_minutes)
						}));

						console.log(`[locationMapTool] Found ${mapData.visits.length} place visits`);
					}

					await client.query('COMMIT');

					// Calculate bounds
					mapData.bounds = calculateBounds(mapData.points, mapData.visits);
					mapData.metadata.pointCount = mapData.points.length;
					mapData.metadata.visitCount = mapData.visits.length;

					// Check if we have any data
					if (mapData.points.length === 0 && mapData.visits.length === 0) {
						return {
							success: false,
							error: `No location data found between ${startTime} and ${endTime}. The user may not have any location tracking data for this time period.`
						};
					}

					console.log('[locationMapTool] Successfully prepared map data:', {
						pointCount: mapData.metadata.pointCount,
						visitCount: mapData.metadata.visitCount,
						bounds: mapData.bounds
					});

					return {
						success: true,
						type: 'map_visualization',
						data: mapData
					};
				} catch (error) {
					await client.query('ROLLBACK');
					throw error;
				} finally {
					client.release();
				}
			} catch (error) {
				console.error('[locationMapTool] Error:', error);
				return {
					success: false,
					error: error instanceof Error ? error.message : 'Unknown database error'
				};
			}
		}
	});
}
