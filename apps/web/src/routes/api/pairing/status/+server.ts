import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { sources } from '$lib/server/schema';
import { eq, and } from 'drizzle-orm';

/**
 * Pairing Status API
 *
 * Polled by Mac app every 2 seconds to check if pairing is complete.
 * Returns pairing status and device token if paired.
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
			.from(sources)
			.where(
				and(
					eq(sources.deviceId, deviceId),
					eq(sources.authType, 'device')
				)
			)
			.limit(1);

		const source = sourcesResult[0];

		if (!source) {
			// Device not yet in pairing flow
			return json({
				paired: false,
				deviceToken: null,
				apiEndpoint: null
			});
		}

		if (source.pairingStatus !== 'active') {
			// Pairing initiated but not confirmed (status is 'pending' or 'revoked')
			return json({
				paired: false,
				deviceToken: null,
				apiEndpoint: null
			});
		}

		// Pairing complete!
		const apiEndpoint = process.env.PUBLIC_API_URL || process.env.ARIATA_API_URL || 'http://localhost:3000';

		return json({
			paired: true,
			deviceToken: source.deviceToken,
			apiEndpoint: apiEndpoint
		});
	} catch (error) {
		console.error('[/api/pairing/status] Error checking pairing status:', error);
		return json(
			{
				error: 'Failed to check pairing status',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
