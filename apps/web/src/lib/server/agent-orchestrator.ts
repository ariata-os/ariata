import { streamText, stepCountIs, type ModelMessage } from 'ai';
import { env } from '$env/dynamic/private';
import type { Pool } from 'pg';
import { z } from 'zod';
import { createMcpClient } from '$lib/mcp/client';
import { createLocationMapTool } from '$lib/tools/query-location-map';

/**
 * Parameters for orchestrating chat
 */
export interface OrchestrationParams {
	messages: ModelMessage[];
	model: string;
	pool: Pool;
	sessionId: string;
	userName: string;
	onStepFinish?: (stepResult: any) => Promise<void>;
	onFinish?: (event: any) => Promise<void>;
}

export async function orchestrateChat(params: OrchestrationParams) {
	const { messages, model, pool, userName, onFinish } = params;

	// Connect to MCP server and load tools
	const mcpServerUrl = env.RUST_API_URL || 'http://localhost:8000';
	const mcpClient = await createMcpClient(`${mcpServerUrl}/mcp`);
	const mcpTools = mcpClient.getTools();
	const tools: Record<string, any> = {};

	// Fetch user profile for system prompt
	let displayName = userName;
	try {
		const profileResponse = await fetch(`${mcpServerUrl}/api/profile`);
		if (profileResponse.ok) {
			const profile = await profileResponse.json();
			// Use preferred_name if set, otherwise full_name, otherwise fallback to userName
			displayName = profile.preferred_name || profile.full_name || userName;
		}
	} catch (error) {
		console.error('[Orchestrator] Failed to fetch user profile:', error);
		// Continue with userName fallback
	}

	// Add custom queryLocationMap tool
	tools.queryLocationMap = await createLocationMapTool(pool);

	// Convert MCP tools to AI SDK tools (v6 format - plain objects)
	for (const [name, mcpTool] of mcpTools) {
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

		// AI SDK v6 tools are plain objects with description, inputSchema, execute
		tools[name] = {
			description: mcpTool.description || name,
			inputSchema: zodSchema,
			execute: async (args: z.infer<typeof zodSchema>) => {
				try {
					const result = await mcpClient.callTool(name, args);
					const textResult = result.content.map(c => c.text).join('\n');

					// Parse the JSON response from MCP server
					// Return the parsed data directly for the model to use in multi-step calls
					try {
						return JSON.parse(textResult);
					} catch (_e) {
						// If not JSON, return as plain text
						return textResult;
					}
				} catch (error) {
					// Log tool execution error and return structured error message
					console.error(`[Tool Error] ${name} failed:`, error);

					// Return error information instead of throwing
					// This allows the conversation to continue with error context
					return {
						success: false,
						error: error instanceof Error ? error.message : 'Unknown tool execution error',
						tool: name,
						arguments: args
					};
				}
			}
		};
	}

	const systemPrompt = `

---

You are Ariata, a personal AI assistant for ${displayName}.

Today's date is ${new Date().toISOString().split('T')[0]} (YYYY-MM-DD format). Use this as reference when interpreting relative dates like "today", "yesterday", etc.

## Available Tools & Routing Strategy

**CRITICAL - Tool Selection Priority:**

For biographical questions ("who did I meet", "what happened", "what did I do", "where was I"):
1. **FIRST CALL: ariata_query_narratives** - This contains pre-synthesized prose summaries with ALL context
2. If narratives exist → Answer directly from narratives
3. ONLY if narratives are empty → Then explore with other tools

For metric/specific data questions ("exact heart rate at 3pm", "all step counts in November"):
1. Use ariata_query_ontology directly with SQL

For exploratory questions ("what data do you have", "what tables exist"):
1. Use ariata_list_ontology_tables

For values/goals/habits questions:
1. Use ariata_query_axiology

For geographic visualizations:
1. Use queryLocationMap with startDate and endDate parameters (both in YYYY-MM-DD format)
   - For a single day: set both startDate and endDate to the same date (e.g., "2025-11-10")
   - For a date range: set startDate to start, endDate to end

**Tools Available:**
- **ariata_query_narratives**: Pre-synthesized biographical narratives (USE FIRST for "what happened" questions)
- **ariata_query_ontology**: Raw SQL queries on ontology tables (use when narratives don't exist or for specific metrics)
- **ariata_query_axiology**: Query values, telos, goals, virtues, vices, habits, temperaments, preferences
- **ariata_list_ontology_tables**: Discover available tables and schemas
- **ariata_get_table_schema**: Get detailed schema for a specific table
- **ariata_trigger_sync**: Trigger manual data sync for a source
- **queryLocationMap**: Geographic visualizations of location data with startDate/endDate (YYYY-MM-DD format)

Always use appropriate date filters and SQL LIMIT clauses in queries.

## CRITICAL - Response Requirements

After calling ANY tool, you MUST:
1. Wait for the tool result
2. Analyze the data returned
3. Provide a clear, natural language response to the user based on the tool results
4. NEVER stop after just calling a tool - always follow up with your interpretation

When narratives are returned, present them in a conversational, helpful way. Don't just acknowledge the tool call - actually answer the user's question using the data you received.`;

	// Use streamText with MCP tools
	// AI Gateway will automatically be used when model string is in format "provider/model"
	// The AI SDK will use AI_GATEWAY_API_KEY environment variable
	const result = await streamText({
		model,
		system: systemPrompt,
		messages,
		tools,
		stopWhen: stepCountIs(5), // Enable multi-step tool calling - stop after max 5 steps
		maxRetries: 3, // Retry failed API calls up to 3 times (helps with transient failures)
		onFinish: async (event) => {
			console.log('[Orchestrator] Chat finished');
			if (onFinish) {
				await onFinish(event);
			}
		},
		onError: ({ error }) => {
			console.error('[Orchestrator] Stream error:', error);
			// Error will be passed to toUIMessageStreamResponse onError handler
		}
	});

	return result;
}
