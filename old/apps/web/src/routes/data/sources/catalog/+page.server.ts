import type { PageServerLoad } from './$types';
import { db } from '$lib/db/client';
import { sourceConfigs } from '$lib/db/schema';

export const load: PageServerLoad = async () => {
	try {
		// Get all source configurations (available sources)
		const allSourceConfigs = await db
			.select()
			.from(sourceConfigs)
			.orderBy(sourceConfigs.displayName);
		
		return {
			sources: allSourceConfigs.map(source => ({
				name: source.name,
				displayName: source.displayName || source.name,
				description: source.description || "",
				icon: source.icon || "",
				video: source.video || null,
				platform: source.platform,
				authType: source.authType,
				company: source.company,
				deviceType: source.deviceType
			}))
		};
	} catch (error) {
		console.error('Error loading source catalog:', error);
		return {
			sources: [],
			error: 'Failed to load source catalog'
		};
	}
};