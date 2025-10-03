import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { sourceConfigs, sources } from '$lib/db/schema';
import { eq, desc } from 'drizzle-orm';

export const load: PageServerLoad = async ({ depends }) => {
	// This allows us to invalidate this specific load function
	depends('app:sources');
	
	try {
		// Get all source configurations (templates)
		const allSourceConfigs = await db
			.select()
			.from(sourceConfigs);
		
		// Get all connected source instances
		const connectedSources = await db
			.select()
			.from(sources);
		
		// Create a map of connected sources by source name
		const connectedByName = new Map<string, typeof connectedSources>();
		for (const source of connectedSources) {
			if (!connectedByName.has(source.sourceName)) {
				connectedByName.set(source.sourceName, []);
			}
			connectedByName.get(source.sourceName)!.push(source);
		}
		
		// Build sources list with connection status
		const sourcesList = allSourceConfigs.map(sourceConfig => {
			const connectedInstances = connectedByName.get(sourceConfig.name) || [];
			const isConnected = connectedInstances.length > 0;
			
			// Get OAuth config
			const oauthConfig = (sourceConfig.oauthConfig as any) || {};
			
			// Sort instances by last update time
			const sortedInstances = [...connectedInstances].sort((a, b) => 
				(b.updatedAt?.getTime() || 0) - (a.updatedAt?.getTime() || 0)
			);
			
			// For connected sources, get the most recent instance
			const latestInstance = sortedInstances[0];
			
			// Check if OAuth needs reauth
			const needsReauth = latestInstance?.oauthExpiresAt && 
				new Date(latestInstance.oauthExpiresAt) < new Date();
			
			// Both device and cloud sources can have multiple connections
			const supportsMultiple = true;

			return {
				id: latestInstance?.id, // Add the source instance ID for navigation
				name: sourceConfig.name,
				display_name: sourceConfig.displayName || sourceConfig.name,
				description: sourceConfig.description || "",
				icon: sourceConfig.icon || "",
				video: sourceConfig.video || null,
				platform: sourceConfig.platform,
				enabled: true,
				auth_type: sourceConfig.authType,
				company: sourceConfig.company,
				device_type: sourceConfig.deviceType,
				// Instance-specific data from connected sources
				device_name: latestInstance?.instanceName,
				last_seen: latestInstance?.lastSyncAt,
				oauth_expires_at: latestInstance?.oauthExpiresAt,
				scopes: latestInstance?.scopes,
				status: needsReauth ? 'needs_reauth' : (latestInstance?.status || null),
				fidelity_options: [],
				insider_tip_prompt: "",
				wizard: {},
				required_scopes: oauthConfig.requiredScopes || [],
				auth_proxy: oauthConfig.authProxy,
				is_connected: isConnected,
				connected_count: connectedInstances.length,
				// Multiple connections support for device sources
				multiple_connections: supportsMultiple,
				// Include all instances for device sources
				instances: supportsMultiple ? sortedInstances.map(instance => ({
					id: instance.id,
					instanceName: instance.instanceName,
					status: instance.status,
					lastSyncAt: instance.lastSyncAt,
					deviceType: instance.deviceType,
					createdAt: instance.createdAt
				})) : []
			};
		});
		
		// Sort: connected sources first, then alphabetically
		sourcesList.sort((a, b) => {
			if (a.is_connected !== b.is_connected) {
				return a.is_connected ? -1 : 1;
			}
			return a.display_name.localeCompare(b.display_name);
		});
		
		const sourcesWithSignals = sourcesList.map(source => ({
			...source,
			active_signals_count: 0 // No longer tracking signals
		}));
		
		return {
			sources: sourcesWithSignals
		};
	} catch (error) {
		console.error('Error loading sources:', error);
		return {
			sources: [],
			error: 'Failed to load sources'
		};
	}
};