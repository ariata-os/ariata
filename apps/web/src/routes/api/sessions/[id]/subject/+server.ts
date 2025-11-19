import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getPool } from '$lib/server/db';
import { env } from '$env/dynamic/private';
import type { ChatMessage } from '$lib/server/schema';
import { generateSubject, updateSubjectInMessages } from '$lib/services/subject-generator';

// Verify AI Gateway API key is set
if (!env.AI_GATEWAY_API_KEY) {
	console.warn('AI_GATEWAY_API_KEY environment variable is not set');
}

export const POST: RequestHandler = async ({ params, request }) => {
	const pool = getPool();
	const sessionId = params.id;

	try {
		const body = await request.json();
		const { exchangeIndex } = body;

		if (typeof exchangeIndex !== 'number' || exchangeIndex < 0) {
			return json(
				{
					error: 'exchangeIndex (number >= 0) is required'
				},
				{ status: 400 }
			);
		}

		// Fetch the session
		const sessionResult = await pool.query(
			`
			SELECT id, messages
			FROM app.chat_sessions
			WHERE id = $1
			`,
			[sessionId]
		);

		if (sessionResult.rows.length === 0) {
			return json(
				{
					error: 'Session not found'
				},
				{ status: 404 }
			);
		}

		const messages: ChatMessage[] = sessionResult.rows[0].messages;

		// Generate subject using the service (handles retries and fallback)
		const subject = await generateSubject(messages, exchangeIndex);

		// Update the messages array with the new subject
		const updatedMessages = updateSubjectInMessages(messages, exchangeIndex, subject);

		// Save back to database
		await pool.query(
			`
			UPDATE app.chat_sessions
			SET
				messages = $1,
				updated_at = NOW()
			WHERE id = $2
			`,
			[JSON.stringify(updatedMessages), sessionId]
		);

		return json({
			sessionId,
			exchangeIndex,
			subject
		});
	} catch (error) {
		console.error('Error generating exchange subject:', error);
		return json(
			{
				error: 'Failed to generate exchange subject',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
