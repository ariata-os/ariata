/**
 * Tool preferences management
 *
 * Handles loading and filtering tools based on user preferences.
 * Default behavior: tools not in preferences object are enabled.
 */
import type { Pool } from 'pg';

/**
 * Tool preferences object
 * - Empty object {} means all tools are enabled
 * - Tools not in object default to enabled
 * - Only tools explicitly set to false are disabled
 *
 * Example: { "queryLocationMap": true, "queryPursuits": false }
 */
export type ToolPreferences = Record<string, boolean>;

/**
 * Load user tool preferences from database
 * @param pool - Database connection pool
 * @returns Tool preferences object (empty object if none set)
 */
export async function loadUserToolPreferences(pool: Pool): Promise<ToolPreferences> {
	try {
		const result = await pool.query(
			'SELECT enabled_tools FROM elt.assistant_profile LIMIT 1'
		);

		const enabledTools = result.rows[0]?.enabled_tools;

		// Return empty object if null/undefined (means all tools enabled)
		if (!enabledTools || typeof enabledTools !== 'object') {
			return {};
		}

		return enabledTools as ToolPreferences;
	} catch (error) {
		console.error('[ToolPreferences] Failed to load preferences:', error);
		// On error, default to all tools enabled
		return {};
	}
}

/**
 * Filter tools based on user preferences
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
		// If tool is not in preferences, default to enabled
		// Only disable if explicitly set to false
		if (preferences[toolName] !== false) {
			filtered[toolName] = tool;
		} else {
			console.log(`[ToolPreferences] Tool disabled by user: ${toolName}`);
		}
	}

	return filtered;
}

/**
 * Check if a specific tool is enabled
 * @param toolName - Name of the tool to check
 * @param preferences - User tool preferences
 * @returns True if enabled, false if disabled
 */
export function isToolEnabled(toolName: string, preferences: ToolPreferences): boolean {
	// Default to enabled if not in preferences
	return preferences[toolName] !== false;
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
