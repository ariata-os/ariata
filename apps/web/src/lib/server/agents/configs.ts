/**
 * Agent configuration definitions
 */
import type { AgentMetadata } from './types';
import { ToolCategory } from '../tools/types';

/**
 * Agent configurations
 * These define the metadata and capabilities of each specialized agent
 */
export const AGENT_CONFIGS: AgentMetadata[] = [
	{
		id: 'analytics',
		name: 'Analytics Agent',
		description: 'Analyzes location patterns, data visualizations, and explores data structures. Best for spatial analysis, data exploration, and understanding what data is available.',
		color: '#3b82f6', // Blue
		icon: '📊',
		defaultModel: 'openai/gpt-oss-120b', // Fast, good for analytical tasks
		toolCategories: [ToolCategory.SHARED, ToolCategory.ANALYTICS],
		maxSteps: 5,
		enabled: true,
	},
	{
		id: 'research',
		name: 'Research Agent',
		description: 'Searches narratives, explores memories, and connects ideas. Best for biographical questions, semantic search, and understanding values and beliefs.',
		color: '#8b5cf6', // Purple
		icon: '🔍',
		defaultModel: 'anthropic/claude-opus-4.1', // Deep reasoning for research
		toolCategories: [ToolCategory.SHARED, ToolCategory.RESEARCH],
		maxSteps: 7, // May need more steps for deep research
		enabled: true,
	},
	{
		id: 'general',
		name: 'General Assistant',
		description: 'Versatile assistant with access to all tools. Best for general conversation, unclear intents, and multi-domain queries.',
		color: '#6b7280', // Gray
		icon: '💬',
		defaultModel: 'anthropic/claude-sonnet-4.5', // Balanced, good all-arounder
		toolCategories: [
			ToolCategory.SHARED,
			ToolCategory.ANALYTICS,
			ToolCategory.RESEARCH,
			ToolCategory.ACTION,
		],
		maxSteps: 5,
		enabled: true,
	},
	{
		id: 'action',
		name: 'Action Agent',
		description: 'Handles system operations like data synchronization. Best for maintenance tasks and system operations.',
		color: '#10b981', // Green
		icon: '⚡',
		defaultModel: 'openai/gpt-oss-120b', // Fast, reliable
		toolCategories: [ToolCategory.SHARED, ToolCategory.ACTION],
		maxSteps: 3, // Actions are usually straightforward
		enabled: false, // Not enabled yet - coming soon
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
 * @returns ID of the default agent (general)
 */
export function getDefaultAgentId(): string {
	return 'general';
}
