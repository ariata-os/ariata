import { tool } from 'ai';
import { z } from 'zod';
import type { Pool } from 'pg';

/**
 * Tool for querying life data from ontology tables
 *
 * Uses simple Zod schema without .describe() chains to avoid serialization issues
 */
export async function createQueryOntologiesTool(pool: Pool) {
	const schema = z.object({
		query: z.string(),
		limit: z.number().optional()
	});

	console.log('[createQueryOntologiesTool] Creating tool with schema');

	return tool({
		description: 'Query life data from ontology tables using SQL. Returns structured data about locations, activities, communications, health metrics, etc.',
		inputSchema: schema,
		execute: async ({ query, limit = 100 }) => {
			console.log('[queryOntologies] ========== EXECUTE FUNCTION CALLED ==========');
			console.log('[queryOntologies] Input query (first 200 chars):', query.slice(0, 200));
			console.log('[queryOntologies] Limit:', limit);
			console.log('[queryOntologies] Pool status:', { connected: !!pool });

			// Validate it's a SELECT query
			if (!query.trim().toLowerCase().startsWith('select')) {
				console.error('[queryOntologies] Invalid query type - not a SELECT statement');
				throw new Error('Only SELECT queries are allowed');
			}

			// Add limit if not present
			const finalQuery = query.includes('LIMIT') ? query : `${query} LIMIT ${Math.min(limit, 1000)}`;
			console.log('[queryOntologies] Final query:', finalQuery);

			try {
				console.log('[queryOntologies] Executing database query...');
				const result = await pool.query(finalQuery);

				console.log('[queryOntologies] Query completed successfully');
				console.log(`[queryOntologies] Returned ${result.rows.length} rows`);
				console.log('[queryOntologies] Row count:', result.rowCount);
				console.log('[queryOntologies] First row sample:', result.rows[0]);

				const resultObject = {
					rows: result.rows,
					rowCount: result.rowCount
				};

				const resultString = JSON.stringify(resultObject);
				console.log('[queryOntologies] Result JSON length:', resultString.length);
				console.log('[queryOntologies] Result preview:', resultString.substring(0, 200) + '...');
				console.log('[queryOntologies] ========== RETURNING RESULT ==========');

				return resultString;
			} catch (error: any) {
				console.error('[queryOntologies] ========== ERROR CAUGHT ==========');
				console.error('[queryOntologies] Error type:', error.constructor.name);
				console.error('[queryOntologies] Error message:', error.message);
				console.error('[queryOntologies] Error stack:', error.stack);
				throw new Error(`Query failed: ${error.message}`);
			}
		}
	});
}
