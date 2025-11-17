import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getDb } from '$lib/server/db';
import { chatSessions, type ChatMessage } from '$lib/server/schema';
import { eq } from 'drizzle-orm';

/**
 * Session Detail API
 *
 * GET: Returns full conversation with all messages from app.chat_sessions
 * PATCH: Updates session title
 * DELETE: Deletes session from operational schema
 */

export const GET: RequestHandler = async ({ params }) => {
	const { id: sessionId } = params;

	// Validate UUID format
	const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	if (!uuidRegex.test(sessionId)) {
		return json(
			{
				error: 'Invalid sessionId format (must be UUID)'
			},
			{ status: 400 }
		);
	}

	const db = getDb();

	try {
		console.log('[/api/sessions/[id]] GET request for session:', sessionId);

		const session = await db.query.chatSessions.findFirst({
			where: eq(chatSessions.id, sessionId)
		});

		if (!session) {
			console.log('[/api/sessions/[id]] Session not found:', sessionId);
			return json(
				{
					error: 'Session not found'
				},
				{ status: 404 }
			);
		}

		console.log('[/api/sessions/[id]] Session found with', session.messages.length, 'messages');

		// Extract metadata from messages (handle empty array)
		const lastMessage = session.messages.length > 0 ? session.messages[session.messages.length - 1] : null;
		const conversation = {
			conversation_id: session.id,
			title: session.title,
			first_message_at: session.createdAt.toISOString(),
			last_message_at: session.updatedAt.toISOString(),
			message_count: session.messageCount,
			model: lastMessage?.model || null,
			provider: lastMessage?.provider || null
		};

		// Format messages for frontend (add IDs for compatibility)
		const messages = session.messages.map((msg: ChatMessage, idx: number) => ({
			id: `${session.id}_${idx}`,
			role: msg.role,
			content: msg.content,
			timestamp: msg.timestamp,
			model: msg.model || undefined,
			tool_calls: msg.tool_calls || undefined
		}));

		// TEMP DEBUG: Log tool_calls for debugging persistence
		const messagesWithTools = messages.filter(m => m.tool_calls && m.tool_calls.length > 0);
		if (messagesWithTools.length > 0) {
			console.log('[/api/sessions/[id]] Found', messagesWithTools.length, 'messages with tool_calls');
			console.log('[/api/sessions/[id]] First tool call sample:', JSON.stringify(messagesWithTools[0].tool_calls[0], null, 2));
		} else {
			console.log('[/api/sessions/[id]] NO messages with tool_calls found');
		}

		return json({
			conversation,
			messages
		});
	} catch (error) {
		console.error('[/api/sessions/[id]] Error fetching session:', error);
		return json(
			{
				error: 'Failed to fetch session',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};

export const PATCH: RequestHandler = async ({ params, request }) => {
	const { id: sessionId } = params;
	const db = getDb();

	try {
		const body = await request.json();
		const { title } = body;

		if (!title || typeof title !== 'string') {
			return json(
				{
					error: 'Title is required and must be a string'
				},
				{ status: 400 }
			);
		}

		console.log('[/api/sessions/[id]] PATCH request for session:', sessionId, 'with title:', title);

		// Update session title
		const result = await db
			.update(chatSessions)
			.set({
				title,
				updatedAt: new Date()
			})
			.where(eq(chatSessions.id, sessionId))
			.returning();

		if (result.length === 0) {
			return json(
				{
					error: 'Session not found'
				},
				{ status: 404 }
			);
		}

		return json({
			conversation_id: result[0].id,
			title: result[0].title,
			updated_at: result[0].updatedAt.toISOString()
		});
	} catch (error) {
		console.error('[/api/sessions/[id]] Error updating session title:', error);
		return json(
			{
				error: 'Failed to update session title',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};

export const DELETE: RequestHandler = async ({ params }) => {
	const { id: sessionId } = params;
	const db = getDb();

	try {
		console.log('[/api/sessions/[id]] DELETE request for session:', sessionId);

		// Delete session from operational schema
		// Note: Existing messages in data.stream_ariata_ai_chat remain for analytics
		const result = await db
			.delete(chatSessions)
			.where(eq(chatSessions.id, sessionId))
			.returning();

		if (result.length === 0) {
			return json(
				{
					error: 'Session not found'
				},
				{ status: 404 }
			);
		}

		return json({
			success: true,
			conversation_id: result[0].id
		});
	} catch (error) {
		console.error('[/api/sessions/[id]] Error deleting session:', error);
		return json(
			{
				error: 'Failed to delete session',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
