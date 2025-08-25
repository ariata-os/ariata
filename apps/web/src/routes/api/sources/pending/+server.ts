import { type RequestHandler } from '@sveltejs/kit';
import { json } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { sources } from '$lib/db/schema';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { sourceName, instanceName, description } = body;

		// Validate required fields
		if (!sourceName || !instanceName) {
			return json({ error: 'Source name and instance name are required' }, { status: 400 });
		}

		// Create a new pending source instance
		const [newSource] = await db
			.insert(sources)
			.values({
				sourceName,
				instanceName,
				description: description || null,
				status: 'pending',
				isActive: false,
				sourceMetadata: {
					createdAt: new Date().toISOString(),
					isPending: true
				}
			})
			.returning();

		console.log(`Created pending source instance ${newSource.id} for ${sourceName}`);

		return json({
			success: true,
			source: {
				id: newSource.id,
				sourceName: newSource.sourceName,
				instanceName: newSource.instanceName,
				status: newSource.status
			}
		});
	} catch (error) {
		console.error('Failed to create pending source:', error);
		return json({ 
			error: 'Failed to create pending source',
			details: error instanceof Error ? error.message : 'Unknown error'
		}, { status: 500 });
	}
};