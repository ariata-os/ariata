/**
 * Tool categorization and filtering for multi-agent system
 */
import { ToolCategory, type ToolMetadata } from './types';

/**
 * Tool categorization mapping
 * This defines which tools belong to which categories
 */
export const TOOL_CATEGORIES: Record<string, ToolCategory> = {
	// Shared tools (all agents have access)
	ariata_query_ontology: ToolCategory.SHARED,
	ariata_list_ontology_tables: ToolCategory.SHARED,
	ariata_get_table_schema: ToolCategory.SHARED,

	// Analytics-specific tools
	queryLocationMap: ToolCategory.ANALYTICS,
	queryPursuits: ToolCategory.ANALYTICS,

	// Research-specific tools
	ariata_query_narratives: ToolCategory.RESEARCH,
	ariata_query_axiology: ToolCategory.RESEARCH,

	// Action tools
	ariata_trigger_sync: ToolCategory.ACTION,
};

/**
 * Tool metadata for UI and documentation
 */
export const TOOL_METADATA: Record<string, ToolMetadata> = {
	queryLocationMap: {
		name: 'queryLocationMap',
		category: ToolCategory.ANALYTICS,
		description: 'Query location data and generate interactive map visualizations',
	},
	queryPursuits: {
		name: 'queryPursuits',
		category: ToolCategory.ANALYTICS,
		description: 'Query temporal pursuits (tasks, initiatives, aspirations) and display them in a unified widget',
	},
	ariata_query_narratives: {
		name: 'ariata_query_narratives',
		category: ToolCategory.RESEARCH,
		description: 'Search pre-synthesized biographical narratives with semantic search',
	},
	ariata_query_axiology: {
		name: 'ariata_query_axiology',
		category: ToolCategory.RESEARCH,
		description: 'Query values, goals, virtues, vices, habits, and preferences',
	},
	ariata_query_ontology: {
		name: 'ariata_query_ontology',
		category: ToolCategory.SHARED,
		description: 'Execute SQL queries on ontology tables for raw data access',
	},
	ariata_list_ontology_tables: {
		name: 'ariata_list_ontology_tables',
		category: ToolCategory.SHARED,
		description: 'List all available ontology tables and their descriptions',
	},
	ariata_get_table_schema: {
		name: 'ariata_get_table_schema',
		category: ToolCategory.SHARED,
		description: 'Get detailed schema for a specific ontology table',
	},
	ariata_trigger_sync: {
		name: 'ariata_trigger_sync',
		category: ToolCategory.ACTION,
		description: 'Trigger manual data synchronization for a source',
	},
};

/**
 * Get tools for a specific category
 * @param category - Tool category to filter by
 * @param allTools - All available tools
 * @returns Filtered tools for the category
 */
export function getToolsForCategory(
	category: ToolCategory,
	allTools: Record<string, any>
): Record<string, any> {
	const filtered: Record<string, any> = {};

	for (const [toolName, tool] of Object.entries(allTools)) {
		const toolCategory = TOOL_CATEGORIES[toolName];
		if (toolCategory === category) {
			filtered[toolName] = tool;
		}
	}

	return filtered;
}

/**
 * Get tools for an agent by combining categories
 * @param categories - Array of categories this agent has access to
 * @param allTools - All available tools
 * @returns Filtered tools for the agent
 */
export function getToolsForAgent(
	categories: ToolCategory[],
	allTools: Record<string, any>
): Record<string, any> {
	const filtered: Record<string, any> = {};

	for (const [toolName, tool] of Object.entries(allTools)) {
		const toolCategory = TOOL_CATEGORIES[toolName];
		if (categories.includes(toolCategory)) {
			filtered[toolName] = tool;
		}
	}

	return filtered;
}

/**
 * Get all tools (for general agent)
 */
export function getAllTools(allTools: Record<string, any>): Record<string, any> {
	return allTools;
}
