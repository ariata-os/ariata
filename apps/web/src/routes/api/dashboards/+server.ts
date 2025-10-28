import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { dashboards } from '$lib/db/schema';

/**
 * Dashboards API
 *
 * This uses the Postgres app database (ariata_app), NOT the ELT database (ariata_elt).
 * Manages saved dashboard layouts and configurations.
 */

// GET /api/dashboards
export const GET: RequestHandler = async () => {
	try {
		const allDashboards = await db.select().from(dashboards);
		return json(allDashboards);
	} catch (error) {
		console.error('Failed to list dashboards:', error);
		return json({ error: 'Failed to list dashboards' }, { status: 500 });
	}
};

// POST /api/dashboards
export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { name, description, layout } = body;

		if (!name || !layout) {
			return json({ error: 'Missing name or layout' }, { status: 400 });
		}

		const [newDashboard] = await db
			.insert(dashboards)
			.values({
				name,
				description: description || null,
				layout: JSON.stringify(layout),
				isDefault: false,
				createdAt: new Date(),
				updatedAt: new Date()
			})
			.returning();

		return json(newDashboard);
	} catch (error) {
		console.error('Failed to create dashboard:', error);
		return json({ error: 'Failed to create dashboard' }, { status: 500 });
	}
};
