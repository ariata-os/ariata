/**
 * SvelteKit server hooks
 * Runs once when the server starts
 */
import { env } from '$env/dynamic/private';
import { getPool } from '$lib/server/db';
import { initializeTools } from '$lib/server/tools/loader';
import { ApiClient } from '$lib/server/apiClient';

/**
 * Server initialization
 * Runs once when the SvelteKit server starts
 */
let initialized = false;

async function initializeServer() {
	if (initialized) {
		console.log('[Server] Already initialized, skipping...');
		return;
	}

	console.log('[Server] ========================================');
	console.log('[Server] Starting Ariata Web Server');
	console.log('[Server] ========================================');

	try {
		// Initialize database pool
		console.log('[Server] Initializing database connection...');
		const pool = getPool();
		console.log('[Server] ✓ Database connection ready');

		// Initialize tools
		const mcpServerUrl = env.RUST_API_URL || 'http://localhost:8000';
		console.log(`[Server] Initializing tools with MCP server at ${mcpServerUrl}...`);
		await initializeTools(pool, `${mcpServerUrl}/mcp`);
		console.log('[Server] ✓ Tools initialized');

		// Initialize agent registry
		console.log('[Server] Initializing agent registry...');
		const { initializeAgents } = await import('$lib/server/agents/registry');
		await initializeAgents('User'); // Default username, will be overridden per-request
		console.log('[Server] ✓ Agent registry ready');

		initialized = true;
		console.log('[Server] ========================================');
		console.log('[Server] ✅ Ariata Web Server Ready');
		console.log('[Server] ========================================');
	} catch (error) {
		console.error('[Server] ❌ Failed to initialize server:', error);
		throw error;
	}
}

/**
 * Handle hook for incoming requests
 * This runs on every request, but initialization only happens once
 */
export async function handle({ event, resolve }) {
	// Initialize on first request (in dev mode, this runs per request until stable)
	if (!initialized) {
		await initializeServer();
	}

	// Attach API client to locals for use in route handlers
	event.locals.apiClient = new ApiClient();

	// Continue with request handling
	return resolve(event);
}
