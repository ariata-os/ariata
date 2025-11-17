/**
 * Simplified routing system for 2-agent architecture
 * - agent: Has tools, handles all queries
 * - chat: No tools, simple conversation
 */
import type { AgentId } from '../agents/types';
import { getDefaultAgentId } from '../agents/configs';
import type { UIMessage } from 'ai';

/**
 * Routing context for making routing decisions
 */
export interface RoutingContext {
	/** Current user message */
	message: string;

	/** Recent conversation history (last 3-5 messages) */
	recentMessages?: UIMessage[];

	/** User's explicit agent preference (if any) */
	explicitAgentId?: AgentId;
}

/**
 * Routing result
 */
export interface RoutingResult {
	/** Selected agent ID */
	agentId: AgentId;

	/** Confidence score (0-1) */
	confidence: number;

	/** Reason for selection */
	reason: string;

	/** Whether this was an explicit user choice */
	explicit: boolean;
}

/**
 * Route a message to the appropriate agent
 *
 * With 2 agents, routing is simple:
 * - Explicit selection takes precedence
 * - Otherwise default to 'agent' (with tools)
 *
 * @param context - Routing context
 * @returns Routing result with selected agent
 */
export function routeToAgent(context: RoutingContext): RoutingResult {
	// 1. Check for explicit agent selection
	if (context.explicitAgentId) {
		return {
			agentId: context.explicitAgentId,
			confidence: 1.0,
			reason: 'User explicitly selected agent',
			explicit: true,
		};
	}

	// 2. Default to agent with tools
	// In simplified 2-agent system, we always use 'agent' unless user explicitly chooses 'chat'
	return {
		agentId: getDefaultAgentId() as AgentId,
		confidence: 1.0,
		reason: 'Using default agent with tools',
		explicit: false,
	};
}

/**
 * Validate agent ID
 *
 * @param agentId - Agent ID to validate
 * @returns True if valid agent ID
 */
export function isValidAgentId(agentId: string): agentId is AgentId {
	return ['agent', 'chat', 'auto'].includes(agentId);
}
