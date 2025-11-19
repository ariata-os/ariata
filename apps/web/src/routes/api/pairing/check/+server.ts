import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { sourceConnections } from '$lib/server/schema';
import { eq, and } from 'drizzle-orm';

/**
 * Pairing Check API
 *
 * Checks if a device ID already exists in the source_connections table.
 * Used by the /pair page to pre-fill device name.
 */
export const GET: RequestHandler = async ({ url }) => {
	const deviceId = url.searchParams.get('device_id');

	if (!deviceId) {
		return json({ error: 'Missing device_id parameter' }, { status: 400 });
	}

	const db = getDb();

	try {
		const sourcesResult = await db
			.select()
			.from(sourceConnections)
			.where(
				and(
					eq(sourceConnections.deviceId, deviceId),
					eq(sourceConnections.authType, 'device')
				)
			)
			.limit(1);

		const source = sourcesResult[0];

		if (!source) {
			return json({
				exists: false,
				deviceName: null
			});
		}

		const deviceInfo = source.deviceInfo as { deviceName?: string } | null;

		return json({
			exists: true,
			deviceName: deviceInfo?.deviceName || source.name,
			paired: source.pairingStatus === 'active'
		});
	} catch (error) {
		console.error('[/api/pairing/check] Error checking device:', error);
		return json(
			{
				error: 'Failed to check device',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
