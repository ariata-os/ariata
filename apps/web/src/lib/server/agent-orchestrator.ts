import { createAnthropic } from '@ai-sdk/anthropic';
import { streamText, type ModelMessage } from 'ai';
import { env } from '$env/dynamic/private';
import type { Pool } from 'pg';
import { z } from 'zod';
import { createMcpClient } from '$lib/mcp/client';

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

// Get Anthropic instance with runtime env
function getAnthropic() {
	const apiKey = env.ANTHROPIC_API_KEY;
	if (!apiKey) {
		throw new Error('ANTHROPIC_API_KEY environment variable is not set');
	}
	return createAnthropic({ apiKey });
}

export async function orchestrateChat(params: OrchestrationParams) {
	const { messages, model, pool, userName, onFinish } = params;

	// Connect to MCP server and load tools
	const mcpServerUrl = env.RUST_API_URL || 'http://localhost:8000';
	const mcpClient = await createMcpClient(`${mcpServerUrl}/mcp`);
	const mcpTools = mcpClient.getTools();
	const tools: Record<string, any> = {};

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
				const result = await mcpClient.callTool(name, args);
				const textResult = result.content.map(c => c.text).join('\n');

				// Parse the JSON response from MCP server
				// The MCP server returns JSON as text, so we need to parse it
				try {
					return JSON.parse(textResult);
				} catch (e) {
					// If not JSON, return as plain text in a wrapper object
					return {
						success: true,
						rawOutput: textResult
					};
				}
			}
		};
	}

	const systemPrompt = `

---

You are Ariata, a personal AI assistant for ${userName}.

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
1. Use queryLocationMap (always use YYYY-MM-DD format for dates)

**Tools Available:**
- **ariata_query_narratives**: Pre-synthesized biographical narratives (USE FIRST for "what happened" questions)
- **ariata_query_ontology**: Raw SQL queries on ontology tables (use when narratives don't exist or for specific metrics)
- **ariata_query_axiology**: Query values, telos, goals, virtues, vices, habits, temperaments, preferences
- **ariata_list_ontology_tables**: Discover available tables and schemas
- **ariata_get_table_schema**: Get detailed schema for a specific table
- **ariata_trigger_sync**: Trigger manual data sync for a source
- **queryLocationMap**: Geographic visualizations of location data (custom tool)

Always use appropriate date filters and SQL LIMIT clauses in queries.`;

	// Use streamText with MCP tools
	const anthropic = getAnthropic();
	const result = await streamText({
		model: anthropic(model),
		system: systemPrompt,
		messages,
		tools,
		onFinish: async (event) => {
			console.log('[Orchestrator] Chat finished');
			if (onFinish) {
				await onFinish(event);
			}
		}
	});

	return result;
}
