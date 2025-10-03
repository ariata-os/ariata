import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { sources, sourceConfigs } from '$lib/db/schema';
import { eq } from 'drizzle-orm';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const { id } = params;
		
		// Fetch the source instance by ID
		const sourceResult = await db
			.select()
			.from(sources)
			.where(eq(sources.id, id))
			.limit(1);
		
		if (sourceResult.length === 0) {
			throw error(404, 'Source not found');
		}
		
		const source = sourceResult[0];
		
		// Fetch the source configuration for display metadata
		const configResult = await db
			.select()
			.from(sourceConfigs)
			.where(eq(sourceConfigs.name, source.sourceName))
			.limit(1);
		
		const config = configResult[0] || {};
		
		// Format the data for display
		const sourceData = {
			// Basic Information
			id: source.id,
			sourceName: source.sourceName,
			displayName: config.displayName || source.sourceName,
			instanceName: source.instanceName,
			description: config.description || '',
			icon: config.icon || '',
			platform: config.platform || '',
			authType: config.authType || '',
			
			// Status
			status: source.status,
			
			// Device Information (if applicable)
			deviceId: source.deviceId,
			deviceType: source.deviceType,
			deviceToken: source.deviceToken ? '••••••' + source.deviceToken.slice(-4) : null,
			pairedDeviceName: source.pairedDeviceName,
			deviceLastSeen: source.deviceLastSeen,
			
			// OAuth Information (if applicable)
			oauthAccessToken: source.oauthAccessToken ? '••••••' + source.oauthAccessToken.slice(-4) : null,
			oauthRefreshToken: source.oauthRefreshToken ? '••••••' + source.oauthRefreshToken.slice(-4) : null,
			oauthExpiresAt: source.oauthExpiresAt,
			scopes: source.scopes || [],
			
			// Pairing Information
			pairingCode: source.pairingCode,
			pairingExpiresAt: source.pairingExpiresAt,
			
			// Sync Information
			lastSyncAt: source.lastSyncAt,
			lastSyncStatus: source.lastSyncStatus,
			lastSyncError: source.lastSyncError,
			
			// Metadata
			sourceMetadata: source.sourceMetadata || {},
			
			// Timestamps
			createdAt: source.createdAt,
			updatedAt: source.updatedAt,
		};
		
		return {
			source: sourceData
		};
	} catch (err) {
		console.error('Error loading source:', err);
		if (err instanceof Error && 'status' in err) {
			throw err; // Re-throw Kit errors
		}
		throw error(500, 'Failed to load source');
	}
};