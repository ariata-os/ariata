/**
 * Tab type definitions for namespace-based URL routing.
 *
 * URL structure:
 * - Entity namespaces: /{namespace} (list) or /{namespace}/{namespace}_{id} (detail)
 * - Storage namespaces: /drive, /lake (with subpaths)
 * - System namespace: /virtues/{page}
 */

// All supported tab types - consolidated namespace-based types
export type TabType =
	// Landing surface
	| 'home' // Home / "Return" page: /home
	// Entity namespaces (Postgres backend)
	| 'chat' // Chat conversations: /, /chat, /chat/chat_{id}
	| 'chat-history' // Chat history list: /chat-history
	| 'page' // User documents: /page, /page/page_{id}
	| 'wiki' // Wiki room (overview + entity sections): /wiki, /wiki/{section}
	| 'person' // Wiki people: /person, /person/person_{id}
	| 'place' // Wiki places: /place, /place/place_{id}
	| 'org' // Wiki organizations: /org, /org/org_{id}
	| 'notebook' // Notebooks (rooms a chat lives in): /notebooks, /notebook/{id}
	| 'bookmarks' // Saved web content (browser bookmarks, stars, saved links): /bookmarks
	| 'bookmark' // One saved thing, and the note on it: /bookmark/{id}
	| 'day' // Wiki days: /day, /day/day_{date}
	| 'year' // Wiki years: /year, /year/{year}
	| 'narrative-identity' // Wiki narrative identity: /narrative-identity
	| 'source' // Data sources: /source, /source/source_{id}
	| 'applets' // Applets list: /applets
	| 'applet' // Applet detail: /applet/applet_{id}
	| 'applet-view' // Applet full-page face: /applet/applet_{id}/view
	| 'developers' // Developers tools (SQL/Terminal/Lake): /developers
	| 'ontology' // Ontology data: /ontologies, /ontologies/{name}
	| 'record' // Single raw record viewer: /record/{ontology}/{id}
	// Storage namespaces
	| 'storage' // Drive | Streams | App Media | Trash: /storage, /storage/{tab}, /storage/drive/{path}
	| 'drive' // LEGACY alias of /storage (kept so old links resolve): /drive, /drive/{path}
	| 'asset' // Single file viewer (open density): /drive/file_{id}
	| 'trash' // LEGACY alias of /storage/trash: /trash
	// System namespace
	| 'virtues'; // System pages: /virtues/{account|assistant|usage|jobs|sql|terminal}

/**
 * Tab interface - flat structure with optional type-specific properties.
 *
 * Note: Entity IDs (e.g., 'chat_abc123') are derived from `route` using
 * `routeToEntityId(tab.route)`. The route is the source of truth.
 */
export interface Tab {
	id: string;
	type: TabType;
	label: string;
	route: string; // URL-native: '/chat/chat_abc123', '/page/page_xyz'
	icon?: string;
	pinned?: boolean;

	// Per-tab navigation history (browser model). `route` is a denormalized
	// mirror of `history[historyIndex]`, always written together with them.
	history: string[]; // routes, oldest → newest
	historyIndex: number; // pointer into history

	// Storage path (for drive/lake namespaces)
	storagePath?: string; // e.g., 'photos/2024/vacation.jpg'

	// System page (for virtues namespace)
	virtuesPage?: string; // e.g., 'account', 'usage', 'sql'

	scrollPosition?: number;
	createdAt: number;
}

// Type guard helpers for narrowing
export function isChatTab(tab: Tab): tab is Tab & { type: 'chat' } {
	return tab.type === 'chat';
}

export function isPageTab(tab: Tab): tab is Tab & { type: 'page' } {
	return tab.type === 'page';
}

export function isPersonTab(tab: Tab): tab is Tab & { type: 'person' } {
	return tab.type === 'person';
}

export function isPlaceTab(tab: Tab): tab is Tab & { type: 'place' } {
	return tab.type === 'place';
}

export function isOrgTab(tab: Tab): tab is Tab & { type: 'org' } {
	return tab.type === 'org';
}

// `isThingTab` lived here — a type guard for a type that no longer exists.
// Things were retired (a container masquerading as an entity); the guard outlived
// them, comparing TabType against a string it can never equal.

export function isDayTab(tab: Tab): tab is Tab & { type: 'day' } {
	return tab.type === 'day';
}

export function isYearTab(tab: Tab): tab is Tab & { type: 'year' } {
	return tab.type === 'year';
}

export function isSourceTab(tab: Tab): tab is Tab & { type: 'source' } {
	return tab.type === 'source';
}

export function isDriveTab(tab: Tab): tab is Tab & { type: 'drive' } {
	return tab.type === 'drive';
}

export function isTrashTab(tab: Tab): tab is Tab & { type: 'trash' } {
	return tab.type === 'trash';
}

export function isVirtuesTab(tab: Tab): tab is Tab & { type: 'virtues' } {
	return tab.type === 'virtues';
}

// Pane state - unified model where every tab lives in a pane
export interface PaneState {
	id: string; // 'left', 'right', or could be UUID for future N-pane
	tabs: Tab[];
	activeTabId: string | null;
	width: number; // percentage (e.g., 50 or 100)
}

// Route parsing result (used by parseRoute)
export interface ParsedRoute {
	type: TabType;
	label: string;
	icon: string;
	entityId?: string;
	storagePath?: string;
	virtuesPage?: string;
	/** If set, use this route instead of the original (e.g., /day → /day/day_2026-01-25) */
	normalizedRoute?: string;
}

// Re-export URL utilities for backward compatibility
// These are now defined in $lib/utils/urlUtils.ts
export {
	parseEntityId,
	entityIdToRoute,
	routeToEntityId
} from '$lib/utils/urlUtils';
