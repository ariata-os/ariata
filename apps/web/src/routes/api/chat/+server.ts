import type { RequestHandler } from './$types';
import type { Config } from '@sveltejs/adapter-vercel';
import { env } from '$env/dynamic/private';
import { getAgent, initializeAgents } from '$lib/server/agents/registry';
import { routeToAgent, isValidAgentId } from '$lib/server/routing/intent-router';
import { getDb, getPool } from '$lib/server/db';
import { chatSessions, preferences, assistantProfile, type ChatMessage } from '$lib/server/schema';
import { eq, sql } from 'drizzle-orm';
import {
	isTextUIPart,
	isToolOrDynamicToolUIPart,
	createAgentUIStreamResponse,
	createIdGenerator,
	type UIMessage
} from 'ai';
import type { AgentId } from '$lib/server/agents/types';

// Constants
const MAX_TITLE_LENGTH = 50;

// AI Gateway model strings (provider/model format)
const ALLOWED_MODELS = [
	'anthropic/claude-sonnet-4.5',
	'anthropic/claude-opus-4.1',
	'anthropic/claude-haiku-4.5',
	'openai/gpt-5',
	'openai/gpt-oss-120b',
	'openai/gpt-oss-20b',
	'google/gemini-2.5-pro',
	'google/gemini-2.5-flash',
	'xai/grok-4',
	'moonshotai/kimi-k2-thinking'
];

