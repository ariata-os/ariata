import { createAnthropic } from '@ai-sdk/anthropic';
import { streamText, stepCountIs, type StreamTextResult } from 'ai';
import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';
import { createQueryOntologiesTool } from '$lib/tools/query-ontologies';
import { getDb, getPool } from '$lib/server/db';
import { chatSessions, type ChatMessage } from '$lib/server/schema';
import { eq, sql } from 'drizzle-orm';
import type { CoreMessage } from 'ai';

// Constants
const MAX_TITLE_LENGTH = 50;
const ALLOWED_MODELS = [
	'claude-sonnet-4-20250514',
	'claude-opus-4-20250514',
	'claude-haiku-4-20250514'
];

// Get Anthropic instance with runtime env
const getAnthropic = () => {
	const apiKey = env.ANTHROPIC_API_KEY;
	if (!apiKey) {
		throw new Error('ANTHROPIC_API_KEY environment variable is not set');
	}
	return createAnthropic({ apiKey });
};

/**
 * Chat API endpoint
 *
 * Accepts messages from the frontend, streams responses from Claude,
 * and saves both user and assistant messages to app.chat_sessions (operational schema).
 * Export to ELT happens asynchronously via scheduled job (every 5 minutes).
 */
export const POST: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const { messages, sessionId, model = 'claude-sonnet-4-20250514' } = body;

		console.log('[API] Received request with model:', model, 'sessionId:', sessionId);

		// Validate messages
		if (!messages || !Array.isArray(messages)) {
			return new Response(JSON.stringify({ error: 'Invalid messages format' }), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Validate sessionId
		if (!sessionId) {
			return new Response(JSON.stringify({ error: 'Missing sessionId' }), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Validate sessionId is a valid UUID
		const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
		if (!uuidRegex.test(sessionId)) {
			return new Response(JSON.stringify({ error: 'Invalid sessionId format (must be UUID)' }), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Validate model
		if (!ALLOWED_MODELS.includes(model)) {
			return new Response(
				JSON.stringify({
					error: 'Invalid model',
					allowed: ALLOWED_MODELS
				}),
				{
					status: 400,
					headers: { 'Content-Type': 'application/json' }
				}
			);
		}

		const db = getDb();
		const pool = getPool();

		// Get or create session
		let session = await db.query.chatSessions.findFirst({
			where: eq(chatSessions.id, sessionId)
		});

		if (!session) {
			// Create new session with auto-generated title from first user message
			const firstUserMessage = messages.find((m) => m.role === 'user')?.content || 'New conversation';
			const autoTitle =
				firstUserMessage.slice(0, MAX_TITLE_LENGTH) +
				(firstUserMessage.length > MAX_TITLE_LENGTH ? '...' : '');

			[session] = await db
				.insert(chatSessions)
				.values({
					id: sessionId,
					title: autoTitle,
					messages: []
				})
				.returning();

			console.log('[API] Created new session:', sessionId, 'with title:', autoTitle);
		}

		// Create dynamic tool based on enabled streams/ontologies
		const queryOntologiesTool = await createQueryOntologiesTool(pool);

		// Call Claude API with streaming
		const anthropic = getAnthropic();
		const result = streamText({
			model: anthropic(model),
			messages: messages as CoreMessage[],
			temperature: 0.7,
			tools: {
				queryOntologies: queryOntologiesTool
			},
			maxSteps: 5 // Allow multi-step tool calls up to 5 steps
		});

		// Convert to Response stream
		const stream = result.toTextStreamResponse();

		// CRITICAL: Save messages synchronously to prevent data loss
		// We must wait for the AI response to complete and save before returning
		// This ensures messages are persisted even if client disconnects
		await saveMessagesToSession(sessionId, messages as CoreMessage[], result, model);

		return stream;
	} catch (error) {
		console.error('Chat API error:', error);
		return new Response(JSON.stringify({ error: 'Internal server error' }), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};

/**
 * Save messages to app.chat_sessions (operational schema)
 *
 * Uses atomic JSONB operations to prevent lost updates from concurrent requests.
 * Export to ELT pipeline happens asynchronously via scheduled job.
 */
async function saveMessagesToSession(
	sessionId: string,
	userMessages: CoreMessage[],
	result: StreamTextResult<any, any>,
	model: string
) {
	const db = getDb();

	// Wait for assistant response to complete
	const assistantContent = await result.text;

	// Build new messages array with accurate timestamps
	const now = new Date();
	const newMessages: ChatMessage[] = [
		// Add user messages (typically just one, but handle multiple)
		...userMessages
			.filter((m) => m.role === 'user')
			.map((m, idx) => ({
				role: 'user' as const,
				content: typeof m.content === 'string' ? m.content : JSON.stringify(m.content),
				timestamp: new Date(now.getTime() + idx).toISOString(), // Offset by 1ms per message
				model: null
			})),
		// Add assistant message
		{
			role: 'assistant' as const,
			content: assistantContent,
			timestamp: new Date(now.getTime() + userMessages.length).toISOString(),
			model,
			provider: 'anthropic'
		}
	];

	// Use atomic JSONB operations to prevent lost updates from concurrent requests
	// This ensures concurrent writes to the same session don't lose data
	const updateResult = await db
		.update(chatSessions)
		.set({
			messages: sql`messages || ${JSON.stringify(newMessages)}::jsonb`,
			updatedAt: now,
			messageCount: sql`message_count + ${newMessages.length}`
		})
		.where(eq(chatSessions.id, sessionId))
		.returning({ messageCount: chatSessions.messageCount });

	if (updateResult.length === 0) {
		throw new Error(`Session not found: ${sessionId}`);
	}

	console.log(
		`[saveMessages] Saved ${newMessages.length} messages to session ${sessionId} (total: ${updateResult[0].messageCount})`
	);
}
