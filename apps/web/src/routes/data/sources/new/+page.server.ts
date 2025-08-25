import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { sourceConfigs, streamConfigs, sources } from '$lib/db/schema';
import { eq, inArray } from 'drizzle-orm';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ url }) => {
	const sourceName = url.searchParams.get('source');
	
	if (!sourceName) {
		throw error(400, 'Source parameter is required');
	}
	
	try {
		// Get the source configuration
		const [sourceConfig] = await db
			.select()
			.from(sourceConfigs)
			.where(eq(sourceConfigs.name, sourceName))
			.limit(1);
		
		if (!sourceConfig) {
			throw error(404, `Source '${sourceName}' not found`);
		}
		
		// Get all existing instances of this source
		const existingSources = await db
			.select()
			.from(sources)
			.where(eq(sources.sourceName, sourceName));
		
		// Check if at least one instance is connected
		const isConnected = existingSources.length > 0;
		const connectionCount = existingSources.length;
		
		// Get the most recent instance for display purposes
		const existingSource = existingSources.sort((a, b) => 
			(b.updatedAt?.getTime() || 0) - (a.updatedAt?.getTime() || 0)
		)[0] || null;
		const connectionSuccessful = url.searchParams.get('connected') === sourceName;
		
		// Get all stream configurations for this source
		const streams = await db
			.select()
			.from(streamConfigs)
			.where(eq(streamConfigs.sourceName, sourceName));
		
		
		// Parse auth config - stored in oauth_config column for all auth types
		const authData = (sourceConfig.oauthConfig as any) || {};
		
		// Build OAuth URL if this is an OAuth source
		let oauthUrl = null;
		if (sourceConfig.authType === 'oauth2' && authData.authProxy) {
			// Construct the OAuth initiation URL
			const authProxyUrl = authData.authProxy;
			const returnUrl = `${url.origin}/oauth/callback`;
			const state = `/data/sources/new?source=${sourceName}`; // Include source parameter
			
			oauthUrl = `${authProxyUrl}?return_url=${encodeURIComponent(returnUrl)}&state=${encodeURIComponent(state)}`;
			console.log('Generated OAuth URL for', sourceName, ':', oauthUrl);
		}
		
		// Extract device setup configuration if this is a device source
		const deviceSetup = sourceConfig.authType === 'device_token' ? authData.device_setup : null;
		
		// Debug logging
		if (sourceName === 'mac') {
			console.log('Mac source oauthConfig:', authData);
			console.log('Mac deviceSetup:', deviceSetup);
		}
		
		return {
			source: {
				name: sourceConfig.name,
				displayName: sourceConfig.displayName || sourceConfig.name,
				description: sourceConfig.description || '',
				icon: sourceConfig.icon || '',
				platform: sourceConfig.platform,
				authType: sourceConfig.authType,
				company: sourceConfig.company,
				deviceType: sourceConfig.deviceType,
				requiredScopes: authData.requiredScopes || [],
				oauthUrl,
				deviceSetup,
				isConnected,
				connectionCount,
				connectionSuccessful,
				existingSource: existingSource ? {
					id: existingSource.id,
					instanceName: existingSource.instanceName,
					status: existingSource.status,
					deviceToken: existingSource.deviceToken,
					isActive: existingSource.isActive,
					lastSyncAt: existingSource.lastSyncAt
				} : null,
				// Add any sync configuration
				syncConfig: (sourceConfig.syncConfig as any) || {}
			},
			streams: streams.map(stream => ({
				id: stream.id,
				name: stream.streamName,
				displayName: stream.displayName,
				description: stream.description,
				ingestionType: stream.ingestionType,
				cronSchedule: stream.cronSchedule,
				settings: stream.settings || {}
			}))
		};
	} catch (err) {
		console.error('Error loading source configuration:', err);
		if (err && typeof err === 'object' && 'status' in err) {
			throw err;
		}
		throw error(500, 'Failed to load source configuration');
	}
};