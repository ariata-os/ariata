/**
 * Tool metadata for UI and documentation
 * Simplified for 2-agent system - no category-based filtering needed
 */
import type { ToolMetadata } from './types';

/**
 * Tool metadata for UI and documentation
 * Provides display names, descriptions, and other UI hints
 */
export const TOOL_METADATA: Record<string, ToolMetadata> = {
	query_location_map: {
		name: 'query_location_map',
		description: 'Query location data and generate interactive map visualizations',
	},
	web_search: {
		name: 'web_search',
		description: 'Search the web using Exa AI for recent information, research, and domain knowledge',
	},
	ariata_query_narratives: {
		name: 'ariata_query_narratives',
		description: 'Search pre-synthesized biographical narratives with semantic search',
	},
	ariata_query_axiology: {
		name: 'ariata_query_axiology',
		description: 'Query values, goals, virtues, vices, habits, and preferences',
	},
	ariata_query_ontology: {
		name: 'ariata_query_ontology',
		description: 'Unified ontology tool: query data (execute SQL), list tables (discover available tables), or get schema (view table structure)',
	},
};

/**
 * Get tool metadata by name
 * @param toolName - Tool name
 * @returns Tool metadata or undefined if not found
 */
export function getToolMetadata(toolName: string): ToolMetadata | undefined {
	return TOOL_METADATA[toolName];
}

/**
 * Get all tool metadata
 * @returns Array of all tool metadata
 */
export function getAllToolMetadata(): ToolMetadata[] {
	return Object.values(TOOL_METADATA);
}
