import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getPool } from '$lib/server/db';
import { generateText } from 'ai';
import { env } from '$env/dynamic/private';

// Verify AI Gateway API key is set
if (!env.AI_GATEWAY_API_KEY) {
	console.warn('AI_GATEWAY_API_KEY environment variable is not set');
}

export const POST: RequestHandler = async ({ request }) => {
	const pool = getPool();

	try {
		const body = await request.json();
		const { sessionId, messages } = body;

		if (!sessionId || !messages || !Array.isArray(messages)) {
			return json(
				{
					error: 'sessionId and messages array are required'
				},
				{ status: 400 }
			);
		}

		// Create a prompt for title generation
		// Include first 2-3 message exchanges to capture the essence of the conversation
		const messagesToInclude = messages.slice(0, Math.min(6, messages.length));
		const conversationSummary = messagesToInclude
			.map((m) => `${m.role}: ${m.content.substring(0, 200)}`)
			.join('\n\n');

		// Use Claude Haiku to generate a short title
		// When using AI Gateway, simply pass the model string
		// The AI SDK will automatically use the AI_GATEWAY_API_KEY environment variable
		const { text } = await generateText({
			model: 'anthropic/claude-haiku-4.5',
			maxSteps: 1,
			prompt: `Based on this conversation, generate a very short title (3-6 words maximum) that captures the main topic or theme. Only return the title, nothing else.

Conversation:
${conversationSummary}`
		});

		let title = text.trim();

		// Remove quotes if present
		title = title.replace(/^["']|["']$/g, '');

		// Truncate if too long
		if (title.length > 60) {
			title = title.substring(0, 57) + '...';
		}

		// Update the chat session title in the app database
		const result = await pool.query(
			`
			UPDATE app.chat_sessions
			SET
				title = $1,
				updated_at = NOW()
			WHERE id = $2
			RETURNING id, title
			`,
			[title, sessionId]
		);

		if (result.rows.length === 0) {
			return json(
				{
					error: 'Session not found'
				},
				{ status: 404 }
			);
		}

		return json({
			session_id: sessionId,
			title
		});
	} catch (error) {
		console.error('Error generating conversation title:', error);
		return json(
			{
				error: 'Failed to generate conversation title',
				details: error instanceof Error ? error.message : 'Unknown error'
			},
			{ status: 500 }
		);
	}
};
