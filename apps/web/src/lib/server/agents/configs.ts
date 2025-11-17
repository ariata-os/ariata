/**
 * Agent configuration definitions
 */
import type { AgentMetadata } from './types';

/**
 * Agent configurations
 * Simplified 2-agent system:
 * - agent: Has all tools, can perform actions
 * - chat: No tools, simple conversation only
 */
export const AGENT_CONFIGS: AgentMetadata[] = [
	{
		id: 'agent',
		name: 'Agent',
		description: 'Intelligent assistant with access to all available tools. Can query data, search the web, visualize information, and help with tasks.',
		color: '#6b7280', // Gray
		icon: '🤖',
		defaultModel: 'anthropic/claude-sonnet-4.5',
		maxSteps: 5,
		enabled: true,
	},
	{
		id: 'chat',
		name: 'Chat',
		description: 'Simple conversational assistant without tool access. Best for quick questions and general conversation.',
		color: '#64748b', // Slate
		icon: '💬',
		defaultModel: 'anthropic/claude-sonnet-4.5',
		maxSteps: 1, // No tools, so only one step
		enabled: true,
	},
];

/**
 * Get agent configuration by ID
 * @param id - Agent ID
 * @returns Agent metadata or undefined if not found
 */
export function getAgentConfig(id: string): AgentMetadata | undefined {
	return AGENT_CONFIGS.find((config) => config.id === id);
}

/**
 * Get all enabled agent configurations
 * @returns Array of enabled agent metadata
 */
export function getEnabledAgentConfigs(): AgentMetadata[] {
	return AGENT_CONFIGS.filter((config) => config.enabled);
}

/**
 * Get default agent ID
 * @returns ID of the default agent
 */
export function getDefaultAgentId(): string {
	return 'agent';
}
