import { type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { sourceConfigs, sources } from '$lib/db/schema';
import { eq, and, isNotNull } from 'drizzle-orm';

export const GET: RequestHandler = async ({ url }) => {
	try {
		const sourceName = url.searchParams.get('source_name');
		
		if (!sourceName) {
			return new Response(JSON.stringify({ 
				error: 'source_name parameter required' 
			}), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Get the source to check if it's a device source
		const [source] = await db
			.select()
			.from(sourceConfigs)
			.where(eq(sourceConfigs.name, sourceName))
			.limit(1);

		if (!source) {
			return new Response(JSON.stringify({ 
				error: 'Source not found' 
			}), {
				status: 404,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// For device sources, check if already paired
		let availableDevices: any[] = [];
		let connectedDevices: any[] = [];

		if (source.authType === 'device_token') {
			// Check if any instances of this source exist
			const sourceInstances = await db
				.select()
				.from(sources)
				.where(eq(sources.sourceName, sourceName));
			
			if (sourceInstances.length > 0) {
				// Map connected source instances
				connectedDevices = sourceInstances.map(instance => ({
					id: instance.id,
					deviceId: instance.deviceId,
					deviceName: instance.instanceName,
					deviceType: instance.deviceType || source.deviceType || sourceName,
					lastSeen: instance.deviceLastSeen,
					isActive: instance.isActive,
					isConnectedToSource: true,
					connectedSources: [sourceName]
				}));
			}
			// If not connected, availableDevices stays empty (devices need to call /api/sources/activate first)
		}

		return new Response(JSON.stringify({
			available_devices: availableDevices,
			connected_devices: connectedDevices,
			source_name: sourceName,
			message: connectedDevices.length > 0 
				? `Found ${connectedDevices.length} connected device(s)`
				: 'No devices connected. Use your device app to pair with this source.'
		}), {
			headers: { 'Content-Type': 'application/json' }
		});

	} catch (error) {
		console.error('Device search error:', error);
		return new Response(JSON.stringify({ 
			error: 'Failed to search for devices' 
		}), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};