// Verify AI Gateway API key is set
if (!env.AI_GATEWAY_API_KEY) {
	console.warn('AI_GATEWAY_API_KEY environment variable is not set');
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
		const {
			messages,
			sessionId,
			model = 'openai/gpt-oss-120b',
			agentId = 'auto'
		} = body;

		console.log('[API] Received request');
		console.log('[API]   sessionId:', sessionId);
		console.log('[API]   model:', model);
		console.log('[API]   agentId:', agentId);
		console.log('[API]   messages:', messages?.length);

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

		// Validate model (still supported for backwards compatibility and override)
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

		// Validate agentId
		if (!isValidAgentId(agentId)) {
			return new Response(
				JSON.stringify({
					error: 'Invalid agentId',
					allowed: ['auto', 'analytics', 'research', 'general', 'action']
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
		const userName = prefs.find((p) => p.key === 'user_name')?.value || 'User';

		// Load assistant profile for AI personalization
		const assistantProfiles = await db.select().from(assistantProfile).limit(1);
		const assistantName = assistantProfiles[0]?.assistantName || 'Ariata';

		// Initialize agents with user and assistant names from database
		// Smart caching: only reinitializes if settings have changed
		await initializeAgents(userName, assistantName);

		// === AGENT ROUTING ===
		// Determine which agent should handle this request
		let selectedAgentId: AgentId;
		let routingReason: string;

		if (agentId === 'auto') {
			// Auto-routing: use intent detection
			const lastMessage = messages[messages.length - 1];
			const messageText =
				lastMessage?.role === 'user'
					? Array.isArray(lastMessage.parts)
						? lastMessage.parts
								.filter(isTextUIPart)
								.map((p) => p.text)
								.join('')
						: lastMessage.content || ''
					: '';

			const routing = routeToAgent({
				message: messageText,
				recentMessages: messages.slice(-3) as UIMessage[], // Last 3 messages for context
			});

			selectedAgentId = routing.agentId;
			routingReason = routing.reason;
			console.log('[API] Auto-routed to agent:', selectedAgentId);
			console.log('[API] Routing reason:', routingReason);
		} else {
			// Manual agent selection
			selectedAgentId = agentId as AgentId;
			routingReason = `User selected ${agentId} agent`;
			console.log('[API] User selected agent:', selectedAgentId);
		}

		// Get the selected agent
		const agent = getAgent(selectedAgentId);

		// Add context as a system message in UIMessage format
		const currentDate = new Date().toISOString().split('T')[0];
		const contextMessage: UIMessage = {
			id: 'system-context',
			role: 'system',
			parts: [
				{
					type: 'text',
					text: `User: ${userName}\nCurrent date: ${currentDate} (YYYY-MM-DD format)`,
				},
			],
		};

		// Prepend context to original UIMessages
		const messagesWithContext = [contextMessage, ...(messages as UIMessage[])];

		// Stream response using agent
		console.log('[API] Streaming response with', selectedAgentId, 'agent');
		return createAgentUIStreamResponse({
			agent,
			messages: messagesWithContext,
			// AI SDK v6: Pass originalMessages so onFinish receives complete conversation
			// Do NOT include the system context message in originalMessages
			originalMessages: messages as UIMessage[],
			// AI SDK v6: Server-side ID generation for consistent message IDs
			generateMessageId: createIdGenerator({
				prefix: 'msg',
				size: 16
			}),
			onFinish: async ({ messages: completeMessages }) => {
				try {
					// Save messages along with agent metadata
					await saveMessagesToSession(sessionId, completeMessages, model, selectedAgentId);
				} catch (error) {
					console.error('[API] Failed to save messages to session:', error);
				}
			},
			// Error handling - sanitize errors before sending to client
			onError: (error) => {
				console.error('[API] Stream error:', error);

				// Return sanitized error message to client
				// Don't expose internal details in production
				if (error instanceof Error) {
					return `An error occurred: ${error.message}`;
				}

				return 'An unexpected error occurred. Please try again.';
			},
		});
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
 *
 * AI SDK v6 Best Practice: Save complete UIMessages from toUIMessageStreamResponse onFinish
 * The messages parameter contains the full conversation including user and assistant messages.
 */
async function saveMessagesToSession(
	sessionId: string,
	completeMessages: UIMessage[],
	model: string,
	agentId?: string
) {
	const db = getDb();

	// Convert complete UIMessages to our ChatMessage format
	const now = new Date();
	const allMessages: ChatMessage[] = [];

	// Build a map of tool results from the messages
	// In UI SDK, tool results are embedded in message parts
	const toolResultsMap = new Map();
	for (const msg of completeMessages) {
		if (msg.role === 'assistant' && Array.isArray(msg.parts)) {
			for (const part of msg.parts) {
				if (isToolOrDynamicToolUIPart(part) && part.state === 'output-available') {
					toolResultsMap.set(part.toolCallId, part.output);
				}
			}
		}
	}

	console.log('[saveMessages] Processing', completeMessages.length, 'UI messages');

	// Convert each UIMessage to our ChatMessage format
	for (let i = 0; i < completeMessages.length; i++) {
		const msg = completeMessages[i];
		const timestamp = new Date(now.getTime() + i).toISOString();

		if (msg.role === 'user') {
			// Extract text content from user message parts
			const textContent = Array.isArray(msg.parts)
				? msg.parts.filter(isTextUIPart).map(p => p.text).join('')
				: '';

			allMessages.push({
				role: 'user',
				content: textContent,
				timestamp,
				model: null
			});
		} else if (msg.role === 'assistant') {
			// Extract text content and tool calls from assistant message parts
			const textParts = Array.isArray(msg.parts) ? msg.parts.filter(isTextUIPart) : [];
			const toolCallParts = Array.isArray(msg.parts) ? msg.parts.filter(isToolOrDynamicToolUIPart) : [];

			const textContent = textParts.map(p => p.text).join('');

			// Extract provider from model string
			const provider = model.includes('/') ? model.split('/')[0] : 'unknown';

			allMessages.push({
				role: 'assistant',
				content: textContent,
				timestamp,
				model,
				provider,
				agentId, // Track which agent handled this message
				tool_calls: toolCallParts.length > 0
					? toolCallParts.map(tc => ({
							tool_name: tc.type.startsWith('tool-')
								? tc.type.replace('tool-', '')
								: (tc as any).toolName,
							tool_call_id: tc.toolCallId,
							arguments: (tc.input || {}) as Record<string, unknown>,
							result: tc.state === 'output-available' ? tc.output : toolResultsMap.get(tc.toolCallId),
							timestamp
						}))
					: undefined
			});
		}
	}

	console.log('[saveMessages] Converted', allMessages.length, 'messages (replacing entire conversation)');

	// Replace all messages (not append) since we're receiving the complete conversation
	const updateResult = await db
		.update(chatSessions)
		.set({
			messages: allMessages,
			updatedAt: now,
			messageCount: allMessages.length
		})
		.where(eq(chatSessions.id, sessionId))
		.returning({ messageCount: chatSessions.messageCount });

	if (updateResult.length === 0) {
		throw new Error(`Session not found: ${sessionId}`);
	}

	console.log(
		`[saveMessages] Saved ${allMessages.length} messages to session ${sessionId} (total: ${updateResult[0].messageCount})`
	);
}
