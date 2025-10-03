import { type RequestHandler } from '@sveltejs/kit';
import { json } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { streamConfigs, sources, streams } from '$lib/db/schema';
import { eq, and } from 'drizzle-orm';

export const GET: RequestHandler = async ({ url }) => {
	try {
		const sourceId = url.searchParams.get('sourceId');
		const sourceName = url.searchParams.get('source');

		// If sourceId is provided, get the actual stream instances
		if (sourceId) {
			const streamInstances = await db
				.select({
					id: streams.id,
					sourceId: streams.sourceId,
					streamConfigId: streams.streamConfigId,
					enabled: streams.enabled,
					syncSchedule: streams.syncSchedule,
					initialSyncType: streams.initialSyncType,
					initialSyncDays: streams.initialSyncDays,
					initialSyncDaysFuture: streams.initialSyncDaysFuture,
					settings: streams.settings,
					lastSyncAt: streams.lastSyncAt,
					lastSyncStatus: streams.lastSyncStatus,
					// Include stream config details
					streamName: streamConfigs.streamName,
					displayName: streamConfigs.displayName,
					description: streamConfigs.description,
					ingestionType: streamConfigs.ingestionType,
					defaultCronSchedule: streamConfigs.cronSchedule,
				})
				.from(streams)
				.innerJoin(streamConfigs, eq(streams.streamConfigId, streamConfigs.id))
				.where(eq(streams.sourceId, sourceId));

			return json(streamInstances);
		}

		// Otherwise, get available stream configs for a source type
		if (sourceName) {
			const configs = await db
				.select()
				.from(streamConfigs)
				.where(eq(streamConfigs.sourceName, sourceName));

			return json(configs);
		}

		// Return all stream configs if no filters
		const allConfigs = await db.select().from(streamConfigs);
		return json(allConfigs);
	} catch (error) {
		console.error('Failed to fetch streams:', error);
		return json({ error: 'Failed to fetch streams' }, { status: 500 });
	}
};

export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { sourceName, sourceId, streamConfigs: streamSettings, instanceName, description } = body;

		// Validate required fields
		if (!sourceName || !sourceId) {
			return json({ error: 'Source name and source ID are required' }, { status: 400 });
		}

		const targetSourceId = sourceId;

		// Verify the source exists
		const [existingSource] = await db
			.select()
			.from(sources)
			.where(eq(sources.id, targetSourceId))
			.limit(1);

		if (!existingSource) {
			return json({ 
				error: 'Source not found. Please ensure the source exists.' 
			}, { status: 404 });
		}

		// Get all stream configs for this source type
		const availableStreamConfigs = await db
			.select()
			.from(streamConfigs)
			.where(eq(streamConfigs.sourceName, sourceName));

		// Create a map for easy lookup
		const streamConfigMap = new Map(
			availableStreamConfigs.map(sc => [sc.streamName, sc])
		);

		// Begin transaction to update streams table
		const createdStreams = [];
		
		// First, delete existing stream entries for this source (we'll recreate them)
		await db.delete(streams).where(eq(streams.sourceId, targetSourceId));

		// Create new stream entries for enabled streams
		for (const streamSetting of streamSettings) {
			const streamConfig = streamConfigMap.get(streamSetting.streamName);
			
			if (!streamConfig) {
				console.warn(`Stream config not found for: ${streamSetting.streamName}`);
				continue;
			}

			if (streamSetting.enabled) {
				const [newStream] = await db
					.insert(streams)
					.values({
						sourceId: targetSourceId,
						streamConfigId: streamConfig.id,
						enabled: true,
						syncSchedule: streamSetting.syncSchedule || streamConfig.cronSchedule,
						initialSyncType: streamSetting.initialSyncType || 'limited',
						initialSyncDays: streamSetting.initialSyncDays || 90,
						initialSyncDaysFuture: streamSetting.initialSyncDaysFuture || 30,
						settings: streamSetting.settings || {},
					})
					.returning();
				
				createdStreams.push(newStream);
			}
		}

		// Update the source to mark it as active and update name/description if provided
		const updateData: any = {
			status: 'active',  // Set status to active when streams are configured
			isActive: true,
			updatedAt: new Date(),
			// Still store a summary in metadata for backward compatibility
			sourceMetadata: {
				...(await db.select().from(sources).where(eq(sources.id, targetSourceId)).limit(1))[0].sourceMetadata as any || {},
				configuredAt: new Date().toISOString(),
				enabledStreams: createdStreams.length
			}
		};

		// Update instanceName and description if provided
		if (instanceName) {
			updateData.instanceName = instanceName;
		}
		if (description) {
			updateData.description = description;
		}

		await db
			.update(sources)
			.set(updateData)
			.where(eq(sources.id, targetSourceId));

		// Log for debugging
		console.log(`Created ${createdStreams.length} stream entries for source ${targetSourceId}`);
		if (createdStreams.length > 0) {
			console.log('Enabled streams:', createdStreams.map(s => s.id).join(', '));
		}

		return json({
			success: true,
			sourceId: targetSourceId,
			message: 'Stream configuration saved successfully',
			configuredStreams: createdStreams.length,
			streamIds: createdStreams.map(s => s.id)
		});
	} catch (error) {
		console.error('Failed to save stream configuration:', error);
		return json({ 
			error: 'Failed to save stream configuration',
			details: error instanceof Error ? error.message : 'Unknown error'
		}, { status: 500 });
	}
};

export const PUT: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { streamId, enabled, syncSchedule, settings } = body;

		if (!streamId) {
			return json({ error: 'Stream ID is required' }, { status: 400 });
		}

		// Update stream instance configuration
		const updateData: any = {
			updatedAt: new Date()
		};

		if (enabled !== undefined) updateData.enabled = enabled;
		if (syncSchedule !== undefined) updateData.syncSchedule = syncSchedule;
		if (settings !== undefined) updateData.settings = settings;

		const [updatedStream] = await db
			.update(streams)
			.set(updateData)
			.where(eq(streams.id, streamId))
			.returning();

		if (!updatedStream) {
			return json({ error: 'Stream instance not found' }, { status: 404 });
		}

		return json({
			success: true,
			stream: updatedStream,
			message: 'Stream updated successfully'
		});
	} catch (error) {
		console.error('Failed to update stream:', error);
		return json({ error: 'Failed to update stream' }, { status: 500 });
	}
};

export const DELETE: RequestHandler = async ({ url }) => {
	try {
		const streamId = url.searchParams.get('id');

		if (!streamId) {
			return json({ error: 'Stream ID is required' }, { status: 400 });
		}

		// Delete the stream instance (or soft delete by marking as disabled)
		const [deletedStream] = await db
			.update(streams)
			.set({
				enabled: false,
				updatedAt: new Date()
			})
			.where(eq(streams.id, streamId))
			.returning();

		if (!deletedStream) {
			return json({ error: 'Stream instance not found' }, { status: 404 });
		}

		return json({
			success: true,
			message: 'Stream disabled successfully'
		});
	} catch (error) {
		console.error('Failed to delete stream:', error);
		return json({ error: 'Failed to delete stream' }, { status: 500 });
	}
};