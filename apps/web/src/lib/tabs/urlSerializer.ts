/**
 * URL Serialization utilities for the tab system.
 *
 * Provides functions to serialize/deserialize tab state to/from URL query parameters.
 * This enables shareable URLs that restore the exact tab configuration.
 */

import type { Tab, TabType, PaneState } from './types';
import { tabRegistry, parseRoute } from './registry';

/**
 * Known tab types for proper deserialization.
 * Types with hyphens must be checked before splitting on underscore.
 */
const KNOWN_TYPES: TabType[] = [
	// Hyphenated types first (checked before splitting on underscore)
	'chat-history',
	'narrative-identity',
	// Then regular types
	'chat',
	'page',
	'person',
	'place',
	'org',
	'notebook',
	'bookmarks',
	'day',
	'year',
	'source',
	'applet',
	'applets',
	'ontology',
	'record',
	'wiki',
	'storage',
	'asset',
	'drive',
	'trash',
	'virtues',
];

/**
 * Serialize a tab to a URL-safe string.
 * Format: <type>[_<id>]
 * Examples: "chat", "chat_chat_abc123", "person_person_def456"
 */
export function serializeTab(tab: Tab): string {
	const def = tabRegistry[tab.type];
	if (!def) {
		return tab.type;
	}

	// Get the type-specific identifier from route or special fields
	let id: string | undefined;

	if (tab.storagePath) {
		// Storage namespaces use storagePath
		id = tab.storagePath;
	} else if (tab.virtuesPage) {
		// System namespace uses virtuesPage
		id = tab.virtuesPage;
	} else {
		// Entity namespaces: extract entity ID from route
		// Route format: /person/person_abc → entityId = person_abc
		const match = tab.route.match(/^\/[a-z]+\/(.+)$/);
		id = match?.[1];
	}

	return def.serialize(id);
}

/**
 * Deserialize a tab string back to a route.
 */
export function deserializeTab(serialized: string): string {
	// Try to match known types (handles hyphenated types correctly)
	let type: TabType | undefined;
	let id: string | undefined;

	for (const knownType of KNOWN_TYPES) {
		if (serialized === knownType) {
			// Exact match - no ID
			type = knownType;
			break;
		}
		if (serialized.startsWith(knownType + '_')) {
			// Type with ID
			type = knownType;
			id = decodeURIComponent(serialized.slice(knownType.length + 1));
			break;
		}
	}

	// Fallback: simple underscore split
	if (!type) {
		const underscoreIndex = serialized.indexOf('_');
		type = (underscoreIndex === -1 ? serialized : serialized.slice(0, underscoreIndex)) as TabType;
		id = underscoreIndex === -1 ? undefined : decodeURIComponent(serialized.slice(underscoreIndex + 1));
	}

	// Use registry to get the route
	const def = tabRegistry[type];
	if (def) {
		return def.deserialize(id ? `${type}_${id}` : type);
	}

	// Fallback to home
	return '/';
}

/**
 * Create a Tab object from a route string.
 */
export function createTabFromRoute(route: string): Tab {
	const parsed = parseRoute(route);

	return {
		id: crypto.randomUUID(),
		type: parsed.type,
		label: parsed.label,
		route,
		icon: parsed.icon,
		storagePath: parsed.storagePath,
		virtuesPage: parsed.virtuesPage,
		history: [route],
		historyIndex: 0,
		createdAt: Date.now(),
	};
}

/**
 * Split state for URL serialization.
 */
interface SplitState {
	enabled: boolean;
	panes: [PaneState, PaneState] | null;
	activePaneId: 'left' | 'right';
}

interface SerializedState {
	tabs: Tab[];
	activeTabId: string | null;
	split: SplitState;
}

/**
 * Serialize the entire tab state to URL query parameters.
 * Format: ?tabs=type_id,type_id&active=0&split=true&tabs2=type_id&active2=0
 */
