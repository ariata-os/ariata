/**
 * Agent type definitions for multi-agent system
 */
import type { ToolLoopAgent } from 'ai';
import type { ToolCategory } from '../tools/types';

/**
 * Agent identifiers
 */
export type AgentId = 'analytics' | 'research' | 'general' | 'action';

/**
 * Agent metadata for UI and configuration
 */
export interface AgentMetadata {
	/** Unique agent identifier */
	id: AgentId;

	/** Human-readable name */
	name: string;

	/** Description of agent's purpose and capabilities */
	description: string;

	/** Color for UI borders and visual identification (hex code) */
	color: string;

	/** Emoji icon for UI (optional) */
	icon?: string;

	/** Default model for this agent */
	defaultModel: string;

	/** Tool categories this agent has access to */
	toolCategories: ToolCategory[];

	/** Maximum number of agentic steps */
	maxSteps?: number;

	/** Whether this agent is currently enabled */
	enabled: boolean;
}

/**
 * Agent registry entry
 */
export interface AgentRegistryEntry {
	metadata: AgentMetadata;
	agent: ToolLoopAgent;
}
