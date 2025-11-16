/**
 * Agent factory for creating ToolLoopAgent instances
 */
import { ToolLoopAgent, stepCountIs } from 'ai';
import type { AgentMetadata } from './types';
import { buildInstructions } from './instructions';
import { getToolsForAgent } from '../tools/categories';
import { filterToolsByPreferences, type ToolPreferences } from '../tools/preferences';
import type { ToolRegistry } from '../tools/types';

/**
 * Create a ToolLoopAgent from metadata
 *
 * @param metadata - Agent configuration
 * @param allTools - All available tools
 * @param userName - User's name for personalization
 * @param assistantName - Assistant's configured name
 * @param toolPreferences - Optional user tool preferences for filtering
 * @returns Configured ToolLoopAgent instance
 */
export function createAgent(
	metadata: AgentMetadata,
	allTools: ToolRegistry,
	userName: string,
	assistantName: string = 'Ariata',
	toolPreferences?: ToolPreferences
): ToolLoopAgent {
	// Filter tools based on agent's allowed categories
	let agentTools = getToolsForAgent(metadata.toolCategories, allTools);

	// Apply user preferences filtering if provided
	if (toolPreferences) {
		agentTools = filterToolsByPreferences(agentTools, toolPreferences);
	}

	// Build agent instructions
	const instructions = buildInstructions(metadata.id, userName, assistantName);

	// Create ToolLoopAgent
	const agent = new ToolLoopAgent({
		id: `ariata-${metadata.id}`,
		model: metadata.defaultModel,
		instructions,
		tools: agentTools,
		stopWhen: stepCountIs(metadata.maxSteps || 5),
		maxRetries: 3,
	});

	console.log(`[AgentFactory] Created ${metadata.name}`);
	console.log(`[AgentFactory]   - Model: ${metadata.defaultModel}`);
	console.log(`[AgentFactory]   - Tools: ${Object.keys(agentTools).length}`);
	console.log(`[AgentFactory]   - Max steps: ${metadata.maxSteps || 5}`);

	return agent;
}
