import { z } from 'zod';
import type { Pool } from 'pg';

/**
 * Tool for querying temporal pursuits (tasks, initiatives, aspirations)
 * Returns a plain object compatible with AI SDK v6 tool format
 */
export async function createPursuitsTool(pool: Pool) {
	const inputSchema = z.object({
		type: z
			.enum(['task', 'initiative', 'aspiration', 'all'])
			.optional()
			.describe('Filter by pursuit type (default: "all")'),
		status: z.string().optional().describe('Filter by status (e.g., "active", "completed")'),
		tags: z.array(z.string()).optional().describe('Filter by tags (pursuits with any of these tags)'),
		limit: z.number().optional().describe('Maximum number of pursuits to return (default: 100)')
	});

	return {
		description:
			'Query temporal pursuits including tasks (daily/weekly), initiatives (monthly/quarterly), and aspirations (multi-year goals). Can filter by type, status, or tags.',
		inputSchema,
		execute: async ({
			type = 'all',
			status,
			tags,
			limit = 100
		}: z.infer<typeof inputSchema>) => {
			console.log('[queryPursuits] Executing with params:', { type, status, tags, limit });

			try {
				const pursuits: any[] = [];
				let taskCount = 0;
				let initiativeCount = 0;
				let aspirationCount = 0;

				// Helper to build WHERE clause
				const buildWhereClause = (paramIndex: number, params: any[]) => {
					let whereConditions = 'is_active = true';

					if (status) {
						whereConditions += ` AND status = $${paramIndex}`;
						params.push(status);
						paramIndex++;
					}

					if (tags && tags.length > 0) {
						// PostgreSQL array overlap operator: tags && ARRAY['tag1', 'tag2']
						whereConditions += ` AND tags && $${paramIndex}::text[]`;
						params.push(tags);
						paramIndex++;
					}

					return { whereConditions, paramIndex };
				};

				// Query tasks if requested
				if (type === 'all' || type === 'task') {
					const params: any[] = [];
					const { whereConditions, paramIndex } = buildWhereClause(1, params);

					const taskQuery = `
						SELECT
							id,
							title,
							description,
							tags,
							topic_id,
							status,
							progress_percent,
							start_date,
							target_date,
							completed_date,
							is_active,
							created_at,
							updated_at
						FROM data.praxis_task
						WHERE ${whereConditions}
						ORDER BY created_at DESC
						LIMIT $${paramIndex}
					`;
					params.push(limit);

					console.log('[queryPursuits] Executing task query:', taskQuery);
					const taskResult = await pool.query(taskQuery, params);
					console.log(`[queryPursuits] Found ${taskResult.rows.length} tasks`);

					taskCount = taskResult.rows.length;
					pursuits.push(
						...taskResult.rows.map((row) => ({
							type: 'task',
							data: {
								id: row.id,
								title: row.title,
								description: row.description,
								tags: row.tags,
								topic_id: row.topic_id,
								status: row.status,
								progress_percent: row.progress_percent,
								start_date: row.start_date,
								target_date: row.target_date,
								completed_date: row.completed_date,
								is_active: row.is_active,
								created_at: row.created_at,
								updated_at: row.updated_at
							}
						}))
					);
				}

				// Query initiatives if requested
				if (type === 'all' || type === 'initiative') {
					const params: any[] = [];
					const { whereConditions, paramIndex } = buildWhereClause(1, params);

					const initiativeQuery = `
						SELECT
							id,
							title,
							description,
							tags,
							topic_id,
							status,
							progress_percent,
							start_date,
							target_date,
							completed_date,
							is_active,
							created_at,
							updated_at
						FROM data.praxis_initiative
						WHERE ${whereConditions}
						ORDER BY created_at DESC
						LIMIT $${paramIndex}
					`;
					params.push(limit);

					console.log('[queryPursuits] Executing initiative query:', initiativeQuery);
					try {
						const initiativeResult = await pool.query(initiativeQuery, params);
						console.log(`[queryPursuits] Found ${initiativeResult.rows.length} initiatives`);

						initiativeCount = initiativeResult.rows.length;
						pursuits.push(
							...initiativeResult.rows.map((row) => ({
								type: 'initiative',
								data: {
									id: row.id,
									title: row.title,
									description: row.description,
									tags: row.tags,
									topic_id: row.topic_id,
									status: row.status,
									progress_percent: row.progress_percent,
									start_date: row.start_date,
									target_date: row.target_date,
									completed_date: row.completed_date,
									is_active: row.is_active,
									created_at: row.created_at,
									updated_at: row.updated_at
								}
							}))
						);
					} catch (error: any) {
						// Table might not exist yet - log warning but continue
						console.warn('[queryPursuits] Initiative table not found or query failed:', error.message);
					}
				}

				// Query aspirations if requested
				if (type === 'all' || type === 'aspiration') {
					const params: any[] = [];
					const { whereConditions, paramIndex } = buildWhereClause(1, params);

					const aspirationQuery = `
						SELECT
							id,
							title,
							description,
							tags,
							topic_id,
							status,
							target_timeframe,
							achieved_date,
							is_active,
							created_at,
							updated_at
						FROM data.praxis_aspiration
						WHERE ${whereConditions}
						ORDER BY created_at DESC
						LIMIT $${paramIndex}
					`;
					params.push(limit);

					console.log('[queryPursuits] Executing aspiration query:', aspirationQuery);
					try {
						const aspirationResult = await pool.query(aspirationQuery, params);
						console.log(`[queryPursuits] Found ${aspirationResult.rows.length} aspirations`);

						aspirationCount = aspirationResult.rows.length;
						pursuits.push(
							...aspirationResult.rows.map((row) => ({
								type: 'aspiration',
								data: {
									id: row.id,
									title: row.title,
									description: row.description,
									tags: row.tags,
									topic_id: row.topic_id,
									status: row.status,
									target_timeframe: row.target_timeframe,
									achieved_date: row.achieved_date,
									is_active: row.is_active,
									created_at: row.created_at,
									updated_at: row.updated_at
								}
							}))
						);
					} catch (error: any) {
						// Table might not exist yet - log warning but continue
						console.warn('[queryPursuits] Aspiration table not found or query failed:', error.message);
					}
				}

				// Sort all pursuits by created_at (most recent first)
				pursuits.sort((a, b) => {
					const dateA = new Date(a.data.created_at).getTime();
					const dateB = new Date(b.data.created_at).getTime();
					return dateB - dateA;
				});

				// Limit total results
				const limitedPursuits = pursuits.slice(0, limit);

				// Wrap result in expected format for frontend PursuitsWidget component
				const wrappedResult = {
					success: true,
					type: 'pursuits_widget',
					data: {
						pursuits: limitedPursuits,
						metadata: {
							totalCount: limitedPursuits.length,
							taskCount,
							initiativeCount,
							aspirationCount
						}
					}
				};

				console.log(
					`[queryPursuits] Returning ${limitedPursuits.length} total pursuits (${taskCount} tasks, ${initiativeCount} initiatives, ${aspirationCount} aspirations)`
				);

				// Return object directly (AI SDK v6 expects objects, not strings)
				return wrappedResult;
			} catch (error: any) {
				console.error('[queryPursuits] ========== ERROR CAUGHT ==========');
				console.error('[queryPursuits] Error type:', error.constructor.name);
				console.error('[queryPursuits] Error message:', error.message);
				console.error('[queryPursuits] Error stack:', error.stack);
				throw new Error(`Pursuits query failed: ${error.message}`);
			}
		}
	};
}
