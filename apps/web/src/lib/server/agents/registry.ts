/**
 * Agent registry singleton
 * Manages all agent instances and provides access to them
 */
import type { ToolLoopAgent } from 'ai';
import { getTools } from '../tools/loader';
import { loadUserToolPreferences, type ToolPreferences } from '../tools/preferences';
import { getPool } from '$lib/server/db';
import { AGENT_CONFIGS, getAgentConfig, getDefaultAgentId, getEnabledAgentConfigs } from './configs';
import { createAgent } from './factory';
import type { AgentId, AgentMetadata, AgentRegistryEntry } from './types';

/**
 * Agent Registry
 * Singleton class that manages all agent instances
 */
class AgentRegistry {
	private agents: Map<AgentId, AgentRegistryEntry> = new Map();
	private initialized: boolean = false;
	private defaultUserName: string = 'User'; // Fallback username
	private defaultAssistantName: string = 'Ariata'; // Fallback assistant name
	private cacheKey: string = ''; // Cache invalidation key based on user/assistant names

	/**
	 * Initialize the agent registry
	 * Creates all enabled agents
	 *
	 * Smart caching: Only reinitializes if userName, assistantName, or tool preferences have changed
	 * This allows the server to run for years while still picking up setting changes
	 *
	 * @param userName - Default username for agent instructions (can be overridden per-request)
	 * @param assistantName - Default assistant name for agent instructions
	 */
	async initialize(userName: string = 'User', assistantName: string = 'Ariata'): Promise<void> {
		try {
			// Load tool preferences
			const toolPreferences = await loadUserToolPreferences(getPool());
			const preferencesHash = JSON.stringify(toolPreferences); // Simple hash for cache key
			const newCacheKey = `${userName}:${assistantName}:${preferencesHash}`;

			// Check if we can use cached agents (same settings)
			if (this.initialized && this.cacheKey === newCacheKey) {
				console.log('[AgentRegistry] Using cached agents (settings unchanged)');
				return;
			}

			// Settings changed or first initialization
			if (this.initialized) {
				console.log(`[AgentRegistry] Settings changed, reinitializing...`);
				this.agents.clear();
			} else {
				console.log('[AgentRegistry] Initializing agents...');
			}

			this.defaultUserName = userName;
			this.defaultAssistantName = assistantName;
			this.cacheKey = newCacheKey;

			// Get all tools
			const tools = getTools();
			console.log(`[AgentRegistry] Using ${Object.keys(tools).length} total tools`);

			// Log disabled tools if any
			const disabledTools = Object.entries(toolPreferences)
				.filter(([_, enabled]) => enabled === false)
				.map(([name, _]) => name);
			if (disabledTools.length > 0) {
				console.log(`[AgentRegistry] User disabled tools: ${disabledTools.join(', ')}`);
			}

			// Create all enabled agents
			const enabledConfigs = getEnabledAgentConfigs();
			console.log(`[AgentRegistry] Creating ${enabledConfigs.length} agents...`);

			for (const config of enabledConfigs) {
				const agent = createAgent(config, tools, userName, assistantName, toolPreferences);
				this.agents.set(config.id, {
					metadata: config,
					agent,
				});
				console.log(`[AgentRegistry] ✓ ${config.name} ready (as ${assistantName})`);
			}

			this.initialized = true;
			console.log(`[AgentRegistry] ✅ Successfully initialized ${this.agents.size} agents`);
		} catch (error) {
			console.error('[AgentRegistry] ❌ Failed to initialize agents:', error);
			throw new Error(`Agent registry initialization failed: ${error}`);
		}
	}

	/**
	 * Get an agent by ID
	 *
	 * @param agentId - Agent identifier
	 * @returns ToolLoopAgent instance
	 * @throws Error if agent not found or registry not initialized
	 */
	get(agentId: AgentId): ToolLoopAgent {
		if (!this.initialized) {
			throw new Error('Agent registry not initialized. Call initialize() first.');
		}

		const entry = this.agents.get(agentId);
		if (!entry) {
			throw new Error(`Agent not found: ${agentId}. Available agents: ${this.getAvailableAgentIds().join(', ')}`);
		}

		return entry.agent;
	}

