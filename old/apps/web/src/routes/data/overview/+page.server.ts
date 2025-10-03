import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { sourceConfigs } from '$lib/db/schema/source_configs';
import { sources } from '$lib/db/schema/sources';
import { streamConfigs } from '$lib/db/schema/stream_configs';
import { streams } from '$lib/db/schema/streams';
import { eq, and, desc } from 'drizzle-orm';

export const load: PageServerLoad = async () => {
	try {
		console.log('Loading overview data...');
		
		// Get all source configurations
		const allSourceConfigs = await db
			.select()
			.from(sourceConfigs);
		console.log('Source configs loaded:', allSourceConfigs?.length);

		// Get all connected source instances
		console.log('Loading connected sources...');
		const connectedSources = await db
			.select()
			.from(sources)
			.orderBy(desc(sources.createdAt));
		console.log('Connected sources loaded:', connectedSources?.length);

		// Group connected sources by sourceName for easy lookup
		const connectedByName = new Map<string, typeof connectedSources>();
		for (const source of connectedSources) {
			if (!connectedByName.has(source.sourceName)) {
				connectedByName.set(source.sourceName, []);
			}
			connectedByName.get(source.sourceName)!.push(source);
		}

		// Get all active streams from database
		console.log('Loading streams...');
		const allStreams = await db
			.select()
			.from(streamConfigs);
		console.log('Streams loaded:', allStreams?.length);

		// Get connected stream instances with source and stream config data
		console.log('Loading connected stream instances...');
		const connectedStreamInstances = await db
			.select({
				streamId: streams.id,
				sourceId: streams.sourceId,
				streamConfigId: streams.streamConfigId,
				enabled: streams.enabled,
				lastSyncAt: streams.lastSyncAt,
				lastSyncStatus: streams.lastSyncStatus,
				sourceName: sources.sourceName,
				sourceInstanceName: sources.instanceName,
				sourceStatus: sources.status,
				streamName: streamConfigs.streamName,
				streamDisplayName: streamConfigs.displayName,
				streamIngestionType: streamConfigs.ingestionType
			})
			.from(streams)
			.leftJoin(sources, eq(streams.sourceId, sources.id))
			.leftJoin(streamConfigs, eq(streams.streamConfigId, streamConfigs.id))
			.where(
				and(
					eq(sources.status, 'active'),   // Only active sources
					eq(streams.enabled, true)      // Only enabled streams
				)
			)
			.orderBy(desc(streams.lastSyncAt));
		console.log('Connected stream instances loaded:', connectedStreamInstances?.length);

		console.log('Data loaded:', {
			sources: allSourceConfigs?.length,
			streams: allStreams?.length
		});

		// Process streams from database
		const processedStreams = allStreams.map(stream => {
			return {
				...stream,
				ingestionType: stream.ingestionType || 'pull'
			};
		});

		// Include all configured sources, regardless of whether they have instances
		const processedSources = allSourceConfigs.map(source => {
			const connectedInstances = connectedByName.get(source.name) || [];
			const activeInstances = connectedInstances.filter(i => i.status === 'active');
			
			return {
				...source,
				hasActiveConnection: activeInstances.length > 0,
				connectedInstances: connectedInstances.map(instance => ({
					id: instance.id,
					instanceName: instance.instanceName,
					status: instance.status,
					lastSyncAt: instance.lastSyncAt,
					deviceType: instance.deviceType
				}))
			};
		});

		// Create source lookup map for frontend
		const sourcesByName = new Map(processedSources.map(s => [s.name, s]));

		// Create stream lookup by source
		const streamsBySource = new Map<string, typeof processedStreams>();
		for (const stream of processedStreams) {
			if (!streamsBySource.has(stream.sourceName)) {
				streamsBySource.set(stream.sourceName, []);
			}
			streamsBySource.get(stream.sourceName)!.push(stream);
		}

		return {
			sources: processedSources,
			streams: processedStreams,
			connectedStreamInstances: connectedStreamInstances || [],
			sourcesByName: Object.fromEntries(sourcesByName || []),
			streamsBySource: Object.fromEntries(streamsBySource || []),
			stats: {
				totalSources: allSourceConfigs?.length || 0,  // Source types available
				totalStreams: allStreams?.length || 0,  // Stream types available
				connectedSources: connectedSources?.length || 0,  // Actual connections
				activeStreams: connectedStreamInstances?.length || 0,  // Active stream instances
				streamsByIngestion: {
					pull: processedStreams?.filter(s => s.ingestionType === 'pull')?.length || 0,
					push: processedStreams?.filter(s => s.ingestionType === 'push')?.length || 0,
				}
			}
		};

	} catch (error: any) {
		console.error('Error loading overview data:', error?.message, error);
		return {
			sources: [],
			streams: [],
			connectedStreamInstances: [],
			sourcesByName: {},
			streamsBySource: {},
			stats: {
				totalSources: 0,
				totalStreams: 0,
				connectedSources: 0,
				activeStreams: 0,
				streamsByIngestion: {
					pull: 0,
					push: 0
				}
			}
		};
	}
};