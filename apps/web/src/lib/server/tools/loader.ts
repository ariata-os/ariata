/**
 * Centralized tool loading for multi-agent system
 * Tools are loaded once at startup and cached
 */
import { z } from 'zod';
import type { Pool } from 'pg';
import { createMcpClient, type McpClient, type McpTool } from '$lib/mcp/client';
import { createLocationMapTool } from '$lib/tools/query-location-map';
import { createPursuitsTool } from '$lib/tools/query-pursuits';
import type { ToolRegistry } from './types';

/**
 * Global tool registry - loaded once at startup
 */
let cachedTools: ToolRegistry | null = null;
let mcpClient: McpClient | null = null;

/**
 * Initialize all tools at server startup
 * This should be called once when the server starts
 *
 * @param pool - Database connection pool
 * @param mcpServerUrl - URL of the MCP server
 * @returns Promise that resolves when all tools are loaded
 */
export async function initializeTools(pool: Pool, mcpServerUrl: string): Promise<void> {
	if (cachedTools) {
		console.log('[Tools] Already initialized, skipping...');
		return;
	}

	console.log('[Tools] Initializing tools...');
	const tools: ToolRegistry = {};

	try {
		// Load custom tools
		console.log('[Tools] Loading custom tools...');
		tools.queryLocationMap = await createLocationMapTool(pool);
		console.log('[Tools] ✓ Loaded queryLocationMap');
		tools.queryPursuits = await createPursuitsTool(pool);
		console.log('[Tools] ✓ Loaded queryPursuits');

		// Load MCP tools (non-blocking)
		try {
			console.log(`[Tools] Connecting to MCP server at ${mcpServerUrl}...`);
			mcpClient = await createMcpClient(mcpServerUrl);
			const mcpTools = mcpClient.getTools();

			console.log(`[Tools] Converting ${mcpTools.size} MCP tools to AI SDK format...`);
			for (const [name, mcpTool] of mcpTools) {
				tools[name] = convertMcpToolToAiSdkTool(name, mcpTool, mcpClient);
				console.log(`[Tools] ✓ Loaded ${name}`);
			}
		} catch (mcpError) {
			console.warn('[Tools] ⚠️  MCP server connection failed, continuing with custom tools only:', mcpError);
			console.warn('[Tools] The app will work but MCP tools will not be available');
		}

		cachedTools = tools;
		console.log(`[Tools] ✅ Successfully initialized ${Object.keys(tools).length} tools`);
	} catch (error) {
		console.error('[Tools] ❌ Failed to initialize tools:', error);
		throw new Error(`Tool initialization failed: ${error}`);
	}
}

/**
 * Convert MCP tool to AI SDK v6 tool format
 * @param name - Tool name
 * @param mcpTool - MCP tool definition
 * @param client - MCP client instance
 * @returns AI SDK tool
 */
function convertMcpToolToAiSdkTool(
	name: string,
	mcpTool: McpTool,
	client: McpClient
): {
	description: string;
	inputSchema: z.ZodType<any, any>;
	execute: (args: any) => Promise<any>;
} {
	// Convert JSON Schema to Zod
	const schema = mcpTool.inputSchema;
	const shape: Record<string, z.ZodTypeAny> = {};
	const required = schema?.required || [];

	for (const [key, prop] of Object.entries(schema?.properties || {})) {
		const propSchema = prop as { type: string; description?: string };
		let field: z.ZodTypeAny;

		if (propSchema.type === 'string') field = z.string();
		else if (propSchema.type === 'integer') field = z.number().int();
		else if (propSchema.type === 'number') field = z.number();
		else field = z.any();

		if (propSchema.description) field = field.describe(propSchema.description);
		if (!required.includes(key)) field = field.optional();
		shape[key] = field;
	}

	const zodSchema = z.object(shape);

	return {
		description: mcpTool.description || name,
		inputSchema: zodSchema,
		execute: async (args: z.infer<typeof zodSchema>) => {
			try {
				const result = await client.callTool(name, args);
				const textResult = result.content.map((c) => c.text).join('\n');

				// Try to parse JSON response
				try {
					return JSON.parse(textResult);
				} catch (_e) {
					// If not JSON, return as plain text
					return textResult;
				}
			} catch (error) {
				console.error(`[Tool Error] ${name} failed:`, error);

				// Return structured error instead of throwing
				// This allows the conversation to continue with error context
				return {
					success: false,
					error: error instanceof Error ? error.message : 'Unknown tool execution error',
					tool: name,
					arguments: args,
				};
			}
		},
	};
}

/**
 * Get all loaded tools
 * @returns Tool registry
 * @throws Error if tools haven't been initialized
 */
export function getTools(): ToolRegistry {
	if (!cachedTools) {
		throw new Error('Tools not initialized. Call initializeTools() first.');
	}
	return cachedTools;
}

/**
 * Check if tools are initialized
 * @returns True if tools are loaded
 */
export function areToolsInitialized(): boolean {
	return cachedTools !== null;
}

/**
 * Get MCP client instance (if needed for debugging/health checks)
 * @returns MCP client or null if not initialized
 */
export function getMcpClient(): McpClient | null {
	return mcpClient;
}

/**
 * Reinitialize tools (useful for development/testing)
 * @param pool - Database connection pool
 * @param mcpServerUrl - URL of the MCP server
 */
export async function reinitializeTools(pool: Pool, mcpServerUrl: string): Promise<void> {
	console.log('[Tools] Reinitializing tools...');

	// Close existing MCP connection if it exists
	if (mcpClient) {
		await mcpClient.close();
		mcpClient = null;
	}

	cachedTools = null;
	await initializeTools(pool, mcpServerUrl);
}

/**
 * Health check for tools
 * @returns Object with health status
 */
export function getToolsHealthStatus(): {
	initialized: boolean;
	toolCount: number;
	toolNames: string[];
	mcpConnected: boolean;
} {
	return {
		initialized: cachedTools !== null,
		toolCount: cachedTools ? Object.keys(cachedTools).length : 0,
		toolNames: cachedTools ? Object.keys(cachedTools) : [],
		mcpConnected: mcpClient !== null,
	};
}
