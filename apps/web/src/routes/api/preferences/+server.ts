import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { preferences } from '$lib/server/schema';

export const GET: RequestHandler = async () => {
	try {
		const db = getDb();
		// Fetch all preferences
		const prefs = await db.select().from(preferences);

		// Convert to object
		const prefsObj: Record<string, string> = {};
		for (const pref of prefs) {
			prefsObj[pref.key] = pref.value;
		}

		// Set defaults for missing values
		if (!prefsObj.user_name) {
			prefsObj.user_name = '';
		}

		return json(prefsObj);
	} catch (error) {
		console.error('Error fetching preferences:', error);
		return json({ error: 'Failed to fetch preferences' }, { status: 500 });
	}
};

export const PATCH: RequestHandler = async ({ request }) => {
	try {
		const db = getDb();
		const updates = await request.json();

		// Validate keys
		const validKeys = ['user_name'];
		const invalidKeys = Object.keys(updates).filter((key) => !validKeys.includes(key));

		if (invalidKeys.length > 0) {
			return json({ error: `Invalid preference keys: ${invalidKeys.join(', ')}` }, { status: 400 });
		}

		// Update each preference
		for (const [key, value] of Object.entries(updates)) {
			await db
				.insert(preferences)
				.values({
					key,
					value: String(value),
					updatedAt: new Date()
				})
				.onConflictDoUpdate({
					target: preferences.key,
					set: {
						value: String(value),
						updatedAt: new Date()
					}
				});
		}

		return json({ success: true });
	} catch (error) {
		console.error('Error updating preferences:', error);
		return json({ error: 'Failed to update preferences' }, { status: 500 });
	}
};