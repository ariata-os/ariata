import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { chatSessions } from '$lib/server/schema';
import { desc } from 'drizzle-orm';

/**
 * Session List API
 *
 * Queries from app.chat_sessions (operational schema) for instant loading.
 * Returns sessions ordered by most recent activity.
 */
export const GET: RequestHandler = async () => {
	console.log('[/api/sessions] GET request received');
	const db = getDb();

	try {
		console.log('[/api/sessions] Querying app.chat_sessions...');

		const sessions = await db
			.select({
				id: chatSessions.id,
				title: chatSessions.title,
				messageCount: chatSessions.messageCount,
				createdAt: chatSessions.createdAt,
				updatedAt: chatSessions.updatedAt
			})
			.from(chatSessions)
			.orderBy(desc(chatSessions.updatedAt))
			.limit(25);

		console.log('[/api/sessions] Query successful, returning', sessions.length, 'sessions');
		return json({
			conversations: sessions.map((s) => ({
				conversation_id: s.id,
				title: s.title,
				message_count: s.messageCount,
				first_message_at: s.createdAt.toISOString(),
				last_updated: s.updatedAt.toISOString()
			})),
			source: 'app_schema'
		});
	} catch (error) {
		console.error('[/api/sessions] Error fetching sessions:', error);
		return json(
			{
				error: 'Failed to fetch sessions',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
