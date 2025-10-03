import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { pipelineActivities, sourceConfigs, streamConfigs } from '$lib/db/schema';
import { desc, eq, sql } from 'drizzle-orm';

export const load: PageServerLoad = async ({ url }) => {
	try {
		const limit = 50; // Show more activities since we're combining all types

		// Get all recent activities in one query, ordered by time
		const allActivities = await db
			.select()
			.from(pipelineActivities)
			.leftJoin(streamConfigs, eq(pipelineActivities.streamId, streamConfigs.id))
			.leftJoin(sourceConfigs, eq(pipelineActivities.sourceName, sourceConfigs.name))
			.orderBy(desc(pipelineActivities.startedAt))
			.limit(limit);

		// Get comprehensive statistics for today
		const [stats] = await db
			.select({
				// Overall stats
				activeTotal: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.status} = 'running' THEN 1 ELSE 0 END), 0)::int`,
				completedToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.status} = 'completed' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				failedToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.status} = 'failed' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				
				// Ingestion specific
				ingestionsToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.activityType} = 'ingestion' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				dataVolumeToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.activityType} = 'ingestion' AND ${pipelineActivities.status} = 'completed' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN ${pipelineActivities.dataSizeBytes} ELSE 0 END), 0)::bigint`,
				
				// Signal creation specific
				signalsCreatedToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.activityType} = 'signal_creation' AND ${pipelineActivities.status} = 'completed' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				
				// Background tasks specific
				tokenRefreshesToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.activityType} = 'token_refresh' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				scheduledChecksToday: sql<number>`COALESCE(SUM(CASE WHEN ${pipelineActivities.activityType} = 'scheduled_check' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0)::int`,
				
				// Success rate
				successRate: sql<number>`
					COALESCE(
						ROUND(
							100.0 * SUM(CASE WHEN ${pipelineActivities.status} = 'completed' AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END) / 
							NULLIF(SUM(CASE WHEN ${pipelineActivities.status} IN ('completed', 'failed') AND DATE(${pipelineActivities.completedAt}) = CURRENT_DATE THEN 1 ELSE 0 END), 0),
							1
						),
						0
					)
				`
			})
			.from(pipelineActivities);

		// Map all activities with their related data
		const mappedActivities = allActivities.map(row => ({
			...row.pipeline_activities,
			// Stream data
			streamName: row.stream_configs?.streamName,
			streamDisplayName: row.stream_configs?.displayName,
			// Signal data
			signalName: row.signal_configs?.signalName,
			signalDisplayName: row.signal_configs?.displayName,
			signalType: row.signal_configs?.computation?.value_type || 'continuous',
			// Source data
			sourceDisplayName: row.source_configs?.displayName
		}));
		
		return {
			activities: mappedActivities,
			stats: stats || {
				activeTotal: 0,
				completedToday: 0,
				failedToday: 0,
				ingestionsToday: 0,
				dataVolumeToday: 0,
				signalsCreatedToday: 0,
				tokenRefreshesToday: 0,
				scheduledChecksToday: 0,
				successRate: 0
			}
		};
	} catch (error) {
		console.error('Error loading pipeline data:', error);
		return {
			activities: [],
			stats: {
				activeTotal: 0,
				completedToday: 0,
				failedToday: 0,
				ingestionsToday: 0,
				dataVolumeToday: 0,
				signalsCreatedToday: 0,
				tokenRefreshesToday: 0,
				scheduledChecksToday: 0,
				successRate: 0
			}
		};
	}
};