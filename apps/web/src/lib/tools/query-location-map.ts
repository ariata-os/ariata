import { tool } from 'ai';
import { z } from 'zod';
import type { Pool } from 'pg';

/**
 * Tool for querying location data and generating interactive maps
 */
export async function createLocationMapTool(pool: Pool) {
	return tool({
		description: 'Query location data and generate an interactive map visualization. IMPORTANT: Dates must be in YYYY-MM-DD format (e.g., "2025-11-10").',
		parameters: z.object({
			startDate: z.string().optional().describe('Start date in YYYY-MM-DD format (e.g., "2025-11-10")'),
			endDate: z.string().optional().describe('End date in YYYY-MM-DD format (e.g., "2025-11-10")'),
			limit: z.number().optional().describe('Maximum number of points to return (default: 10000)')
		}),
		execute: async ({ startDate, endDate, limit = 10000 }) => {
			console.log('[queryLocationMap] ========== EXECUTE FUNCTION CALLED ==========');
			console.log('[queryLocationMap] Input params:', { startDate, endDate, limit });
			console.log('[queryLocationMap] Pool status:', { connected: !!pool });

			// Validate date format
			const dateRegex = /^\d{4}-\d{2}-\d{2}$/;
			if (startDate && !dateRegex.test(startDate)) {
				console.error('[queryLocationMap] Invalid startDate format:', startDate, '- expected YYYY-MM-DD');
				throw new Error(`Invalid startDate format: "${startDate}". Expected YYYY-MM-DD format (e.g., "2025-11-10")`);
			}
			if (endDate && !dateRegex.test(endDate)) {
				console.error('[queryLocationMap] Invalid endDate format:', endDate, '- expected YYYY-MM-DD');
				throw new Error(`Invalid endDate format: "${endDate}". Expected YYYY-MM-DD format (e.g., "2025-11-10")`);
			}

			try {
				// Query location data from elt.location_point table
				// Use systematic sampling (every 4th point) to evenly distribute points across the day
				const params: any[] = [];
				let paramIndex = 1;

				let whereConditions = '1=1';

				if (startDate) {
					whereConditions += ` AND DATE(timestamp) >= $${paramIndex}::date`;
					params.push(startDate);
					paramIndex++;
				}

				if (endDate) {
					whereConditions += ` AND DATE(timestamp) <= $${paramIndex}::date`;
					params.push(endDate);
					paramIndex++;
				}

				const query = `
					WITH numbered_points AS (
						SELECT
							timestamp,
							latitude,
							longitude,
							altitude_meters as altitude,
							accuracy_meters as horizontal_accuracy,
							speed_meters_per_second as speed,
							course_degrees as course,
							ROW_NUMBER() OVER (ORDER BY timestamp) as rn
						FROM elt.location_point
						WHERE ${whereConditions}
					)
					SELECT
						timestamp,
						latitude,
						longitude,
						altitude,
						horizontal_accuracy,
						speed,
						course
					FROM numbered_points
					WHERE rn % 4 = 0
					ORDER BY timestamp DESC
					LIMIT $${paramIndex}
				`;
				params.push(Math.min(limit, 10000));

				console.log('[queryLocationMap] Executing query:', query);
				console.log('[queryLocationMap] Query params:', params);

				let result = await pool.query(query, params);

				// Fallback: if sampling returns 0 results, use simple LIMIT query
				if (result.rows.length === 0) {
					console.warn('[queryLocationMap] Sampling returned 0 results, falling back to simple LIMIT query');

					const fallbackParams: any[] = [];
					let fallbackParamIndex = 1;
					let fallbackQuery = `
						SELECT
							timestamp,
							latitude,
							longitude,
							altitude_meters as altitude,
							accuracy_meters as horizontal_accuracy,
							speed_meters_per_second as speed,
							course_degrees as course
						FROM elt.location_point
						WHERE 1=1
					`;

					if (startDate) {
						fallbackQuery += ` AND DATE(timestamp) >= $${fallbackParamIndex}::date`;
						fallbackParams.push(startDate);
						fallbackParamIndex++;
					}

					if (endDate) {
						fallbackQuery += ` AND DATE(timestamp) <= $${fallbackParamIndex}::date`;
						fallbackParams.push(endDate);
						fallbackParamIndex++;
					}

					fallbackQuery += ` ORDER BY timestamp DESC LIMIT $${fallbackParamIndex}`;
					fallbackParams.push(Math.min(limit, 5000)); // Use 5000 for fallback

					console.log('[queryLocationMap] Fallback query:', fallbackQuery);
					result = await pool.query(fallbackQuery, fallbackParams);
					console.log(`[queryLocationMap] Fallback returned ${result.rows.length} location points`);
				}

				console.log('[queryLocationMap] Query completed successfully');
				console.log(`[queryLocationMap] Returned ${result.rows.length} location points`);
				console.log('[queryLocationMap] First row sample:', result.rows[0]);

				// Transform to MapVisualization format
				const points = result.rows.map((row) => ({
					latitude: row.latitude,
					longitude: row.longitude,
					timestamp: row.timestamp,
					accuracy_meters: row.horizontal_accuracy,
					speed_meters_per_second: row.speed
				}));

				// Calculate bounds
				let bounds = null;
				if (points.length > 0) {
					const lats = points.map(p => p.latitude);
					const lons = points.map(p => p.longitude);
					bounds = {
						minLat: Math.min(...lats),
						maxLat: Math.max(...lats),
						minLon: Math.min(...lons),
						maxLon: Math.max(...lons)
					};
				}

				// Get time range from actual data
				const timeRange = {
					startTime: points.length > 0 ? points[points.length - 1].timestamp : (startDate || ''),
					endTime: points.length > 0 ? points[0].timestamp : (endDate || '')
				};

				// Wrap result in expected format for frontend MapVisualization component
				const wrappedResult = {
					success: true,
					type: 'map_visualization',
					data: {
						points: points,
						visits: [],
						bounds: bounds,
						metadata: {
							startTime: timeRange.startTime,
							endTime: timeRange.endTime,
							pointCount: points.length,
							visitCount: 0
						}
					}
				};

				const resultString = JSON.stringify(wrappedResult);
				console.log('[queryLocationMap] Wrapped result created, length:', resultString.length);
				console.log('[queryLocationMap] Point count:', result.rows.length);
				console.log('[queryLocationMap] ========== RETURNING RESULT ==========');

				return resultString;
			} catch (error: any) {
				console.error('[queryLocationMap] ========== ERROR CAUGHT ==========');
				console.error('[queryLocationMap] Error type:', error.constructor.name);
				console.error('[queryLocationMap] Error message:', error.message);
				console.error('[queryLocationMap] Error stack:', error.stack);
				throw new Error(`Location query failed: ${error.message}`);
			}
		}
	});
}
