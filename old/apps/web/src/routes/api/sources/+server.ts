import { type RequestHandler } from '@sveltejs/kit';
import { json } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { sourceConfigs, sources } from '$lib/db/schema';
import { eq } from 'drizzle-orm';

export const GET: RequestHandler = async () => {
	try {
		// Get all sources
		const allSources = await db.select().from(sourceConfigs);

		// Build source list from database
		const sourceList = allSources.map(source => {
			// Get OAuth config or empty object if null
			const oauthConfig = (source.oauthConfig as any) || {};

			return {
				name: source.name,
				display_name: source.displayName || source.name,
				description: source.description || "",
				platform: source.platform,
				enabled: true,
				auth_type: source.authType,
				company: source.company,
				required_scopes: oauthConfig.requiredScopes || [],
				auth_proxy: oauthConfig.authProxy,
				multiple_connections: false
			};
		});

		return new Response(JSON.stringify(sourceList), {
			headers: { 'Content-Type': 'application/json' }
		});

	} catch (error) {
		console.error('Failed to fetch sources:', error);
		return new Response(JSON.stringify({ error: 'Failed to fetch sources' }), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};

export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const {
			sourceName,
			instanceName,
			description,
			deviceToken,
			streamConfigs: streamSettings,
		} = body;

		// Validate required fields
		if (!sourceName || !instanceName) {
			return json({ 
				error: 'Source name and instance name are required' 
			}, { status: 400 });
		}

		// Get source configuration
		const [sourceConfig] = await db
			.select()
			.from(sourceConfigs)
			.where(eq(sourceConfigs.name, sourceName))
			.limit(1);

		if (!sourceConfig) {
			return json({ 
				error: 'Invalid source type' 
			}, { status: 400 });
		}

		// Create the source instance
		const [newSource] = await db
			.insert(sources)
			.values({
				sourceName,
				instanceName,
				deviceToken: deviceToken || null,
				lastSyncStatus: 'pending',
				sourceMetadata: {
					createdFrom: 'web',
					description: description || null,
					streamConfigs: streamSettings || [],
				},
			})
			.returning();

		return json({
			success: true,
			id: newSource.id,
			message: 'Device source created successfully'
		});

	} catch (error) {
		console.error('Failed to create source:', error);
		return json({ 
			error: 'Failed to create source' 
		}, { status: 500 });
	}
};