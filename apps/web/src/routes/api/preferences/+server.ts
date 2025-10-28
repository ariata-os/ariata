import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { preferences } from '$lib/db/schema';
import { eq } from 'drizzle-orm';

/**
 * Application Preferences API
 *
 * This uses the Postgres app database (ariata_app), NOT the ELT database (ariata_elt).
 * Handles UI preferences like theme, sidebar state, etc.
 */

// GET /api/preferences
export const GET: RequestHandler = async () => {
	try {
		const prefs = await db.select().from(preferences);

		// Convert to key-value object
		const prefsObj = prefs.reduce(
			(acc, pref) => {
				acc[pref.key] = pref.value;
				return acc;
			},
			{} as Record<string, string>
		);

		return json(prefsObj);
	} catch (error) {
		console.error('Failed to get preferences:', error);
		return json({ error: 'Failed to get preferences' }, { status: 500 });
	}
};

// POST /api/preferences
export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { key, value } = body;

		if (!key || value === undefined) {
			return json({ error: 'Missing key or value' }, { status: 400 });
		}

		// Upsert preference
		await db
			.insert(preferences)
			.values({
				key,
				value: JSON.stringify(value),
				updatedAt: new Date()
			})
			.onConflictDoUpdate({
				target: preferences.key,
				set: {
					value: JSON.stringify(value),
					updatedAt: new Date()
				}
			});

		return json({ success: true });
	} catch (error) {
		console.error('Failed to update preference:', error);
		return json({ error: 'Failed to update preference' }, { status: 500 });
	}
};
