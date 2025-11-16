/**
 * Intent-based routing system
 * Determines which agent should handle a query based on keywords and context
 */
import type { AgentId } from '../agents/types';
import { getDefaultAgentId } from '../agents/configs';
import type { UIMessage } from 'ai';

/**
 * Keywords that indicate different agent intents
 */
const AGENT_KEYWORDS: Record<AgentId, string[]> = {
	analytics: [
		// Location keywords
		'location',
		'map',
		'where',
		'place',
		'geograph',
		'spatial',
		'coordinate',
		'latitude',
		'longitude',
		// Data exploration keywords
		'data',
		'table',
		'schema',
		'explore',
		'structure',
		'database',
		'query',
		'sql',
		// Pattern keywords
		'pattern',
		'trend',
		'analysis',
		'metric',
		'statistic',
		'visualiz',
		'chart',
		'graph',
	],
	research: [
		// Memory keywords
		'remember',
		'recall',
		'memory',
		'memor',
		// Narrative keywords
		'narrative',
		'story',
		'happened',
		'event',
		'experience',
		// Connection keywords
		'learn',
		'thought',
		'idea',
		'connect',
		'relate',
		'similar',
		// People keywords
		'meet',
		'met',
		'conversation',
		'talk',
		'discuss',
		// Values keywords
		'value',
		'belief',
		'goal',
		'virtue',
		'habit',
		'temperament',
		'preference',
		'philosophy',
	],
	general: [], // General is the fallback
	action: [
		'sync',
		'refresh',
		'update',
		'trigger',
		'reload',
		'fetch',
	],
};

/**
 * Routing context for making routing decisions
 */
export interface RoutingContext {
	/** Current user message */
	message: string;

	/** Recent conversation history (last 3-5 messages) */
	recentMessages?: UIMessage[];

	/** Last agent used (for sticky behavior) */
	lastAgentId?: AgentId;

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

	// 2. Score each agent based on keyword matching
	const scores = scoreAgents(context.message);

	// 3. Find highest scoring agent
	const entries = Object.entries(scores) as [AgentId, number][];
	const sorted = entries.sort((a, b) => b[1] - a[1]);
	const [topAgent, topScore] = sorted[0];

	// 4. Apply threshold - if score is too low, use general agent
	const CONFIDENCE_THRESHOLD = 0.3;
	if (topScore < CONFIDENCE_THRESHOLD) {
		return {
			agentId: getDefaultAgentId() as AgentId,
			confidence: 0.5,
			reason: 'No clear intent detected, using general agent',
			explicit: false,
		};
	}

	// 5. Return routing decision
	return {
		agentId: topAgent,
		confidence: topScore,
		reason: `Intent detected: ${topAgent} (score: ${topScore.toFixed(2)})`,
		explicit: false,
	};
}

/**
 * Score each agent based on keyword presence in the message
 *
 * @param message - User message
 * @returns Object mapping agent IDs to scores (0-1)
 */
function scoreAgents(message: string): Record<AgentId, number> {
	const lowerMessage = message.toLowerCase();
	const scores: Record<AgentId, number> = {
		analytics: 0,
		research: 0,
		general: 0,
		action: 0,
	};

	// Count keyword matches for each agent
	for (const [agentId, keywords] of Object.entries(AGENT_KEYWORDS)) {
		let matchCount = 0;
		for (const keyword of keywords) {
			if (lowerMessage.includes(keyword)) {
				matchCount++;
			}
		}

		// Normalize score by number of keywords for that agent
		if (keywords.length > 0) {
			scores[agentId as AgentId] = matchCount / keywords.length;
		}
	}

	return scores;
}

/**
 * Get routing suggestions for a message (useful for debugging)
 *
 * @param message - User message
 * @returns Ranked list of agents with scores
 */
export function getRoutingSuggestions(
	message: string
): Array<{ agentId: AgentId; score: number }> {
	const scores = scoreAgents(message);
	const entries = Object.entries(scores) as [AgentId, number][];
	return entries
		.sort((a, b) => b[1] - a[1])
		.map(([agentId, score]) => ({ agentId, score }));
}

/**
 * Validate agent ID
 *
 * @param agentId - Agent ID to validate
 * @returns True if valid agent ID
 */
export function isValidAgentId(agentId: string): agentId is AgentId {
	return ['analytics', 'research', 'general', 'action', 'auto'].includes(agentId);
}
