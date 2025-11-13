import { tool } from 'ai';
import { z } from 'zod';
import type { Pool } from 'pg';

/**
 * Tool for listing available tables in the elt schema
 *
 * Queries the database information schema to show available tables
 * organized by category (location, health, social, etc.)
 */
export async function createListTablesTool(pool: Pool) {
	return tool({
		description: 'List all available data tables in the database with their columns and types. Helps discover what life data is available to query.',
		inputSchema: z.object({
			category: z.string().optional().describe('Optional filter by category: location, health, social, activity, knowledge, entities, axiology, narrative, or ambient')
		}),
		execute: async ({ category }) => {
			console.log('[listTables] ========== EXECUTE FUNCTION CALLED ==========');
			console.log('[listTables] Category filter:', category || 'all');

			try {
				// Query information schema to get all tables in elt schema with their columns
				const query = `
					SELECT
						t.table_name,
						array_agg(
							c.column_name || ': ' || c.data_type
							ORDER BY c.ordinal_position
						) as columns
					FROM information_schema.tables t
					LEFT JOIN information_schema.columns c
						ON t.table_name = c.table_name
						AND t.table_schema = c.table_schema
					WHERE t.table_schema = 'elt'
						AND t.table_type = 'BASE TABLE'
						AND t.table_name NOT LIKE '\\_%'  -- Exclude internal tables like _sqlx_migrations
					GROUP BY t.table_name
					ORDER BY t.table_name
				`;

				console.log('[listTables] Executing query:', query);
				const result = await pool.query(query);

				console.log(`[listTables] Found ${result.rows.length} tables`);

				// Organize tables by category
				const categorized: Record<string, any[]> = {
					location: [],
					health: [],
					social: [],
					activity: [],
					knowledge: [],
					entities: [],
					axiology: [],
					narrative: [],
					ambient: [],
					other: []
				};

				for (const row of result.rows) {
					const tableName = row.table_name;
					const tableInfo = {
						name: `elt.${tableName}`,
						columns: row.columns
					};

					// Categorize based on prefix
					if (tableName.startsWith('location_')) {
						categorized.location.push(tableInfo);
					} else if (tableName.startsWith('health_')) {
						categorized.health.push(tableInfo);
					} else if (tableName.startsWith('social_')) {
						categorized.social.push(tableInfo);
					} else if (tableName.startsWith('activity_')) {
						categorized.activity.push(tableInfo);
					} else if (tableName.startsWith('knowledge_')) {
						categorized.knowledge.push(tableInfo);
					} else if (tableName.startsWith('entities_')) {
						categorized.entities.push(tableInfo);
					} else if (tableName.startsWith('axiology_')) {
						categorized.axiology.push(tableInfo);
					} else if (tableName.startsWith('narrative_')) {
						categorized.narrative.push(tableInfo);
					} else if (tableName.startsWith('ambient_')) {
						categorized.ambient.push(tableInfo);
					} else {
						categorized.other.push(tableInfo);
					}
				}

				// Filter by category if specified
				let resultData = categorized;
				if (category) {
					const cat = category.toLowerCase();
					if (categorized[cat]) {
						resultData = { [cat]: categorized[cat] };
					}
				}

				// Remove empty categories
				const filtered = Object.fromEntries(
					Object.entries(resultData).filter(([_, tables]) => tables.length > 0)
				);

				const resultString = JSON.stringify({
					schema: 'elt',
					totalTables: result.rows.length,
					categories: filtered
				}, null, 2);

				console.log('[listTables] Result length:', resultString.length);
				console.log('[listTables] Categories:', Object.keys(filtered));
				console.log('[listTables] ========== RETURNING RESULT ==========');

				return resultString;
			} catch (error: any) {
				console.error('[listTables] ========== ERROR CAUGHT ==========');
				console.error('[listTables] Error type:', error.constructor.name);
				console.error('[listTables] Error message:', error.message);
				console.error('[listTables] Error stack:', error.stack);
				throw new Error(`List tables failed: ${error.message}`);
			}
		}
	});
}
