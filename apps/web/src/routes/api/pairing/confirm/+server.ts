import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { sourceConnections } from '$lib/server/schema';
import { eq, and } from 'drizzle-orm';
import { randomUUID } from 'crypto';

/**
 * Pairing Confirm API
 *
 * Called when user clicks "Confirm Pairing" in the /pair page.
 * Creates or updates source record with device pairing.
 */
export const POST: RequestHandler = async ({ request }) => {
	const body = await request.json();
	const { device_id, device_name, device_type } = body;

	if (!device_id || !device_type) {
		return json({ error: 'Missing required fields: device_id, device_type' }, { status: 400 });
	}

	const db = getDb();

	try {
		// Check if device source already exists
		const existingSources = await db
			.select()
			.from(sourceConnections)
			.where(
				and(
					eq(sourceConnections.deviceId, device_id),
					eq(sourceConnections.authType, 'device')
				)
			)
			.limit(1);

		const existing = existingSources[0];

		if (existing) {
			// Update existing source to active pairing
			const updatedDeviceInfo = {
				...(existing.deviceInfo as Record<string, unknown> || {}),
				deviceName: device_name || (existing.deviceInfo as any)?.deviceName,
				deviceType: device_type
			};

			await db
				.update(sourceConnections)
				.set({
					pairingStatus: 'active',
					lastSeenAt: new Date(),
					deviceInfo: updatedDeviceInfo,
					name: device_name || existing.name
				})
				.where(eq(sourceConnections.id, existing.id));

			console.log(`[/api/pairing/confirm] Device ${device_id} re-paired`);

			return json({
				success: true,
				deviceToken: existing.deviceToken
			});
		}

		// Create new device source
		const deviceToken = randomUUID();
		const sourceName = device_name || `${device_type === 'mac' ? 'Mac' : 'iOS'} Device`;

		await db.insert(sourceConnections).values({
			source: device_type, // 'mac' or 'ios'
			name: sourceName,
			authType: 'device',
			deviceId: device_id,
			deviceToken: deviceToken,
			pairingStatus: 'active',
			deviceInfo: {
				deviceName: device_name,
				deviceType: device_type
			},
			isActive: true,
			lastSeenAt: new Date()
		});

		console.log(`[/api/pairing/confirm] New device ${device_id} paired with token ${deviceToken}`);

		return json({
			success: true,
			deviceToken: deviceToken
		});
	} catch (error) {
		console.error('[/api/pairing/confirm] Error confirming pairing:', error);
		return json(
			{
				error: 'Failed to confirm pairing',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
