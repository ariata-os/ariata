import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { pipelineActivities, sources, streamConfigs, streams } from '$lib/db/schema';
import { eq, and } from 'drizzle-orm';
import { queueCeleryTask } from '$lib/redis';

function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 Bytes';
	const k = 1024;
	const sizes = ['Bytes', 'KB', 'MB', 'GB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export const POST: RequestHandler = async ({ request }) => {
	const deviceToken = request.headers.get('x-device-token');

	if (!deviceToken) {
		return json({ error: 'Device token required' }, { status: 401 });
	}

	try {
		// Normalize token to uppercase for case-insensitive comparison
		const normalizedToken = deviceToken.toUpperCase();
		
		// Find device source instance by token (case-insensitive)
		const [deviceSource] = await db
			.select()
			.from(sources)
			.where(eq(sources.deviceToken, normalizedToken))
			.limit(1);

		if (!deviceSource) {
			return json({ error: 'Invalid device token' }, { status: 401 });
		}

		// Update last sync time
		await db
			.update(sources)
			.set({ lastSyncAt: new Date() })
			.where(eq(sources.id, deviceSource.id));

		// Parse request body
		const contentType = request.headers.get('content-type');
		const contentEncoding = request.headers.get('content-encoding');
		let body: any;

		if (contentEncoding === 'gzip' || contentType?.includes('application/gzip')) {
			// Handle gzipped content
			const buffer = await request.arrayBuffer();
			const { gunzipSync } = await import('zlib');
			const decompressed = gunzipSync(Buffer.from(buffer));
			body = JSON.parse(decompressed.toString());
		} else {
			body = await request.json();
		}

		// Get stream name from request (required field)
		const streamName = body.stream_name;
		if (!streamName) {
			return json({ error: 'stream_name is required' }, { status: 400 });
		}

		console.log('[ INGEST EVENT ] :: Stream:', streamName)

		// Check if stream exists, create if not
		let stream = await db
			.select()
			.from(streamConfigs)
			.where(eq(streamConfigs.streamName, streamName))
			.limit(1)
			.then(rows => rows[0]);

		if (!stream) {
			// Create stream record
			const [newStream] = await db.insert(streamConfigs).values({
				streamName: streamName,
				sourceName: deviceSource.sourceName,
				displayName: streamName.replace(/_/g, ' ').replace(/\b\w/g, (l: string) => l.toUpperCase()),
				description: `Raw data stream from ${deviceSource.instanceName}`,
				ingestionType: 'push',
				status: 'active',
				lastIngestionAt: new Date()
			}).returning();
			stream = newStream;
		}

		// Find the actual stream instance for this source and stream config
		let streamInstance = await db
			.select()
			.from(streams)
			.where(and(
				eq(streams.sourceId, deviceSource.id),
				eq(streams.streamConfigId, stream.id)
			))
			.limit(1)
			.then(rows => rows[0]);

		// Create stream instance if it doesn't exist
		if (!streamInstance) {
			const [newStreamInstance] = await db.insert(streams).values({
				sourceId: deviceSource.id,
				streamConfigId: stream.id,
				enabled: true
			}).returning();
			streamInstance = newStreamInstance;
		}

		// Create pipeline activity for ingestion
		const [pipelineActivity] = await db
			.insert(pipelineActivities)
			.values({
				activityType: 'ingestion',
				activityName: `${streamName}_ingestion`,
				sourceName: deviceSource.sourceName,
				streamId: streamInstance.id,  // Use the stream instance ID
				status: 'running',
				startedAt: new Date()
			})
			.returning();

		try {
			// Update last ingestion time
			await db
				.update(streamConfigs)
				.set({ lastIngestionAt: new Date() })
				.where(eq(streamConfigs.id, stream.id));

			// Generic record count - let the source-specific processor determine what to count
			const recordCount = body.batch_metadata?.total_records ||
				body.batch_metadata?.total_points ||
				body.batch_metadata?.total_classifications ||
				body.batch_metadata?.total_chunks ||
				body.data?.length ||
				1; // Default to 1 if no metadata

			// Calculate approximate data size for tracking
			const dataSizeBytes = JSON.stringify(body).length;

			// Queue processing task with raw data directly (no MinIO storage)
			const taskId = await queueCeleryTask('process_stream_data', [
				streamName,
				body,  // Send raw data directly to Celery
				deviceSource.id,
				streamInstance.id,
				pipelineActivity.id
			]);

			// Update pipeline activity to track that we've queued the task
			await db
				.update(pipelineActivities)
				.set({
					dataSizeBytes: dataSizeBytes,
					streamId: streamInstance.id,
					status: 'running',  // Use 'running' instead of 'queued'
					recordsProcessed: recordCount,
					activityMetadata: { task_id: taskId }
				})
				.where(eq(pipelineActivities.id, pipelineActivity.id))

			return json({
				success: true,
				task_id: taskId,
				pipeline_activity_id: pipelineActivity.id,
				data_size_bytes: dataSizeBytes,
				data_size: formatBytes(dataSizeBytes),
				stream_id: streamInstance.id,
				source: deviceSource.sourceName,
				message: `${streamName} data queued for processing`
			});

		} catch (error) {
			// Update pipeline activity with failure
			await db
				.update(pipelineActivities)
				.set({
					status: 'failed',
					completedAt: new Date(),
					errorMessage: error instanceof Error ? error.message : 'Unknown error'
				})
				.where(eq(pipelineActivities.id, pipelineActivity.id));

			throw error;
		}

	} catch (error) {
		console.error('Error processing ingest request:', error);
		return json({
			error: 'Failed to ingest data',
			details: error instanceof Error ? error.message : 'Unknown error'
		}, { status: 500 });
	}
};