/**
 * Agent UI metadata
 * Shared configuration for agent display in the UI
 *
 * NOTE: This is a CLIENT-SIDE config file.
 * Keep in sync with server configs at lib/server/agents/configs.ts
 */

/**
 * UI-safe agent metadata
 * Excludes sensitive backend details like tool categories and default models
 */
export const AGENT_UI_METADATA = [
	{
		id: 'analytics',
		name: 'Analytics',
		description: 'Specializes in data exploration, location analysis, and visualizations',
		color: '#3b82f6',
		icon: 'ri:bar-chart-line',
		enabled: true,
	},
	{
		id: 'research',
		name: 'Research',
		description: 'Focuses on narratives, semantic search, values, and connecting ideas',
		color: '#8b5cf6',
		icon: 'ri:book-open-line',
		enabled: true,
	},
	{
		id: 'general',
		name: 'General',
		description: 'Adaptive assistant for general queries and mixed tasks',
		color: '#6b7280',
		icon: 'ri:chat-3-line',
		enabled: true,
	},
] as const;

/**
 * Type for agent UI metadata
 */
export type AgentUIMetadata = (typeof AGENT_UI_METADATA)[number];

/**
 * Get agent metadata by ID
 * @param id - Agent ID
 * @returns Agent metadata or undefined if not found
 */
export function getAgentUIMetadata(id: string): AgentUIMetadata | undefined {
	return AGENT_UI_METADATA.find((agent) => agent.id === id);
}

/**
 * Get all enabled agents for UI display
 * @returns Array of enabled agent metadata
 */
export function getEnabledAgents(): AgentUIMetadata[] {
	return AGENT_UI_METADATA.filter((agent) => agent.enabled);
}
