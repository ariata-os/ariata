/**
 * Tool preferences management
 *
 * Handles loading and filtering tools based on user preferences.
 * Only specific UI widget tools are controlled by preferences.
 * MCP tools and system tools are always enabled.
 */
import type { Pool } from 'pg';

/**
 * Tools that can be controlled via user preferences
 * Other tools (MCP tools, system tools) are always enabled
 */
export const PREFERENCE_CONTROLLED_TOOLS = ['query_location_map', 'web_search'] as const;

/**
 * Tool preferences object
 * - Only contains preferences for UI widget tools
 * - Tools set to true are enabled
 * - Tools set to false are disabled
 *
 */
export type ToolPreferences = Record<string, boolean>;

/**
 * Default tool preferences - all preference-controlled tools enabled by default
 * When new tools are added, they'll automatically be enabled for existing users
 */
export const DEFAULT_TOOL_PREFERENCES: ToolPreferences = {
	query_location_map: true,
	web_search: true
};

/**
 * Load user tool preferences from database
 * Auto-migrates new tools by merging with defaults
 * @param pool - Database connection pool
 * @returns Tool preferences object with explicit true/false values
 */
export async function loadUserToolPreferences(pool: Pool): Promise<ToolPreferences> {
	try {
		const result = await pool.query(
			'SELECT enabled_tools FROM app.assistant_profile LIMIT 1'
		);

		const enabledTools = result.rows[0]?.enabled_tools;

		// Return default preferences if null/undefined
		if (!enabledTools || typeof enabledTools !== 'object') {
			return { ...DEFAULT_TOOL_PREFERENCES };
		}

		// Merge defaults with user preferences
		// This ensures new tools are auto-enabled while preserving user choices
		return {
			...DEFAULT_TOOL_PREFERENCES,
			...enabledTools
		};
	} catch (error) {
		console.error('[ToolPreferences] Failed to load preferences:', error);
		// On error, default to all tools enabled
		return { ...DEFAULT_TOOL_PREFERENCES };
	}
}

/**
 * Filter tools based on user preferences
 * Only filters preference-controlled tools (UI widgets)
 * MCP and system tools are always included
 * @param tools - All available tools
 * @param preferences - User tool preferences
 * @returns Filtered tools (only enabled ones)
 */
export function filterToolsByPreferences(
	tools: Record<string, any>,
	preferences: ToolPreferences
): Record<string, any> {
	const filtered: Record<string, any> = {};

	for (const [toolName, tool] of Object.entries(tools)) {
		// Check if this tool is preference-controlled
		if (PREFERENCE_CONTROLLED_TOOLS.includes(toolName as any)) {
			// Only include if explicitly enabled
			if (preferences[toolName] === true) {
				filtered[toolName] = tool;
			} else {
				console.log(`[ToolPreferences] Tool disabled by user: ${toolName}`);
			}
		} else {
			// Not preference-controlled - always include (MCP tools, system tools, etc.)
			filtered[toolName] = tool;
		}
	}

	return filtered;
}

/**
 * Check if a specific tool is enabled
 * Preference-controlled tools require explicit true value
 * Non-preference-controlled tools are always enabled
 * @param toolName - Name of the tool to check
 * @param preferences - User tool preferences
 * @returns True if enabled, false if disabled
 */
export function isToolEnabled(toolName: string, preferences: ToolPreferences): boolean {
	// If not preference-controlled, always enabled (MCP tools, system tools)
	if (!PREFERENCE_CONTROLLED_TOOLS.includes(toolName as any)) {
		return true;
	}

	// Preference-controlled tool - only enabled if explicitly set to true
	return preferences[toolName] === true;
}

/**
 * Get list of all disabled tool names
 * @param preferences - User tool preferences
 * @returns Array of disabled tool names
 */
export function getDisabledTools(preferences: ToolPreferences): string[] {
	return Object.entries(preferences)
		.filter(([_, enabled]) => enabled === false)
		.map(([toolName, _]) => toolName);
}

/**
 * Get list of all explicitly enabled tool names
 * @param preferences - User tool preferences
 * @returns Array of explicitly enabled tool names
 */
export function getEnabledTools(preferences: ToolPreferences): string[] {
	return Object.entries(preferences)
		.filter(([_, enabled]) => enabled === true)
		.map(([toolName, _]) => toolName);
}
