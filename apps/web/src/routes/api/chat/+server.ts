import type { RequestHandler } from './$types';
import type { Config } from '@sveltejs/adapter-vercel';
import { env } from '$env/dynamic/private';
import { orchestrateChat } from '$lib/server/agent-orchestrator';
import { getDb, getPool } from '$lib/server/db';
import { chatSessions, preferences, type ChatMessage } from '$lib/server/schema';
import { eq, sql } from 'drizzle-orm';
import { convertToModelMessages, type UIMessage } from 'ai';

// Constants
const MAX_TITLE_LENGTH = 50;
// Anthropic model strings
const ALLOWED_MODELS = [
	'claude-sonnet-4-20250514', // Claude Sonnet 4
	'claude-opus-4-20250514', // Claude Opus 4
	'claude-haiku-4-20250514' // Claude Haiku 4
];

// Verify Anthropic API key is set
if (!env.ANTHROPIC_API_KEY) {
	console.warn('ANTHROPIC_API_KEY environment variable is not set');
}

// Vercel serverless function configuration
// Increase timeout for complex multi-step queries (default is 10s on Hobby, 15s on Pro)
export const config: Config = {
	maxDuration: 60 // seconds - adjust based on Vercel plan
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
		console.log('[API] Messages format:', messages?.[0]); // Log first message to see format

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

		// Load user preferences for personalization
		const prefs = await db.select().from(preferences);
		const userName = prefs.find((p) => p.key === 'user_name')?.value || 'the user';

		// Get database pool for tools
		const pool = getPool();

		// Convert UIMessages to ModelMessages
		const modelMessages = convertToModelMessages(messages as UIMessage[]);
		console.log('[API] Converted to ModelMessages:', modelMessages.length, 'messages');

		// Run agent with custom tools (queryOntologies, queryLocationMap)
		// Uses @ai-sdk/anthropic provider with ANTHROPIC_API_KEY
		const result = await orchestrateChat({
			messages: modelMessages,
			model,
			pool,
			sessionId,
			userName,
			onFinish: async (event) => {
				try {
					await saveMessagesToSession(sessionId, messages as UIMessage[], event, model);
				} catch (error) {
					console.error('[API] Failed to save messages to session:', error);
				}
			}
		});

		// toUIMessageStreamResponse() streams tool calls in real-time (compatible with useChat)
		// Tool calls are saved to DB in the onFinish callback
		return result.toUIMessageStreamResponse();
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
	userMessages: UIMessage[],
	event: any, // onFinish event containing text, toolCalls, toolResults, etc.
	model: string
) {
	const db = getDb();

	// Extract data from the onFinish event
	const assistantContent = event.text;
	const toolCalls = event.toolCalls;
	const toolResults = event.toolResults;

	console.log('[saveMessages] Tool calls:', toolCalls?.length || 0);
	console.log('[saveMessages] Tool results:', toolResults?.length || 0);
	if (toolCalls && toolCalls.length > 0) {
		console.log('[saveMessages] First tool call:', JSON.stringify(toolCalls[0], null, 2));
		console.log('[saveMessages] First tool result:', JSON.stringify(toolResults?.[0], null, 2));
		console.log('[saveMessages] First tool result.output:', JSON.stringify(toolResults?.[0]?.output, null, 2));
	}

	// Build new messages array with accurate timestamps
	const now = new Date();
	const newMessages: ChatMessage[] = [
		// Add user messages (typically just one, but handle multiple)
		...userMessages
			.filter((m) => m.role === 'user')
			.map((m, idx) => ({
				role: 'user' as const,
				content: m.parts.find(p => p.type === 'text')?.text || '',
				timestamp: new Date(now.getTime() + idx).toISOString(), // Offset by 1ms per message
				model: null
			})),
		// Add assistant message with tool calls
		{
			role: 'assistant' as const,
			content: assistantContent,
			timestamp: new Date(now.getTime() + userMessages.length).toISOString(),
			model,
			provider: 'anthropic',
			tool_calls:
				toolCalls && toolCalls.length > 0
					? toolCalls.map((call, idx) => ({
							tool_name: call.toolName,
							arguments: call.args,
							result: toolResults?.[idx]?.output || toolResults?.[idx]?.result,
							timestamp: new Date().toISOString()
						}))
					: undefined
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