export function serializeToUrl(state: SerializedState): string {
	const params = new URLSearchParams();

	if (state.split.enabled && state.split.panes) {
		// Split mode: serialize both panes
		const leftPane = state.split.panes[0];
		const rightPane = state.split.panes[1];

		// Left pane tabs
		if (leftPane.tabs.length > 0) {
			params.set('tabs', leftPane.tabs.map((t) => serializeTab(t)).join(','));
			const activeIndex = leftPane.tabs.findIndex((t) => t.id === leftPane.activeTabId);
			if (activeIndex >= 0) {
				params.set('active', String(activeIndex));
			}
		}

		// Right pane tabs
		if (rightPane.tabs.length > 0) {
			params.set('tabs2', rightPane.tabs.map((t) => serializeTab(t)).join(','));
			const activeIndex = rightPane.tabs.findIndex((t) => t.id === rightPane.activeTabId);
			if (activeIndex >= 0) {
				params.set('active2', String(activeIndex));
			}
		}

		params.set('split', 'true');

		// Store pane widths if not default
		if (leftPane.width !== 50 || rightPane.width !== 50) {
			params.set('widths', `${leftPane.width},${rightPane.width}`);
		}
	} else {
		// Single pane mode
		if (state.tabs.length > 0) {
			params.set('tabs', state.tabs.map((t) => serializeTab(t)).join(','));
			const activeIndex = state.tabs.findIndex((t) => t.id === state.activeTabId);
			if (activeIndex >= 0) {
				params.set('active', String(activeIndex));
			}
		}
	}

	const queryString = params.toString();
	return queryString ? `/?${queryString}` : '/';
}

/**
 * Deserialize URL query parameters to restore tab state.
 * Returns the reconstructed state or null if no tab params in URL.
 */
export function deserializeFromUrl(url: string): SerializedState | null {
	try {
		const parsedUrl = new URL(url, 'http://localhost');
		const params = parsedUrl.searchParams;

		const tabsParam = params.get('tabs');
		const activeParam = params.get('active');
		const splitParam = params.get('split');
		const tabs2Param = params.get('tabs2');
		const active2Param = params.get('active2');
		const widthsParam = params.get('widths');

		// Skip if no tab params in URL
		if (!tabsParam) {
			return null;
		}

		if (splitParam === 'true') {
			// Restore split mode
			const leftTabs = tabsParam.split(',').filter(Boolean);
			const rightTabs = tabs2Param?.split(',').filter(Boolean) || [];
			const leftActive = activeParam ? parseInt(activeParam, 10) : 0;
			const rightActive = active2Param ? parseInt(active2Param, 10) : 0;

			// Parse widths
			let leftWidth = 50;
			let rightWidth = 50;
			if (widthsParam) {
				const [left, right] = widthsParam.split(',').map((w) => parseFloat(w));
				if (!Number.isNaN(left) && !Number.isNaN(right)) {
					leftWidth = left;
					rightWidth = right;
				}
			}

			// Create tabs for left pane
			const leftPaneTabs: Tab[] = leftTabs.map((serialized) => {
				const route = deserializeTab(serialized);
				return createTabFromRoute(route);
			});

			// Create tabs for right pane
			const rightPaneTabs: Tab[] = rightTabs.map((serialized) => {
				const route = deserializeTab(serialized);
				return createTabFromRoute(route);
			});

			return {
				tabs: [],
				activeTabId: null,
				split: {
					enabled: true,
					panes: [
						{
							id: 'left',
							tabs: leftPaneTabs,
							activeTabId: leftPaneTabs[leftActive]?.id || leftPaneTabs[0]?.id || null,
							width: leftWidth,
						},
						{
							id: 'right',
							tabs: rightPaneTabs,
							activeTabId: rightPaneTabs[rightActive]?.id || rightPaneTabs[0]?.id || null,
							width: rightWidth,
						},
					],
					activePaneId: 'left',
				},
			};
		} else {
			// Restore single pane mode
			const tabStrings = tabsParam.split(',').filter(Boolean);
			const activeIndex = activeParam ? parseInt(activeParam, 10) : 0;

			const tabs: Tab[] = tabStrings.map((serialized) => {
				const route = deserializeTab(serialized);
				return createTabFromRoute(route);
			});

			return {
				tabs,
				activeTabId: tabs[activeIndex]?.id || tabs[0]?.id || null,
				split: {
					enabled: false,
					panes: null,
					activePaneId: 'left',
				},
			};
		}
	} catch (e) {
		console.warn('[urlSerializer] Failed to deserialize URL:', e);
		return null;
	}
}

/**
 * Check if the URL has tab params that should be restored.
 */
export function hasUrlTabParams(url: string): boolean {
	try {
		const parsedUrl = new URL(url, 'http://localhost');
		return parsedUrl.searchParams.has('tabs');
	} catch {
		return false;
	}
}
