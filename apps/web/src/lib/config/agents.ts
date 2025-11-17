/**
 * Agent UI metadata
 * Shared configuration for agent display in the UI
 *
 * NOTE: This is a CLIENT-SIDE config file.
 * Keep in sync with server configs at lib/server/agents/configs.ts
 */

/**
 * UI-safe agent metadata
 * Simplified 2-agent system:
 * - agent: Has all tools, handles queries requiring data/actions
 * - chat: No tools, simple conversation only
 */
export const AGENT_UI_METADATA = [
	{
		id: 'agent',
		name: 'Agent',
		description: 'Intelligent assistant with access to all available tools. Can query data, search the web, visualize information, and help with tasks.',
		color: '#6b7280',
		icon: 'ri:robot-line',
		enabled: true,
	},
	{
		id: 'chat',
		name: 'Chat',
		description: 'Simple conversational assistant without tool access. Best for quick questions and general conversation.',
		color: '#64748b',
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
