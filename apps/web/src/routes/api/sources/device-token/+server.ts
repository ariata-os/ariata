import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { sources } from '$lib/db/schema';
import { eq } from 'drizzle-orm';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { sourceName, instanceName, description } = body;

		// Validate required fields
		if (!sourceName || !instanceName) {
			return json({ 
				success: false,
				error: 'Source name and instance name are required' 
			}, { status: 400 });
		}

		// Generate device token - 8 character alphanumeric, easy to type
		// Using safe character set (no confusing chars like 0/O, 1/I/L)
		const safeChars = '23456789ABCDEFGHJKMNPQRSTUVWXYZ';
		let deviceToken = '';
		const array = new Uint8Array(8);
		crypto.getRandomValues(array);
		for (let i = 0; i < 8; i++) {
			deviceToken += safeChars[array[i] % safeChars.length];
		}

		// Generate a unique device ID
		const deviceId = crypto.randomUUID();

		// Create the source immediately with pending status
		const [newSource] = await db
			.insert(sources)
			.values({
				sourceName,
				instanceName: instanceName.trim(),
				status: 'pending', // Will become 'active' when device connects
				deviceToken,
				deviceId,
				deviceType: sourceName === 'ios' ? 'ios' : sourceName === 'mac' ? 'macos' : null,
				sourceMetadata: {
					description: description || '',
					createdVia: 'web',
					tokenGeneratedAt: new Date().toISOString()
				}
			})
			.returning();

		if (!newSource) {
			return json({ 
				success: false,
				error: 'Failed to create device source' 
			}, { status: 500 });
		}

		// Return success with the generated token
		return json({
			success: true,
			source: {
				id: newSource.id,
				instanceName: newSource.instanceName,
				sourceName: newSource.sourceName,
				status: newSource.status
			},
			deviceToken,
			message: 'Device token generated successfully. Use this token in your device app.'
		});

	} catch (error) {
		console.error('Device token generation error:', error);
		return json({ 
			success: false,
			error: 'Failed to generate device token' 
		}, { status: 500 });
	}
};

// GET endpoint to check if a token exists
export const GET: RequestHandler = async ({ url }) => {
	const token = url.searchParams.get('token');
	
	if (!token) {
		return json({ 
			success: false,
			error: 'Token parameter required' 
		}, { status: 400 });
	}

	try {
		const [source] = await db
			.select({
				id: sources.id,
				instanceName: sources.instanceName,
				status: sources.status,
				lastSyncAt: sources.lastSyncAt
			})
			.from(sources)
			.where(eq(sources.deviceToken, token))
			.limit(1);

		if (!source) {
			return json({ 
				success: false,
				exists: false
			});
		}

		return json({
			success: true,
			exists: true,
			source: {
				id: source.id,
				instanceName: source.instanceName,
				status: source.status,
				connected: source.status === 'active',
				lastSyncAt: source.lastSyncAt
			}
		});

	} catch (error) {
		console.error('Token check error:', error);
		return json({ 
			success: false,
			error: 'Failed to check token' 
		}, { status: 500 });
	}
};