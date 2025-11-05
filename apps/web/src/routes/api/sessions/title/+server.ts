import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getPool } from '$lib/server/db';
import { createAnthropic } from '@ai-sdk/anthropic';
import { generateText } from 'ai';
import { env } from '$env/dynamic/private';

// Get Anthropic instance with runtime env
const getAnthropic = () => {
	const apiKey = env.ANTHROPIC_API_KEY;
	if (!apiKey) {
		throw new Error('ANTHROPIC_API_KEY environment variable is not set');
	}
	return createAnthropic({ apiKey });
};

export const POST: RequestHandler = async ({ request }) => {
	const pool = getPool();

	try {
		const body = await request.json();
		const { conversationId, messages } = body;

		if (!conversationId || !messages || !Array.isArray(messages)) {
			return json(
				{
					error: 'conversation_id and messages array are required'
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
		const anthropic = getAnthropic();
		const { text } = await generateText({
			model: anthropic('claude-haiku-4-20250514'),
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

		// Update the conversation title in the database
		const result = await pool.query(
			`
			UPDATE elt.knowledge_ai_conversation
			SET
				conversation_title = $1,
				conversation_last_updated = NOW(),
				updated_at = NOW()
			WHERE conversation_id = $2
			RETURNING conversation_id, conversation_title
			`,
			[title, conversationId]
		);

		if (result.rows.length === 0) {
			return json(
				{
					error: 'Conversation not found'
				},
				{ status: 404 }
			);
		}

		// Refresh materialized view in background (don't wait)
		pool.query('REFRESH MATERIALIZED VIEW CONCURRENTLY elt.conversation_list').catch((err) => {
			console.error('Failed to refresh conversation_list view:', err);
		});

		return json({
			conversation_id: conversationId,
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