	/**
	 * Get agent metadata
	 *
	 * @param agentId - Agent identifier
	 * @returns Agent metadata
	 */
	getMetadata(agentId: AgentId): AgentMetadata | undefined {
		const entry = this.agents.get(agentId);
		return entry?.metadata;
	}

	/**
	 * Get all agent metadata (for UI)
	 *
	 * @returns Array of all agent metadata
	 */
	getAllMetadata(): AgentMetadata[] {
		return Array.from(this.agents.values()).map((entry) => entry.metadata);
	}

	/**
	 * Get default agent
	 *
	 * @returns Default agent (general)
	 */
	getDefault(): ToolLoopAgent {
		const defaultId = getDefaultAgentId() as AgentId;
		return this.get(defaultId);
	}

	/**
	 * Get default agent ID
	 *
	 * @returns Default agent ID
	 */
	getDefaultId(): AgentId {
		return getDefaultAgentId() as AgentId;
	}

	/**
	 * Check if an agent exists
	 *
	 * @param agentId - Agent identifier
	 * @returns True if agent exists
	 */
	has(agentId: string): boolean {
		return this.agents.has(agentId as AgentId);
	}

	/**
	 * Get list of available agent IDs
	 *
	 * @returns Array of agent IDs
	 */
	getAvailableAgentIds(): AgentId[] {
		return Array.from(this.agents.keys());
	}

	/**
	 * Check if registry is initialized
	 *
	 * @returns True if initialized
	 */
	isInitialized(): boolean {
		return this.initialized;
	}

	/**
	 * Get health status of agent registry
	 *
	 * @returns Health status object
	 */
	getHealthStatus(): {
		initialized: boolean;
		agentCount: number;
		agents: Array<{ id: AgentId; name: string; enabled: boolean }>;
	} {
		return {
			initialized: this.initialized,
			agentCount: this.agents.size,
			agents: Array.from(this.agents.values()).map((entry) => ({
				id: entry.metadata.id,
				name: entry.metadata.name,
				enabled: entry.metadata.enabled,
			})),
		};
	}

	/**
	 * Reinitialize agents (useful for development/testing)
	 *
	 * @param userName - Username for agent instructions
	 * @param assistantName - Assistant name for agent instructions
	 */
	async reinitialize(userName: string = 'User', assistantName: string = 'Ariata'): Promise<void> {
		console.log('[AgentRegistry] Reinitializing agents...');
		this.agents.clear();
		this.initialized = false;
		await this.initialize(userName, assistantName);
	}
}

/**
 * Singleton instance of the agent registry
 */
export const agentRegistry = new AgentRegistry();

/**
 * Initialize the agent registry (called from server startup)
 *
 * @param userName - Default username
 * @param assistantName - Default assistant name
 */
export async function initializeAgents(userName: string = 'User', assistantName: string = 'Ariata'): Promise<void> {
	await agentRegistry.initialize(userName, assistantName);
}

/**
 * Get an agent by ID
 *
 * @param agentId - Agent identifier or 'auto' for default
 * @returns ToolLoopAgent instance
 */
export function getAgent(agentId: string): ToolLoopAgent {
	// Handle 'auto' mode - return default agent
	if (agentId === 'auto') {
		return agentRegistry.getDefault();
	}

	return agentRegistry.get(agentId as AgentId);
}

/**
 * Get agent metadata
 *
 * @param agentId - Agent identifier
 * @returns Agent metadata
 */
export function getAgentMetadata(agentId: string): AgentMetadata | undefined {
	if (agentId === 'auto') {
		return agentRegistry.getMetadata(agentRegistry.getDefaultId());
	}
	return agentRegistry.getMetadata(agentId as AgentId);
}

/**
 * Get all agent metadata for UI
 *
 * @returns Array of agent metadata
 */
export function getAllAgentMetadata(): AgentMetadata[] {
	return agentRegistry.getAllMetadata();
}
