/**
 * Tab Registry - Namespace-based tab definitions with URL routing.
 *
 * URL Patterns:
 * - Entity namespaces: /{namespace} (list) or /{namespace}/{namespace}_{id} (detail)
 * - Storage: /drive, /drive/{path}
 * - System: /virtues/{page}
 */

import type { Component } from 'svelte';
import type { TabType, ParsedRoute } from './types';
import { getLocalDateSlug } from '$lib/utils/dateUtils';

// Import all view components
import HomeView from '$lib/components/tabs/views/HomeView.svelte';
import ChatView from '$lib/components/tabs/views/ChatView.svelte';
import HistoryView from '$lib/components/tabs/views/HistoryView.svelte';
import WikiView from '$lib/components/tabs/views/WikiView.svelte';
import WikiDetailView from '$lib/components/tabs/views/WikiDetailView.svelte';
import WikiListView from '$lib/components/tabs/views/WikiListView.svelte';
import SourcesView from '$lib/components/sources/SourcesView.svelte';
import CredentialDetailView from '$lib/components/tabs/views/CredentialDetailView.svelte';
import AppletsView from '$lib/components/tabs/views/AppletsView.svelte';
import AppletDetailView from '$lib/components/tabs/views/AppletDetailView.svelte';
import AppletView from '$lib/components/tabs/views/AppletView.svelte';
import DevelopersView from '$lib/components/tabs/views/DevelopersView.svelte';
import SettingsView from '$lib/components/tabs/views/SettingsView.svelte';
import StorageView from '$lib/components/tabs/views/StorageView.svelte';
import AssetView from '$lib/components/tabs/views/AssetView.svelte';
import PagesView from '$lib/components/tabs/views/PagesView.svelte';
import PageDetailView from '$lib/components/tabs/views/PageDetailView.svelte';
import BookmarksView from '$lib/components/tabs/views/BookmarksView.svelte';
import BookmarkDetailView from '$lib/components/tabs/views/BookmarkDetailView.svelte';
import NotebooksListView from '$lib/components/tabs/views/NotebooksListView.svelte';
import NotebookDetailView from '$lib/components/tabs/views/NotebookDetailView.svelte';
import NarrativeIdentityView from '$lib/components/tabs/views/NarrativeIdentityView.svelte';
import OntologyIndexView from '$lib/components/tabs/views/OntologyIndexView.svelte';
import OntologyDetailView from '$lib/components/tabs/views/OntologyDetailView.svelte';
import DataView from '$lib/components/tabs/views/DataView.svelte';

export interface TabDefinition {
	// Route matching
	match: (path: string, params: URLSearchParams) => boolean;
	parse: (path: string, params: URLSearchParams) => ParsedRoute;

	// Serialization (for URL sharing)
	serialize: (id?: string) => string;
	deserialize: (serialized: string) => string; // returns route

	// Metadata
	icon: string;
	defaultLabel: string;

	// Component reference
	// biome-ignore lint/suspicious/noExplicitAny: Component props vary by tab type
	component: Component<any>;

	// Optional: detail component for entity namespaces
	// biome-ignore lint/suspicious/noExplicitAny: Component props vary by tab type
	detailComponent?: Component<any>;
}

// Complete tab registry with namespace-based URL patterns
/**
 * Every path the wiki room answers to — the ONE list.
 *
 * This regex used to be written out twice, here and in WikiView's own section
 * parser. Adding Lifeline and History to one copy and not the other made both
 * rooms unreachable: the section rendered fine, the sidebar linked to it, the
 * typechecker was happy, and no tab would open because the router did not
 * recognise the path. Two lists that must agree is a bug waiting for the next
 * section.
 */
export const WIKI_SECTION_RE =
	/^\/wiki\/(days|years|entities|identity|chapters|lifeline|history|people|places|orgs|unlinked)$/;

/**
 * Sections of the Sources room. Same one-list rule as the wiki above, and here
 * it also disambiguates: `/sources/<x>` is a credential detail unless `<x>` is
 * one of these words, so the router and the view must agree about which words
 * are reserved or a section silently becomes a lookup for a credential that
 * does not exist.
 *
 * Overview is the bare `/sources` and so is not in this list.
 */
export const SOURCES_SECTIONS = ['catalog', 'activity'] as const;
export type SourcesSection = (typeof SOURCES_SECTIONS)[number];

export const tabRegistry: Record<TabType, TabDefinition> = {
	// ========================================================================
	// HOME: /home — the default landing / "Return" page (synthesis surface)
	// ========================================================================
	home: {
		match: (path) => path === '/home',
		parse: () => ({
			type: 'home',
			label: 'Home',
			icon: 'ri:home-5-line',
		}),
		serialize: () => 'home',
		deserialize: () => '/home',
		icon: 'ri:home-5-line',
		defaultLabel: 'Home',
		component: HomeView,
	},

	// ========================================================================
	// CHAT NAMESPACE: /, /chat, /chat/chat_{id}
	// ========================================================================
	chat: {
		match: (path) => path === '/' || path === '/chat' || /^\/chat\/chat_[^/]+$/.test(path),
		parse: (path, params) => {
			// Root or /chat = new chat
			if (path === '/' || path === '/chat') {
				// Preserve the temporary/ghost flag so ChatView can start in ghost mode.
				const temporary = params?.get('temporary') === '1';
				return {
					type: 'chat',
					label: temporary ? 'Temporary Chat' : 'New Chat',
					icon: temporary ? 'ri:ghost-line' : 'ri:chat-1-line',
					normalizedRoute: temporary ? '/chat?temporary=1' : '/chat',
				};
			}
			// Detail view
			const match = path.match(/^\/chat\/(chat_[^/]+)$/);
			return {
				type: 'chat',
				label: 'Chat',
				icon: 'ri:chat-1-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `chat_${id}` : 'chat'),
		deserialize: (serialized) => {
			if (serialized.startsWith('chat_')) {
				return `/chat/${serialized}`;
			}
			return '/chat';
		},
		icon: 'ri:chat-1-line',
		defaultLabel: 'Chats',
		component: ChatView,
		detailComponent: ChatView,
	},

	// ========================================================================
	// CHAT HISTORY: /chat-history
	// ========================================================================
	'chat-history': {
		match: (path) => path === '/chat-history',
		parse: () => ({
			type: 'chat-history',
			label: 'All Chats',
			icon: 'ri:chat-history-line',
		}),
		serialize: () => 'chat-history',
		deserialize: () => '/chat-history',
		icon: 'ri:chat-history-line',
		defaultLabel: 'All Chats',
		component: HistoryView,
	},

	// ========================================================================
	// PAGE NAMESPACE: /page, /page/page_{id}
	// ========================================================================
	page: {
		match: (path) => path === '/page' || /^\/page\/page_[^/]+$/.test(path),
		parse: (path) => {
			// List view
			if (path === '/page') {
				return {
					type: 'page',
					label: 'Pages',
					icon: 'ri:file-list-3-line',
				};
			}
			// Detail view
			const match = path.match(/^\/page\/(page_[^/]+)$/);
			return {
				type: 'page',
				label: 'Page',
				icon: 'ri:file-text-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `page_${id}` : 'page'),
		deserialize: (serialized) => {
			if (serialized.startsWith('page_')) {
				return `/page/${serialized}`;
			}
			return '/page';
		},
		icon: 'ri:file-list-3-line',
		defaultLabel: 'Pages',
		component: PagesView,
		detailComponent: PageDetailView,
	},

	// ========================================================================
	// WIKI: /wiki, /wiki/{days|entities|identity}
	// Legacy paths still match so old pins/deep-links land in the wiki:
	// /entities, and /wiki/{people|places|orgs|unlinked} (folded into the
	// unified entities section).
	// ========================================================================
	wiki: {
		match: (path) =>
			path === '/wiki' ||
			WIKI_SECTION_RE.test(path) ||
			path === '/entities',
		parse: () => ({
			type: 'wiki',
			label: 'Wiki',
			icon: 'ri:book-2-line',
		}),
		serialize: () => 'wiki',
		deserialize: () => '/wiki',
		icon: 'ri:book-2-line',
		defaultLabel: 'Wiki',
		component: WikiView,
	},

	// ========================================================================
	// PERSON NAMESPACE: /person, /person/{id}
	// ========================================================================
	person: {
		match: (path) => path === '/person' || /^\/person\/[^/]+$/.test(path),
		parse: (path) => {
			if (path === '/person') {
				return {
					type: 'person',
					label: 'People',
					icon: 'ri:user-line',
				};
			}
			const match = path.match(/^\/person\/([^/]+)$/);
			return {
				type: 'person',
				label: 'Person',
				icon: 'ri:user-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => id || 'person',
		deserialize: (serialized) => {
			if (serialized && serialized !== 'person') {
				return `/person/${serialized}`;
			}
			return '/person';
		},
		icon: 'ri:user-line',
		defaultLabel: 'People',
		component: WikiListView,
		detailComponent: WikiDetailView,
	},

	// ========================================================================
	// PLACE NAMESPACE: /place, /place/{id}
	// ========================================================================
	place: {
		match: (path) => path === '/place' || /^\/place\/[^/]+$/.test(path),
		parse: (path) => {
			if (path === '/place') {
				return {
					type: 'place',
					label: 'Places',
					icon: 'ri:map-pin-line',
				};
			}
			const match = path.match(/^\/place\/([^/]+)$/);
			return {
				type: 'place',
				label: 'Place',
				icon: 'ri:map-pin-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => id || 'place',
		deserialize: (serialized) => {
			if (serialized && serialized !== 'place') {
				return `/place/${serialized}`;
			}
			return '/place';
		},
		icon: 'ri:map-pin-line',
		defaultLabel: 'Places',
		component: WikiListView,
		detailComponent: WikiDetailView,
	},

	// ========================================================================
	// ORG NAMESPACE: /org, /org/{id}
	// ========================================================================
	org: {
		match: (path) => path === '/org' || /^\/org\/[^/]+$/.test(path),
		parse: (path) => {
			if (path === '/org') {
				return {
					type: 'org',
					label: 'Organizations',
					icon: 'ri:building-line',
				};
			}
			const match = path.match(/^\/org\/([^/]+)$/);
			return {
				type: 'org',
				label: 'Organization',
				icon: 'ri:building-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => id || 'org',
		deserialize: (serialized) => {
			if (serialized && serialized !== 'org') {
				return `/org/${serialized}`;
			}
			return '/org';
		},
		icon: 'ri:building-line',
		defaultLabel: 'Organizations',
		component: WikiListView,
		detailComponent: WikiDetailView,
	},

	// ========================================================================
	// BOOKMARK NAMESPACE: /bookmarks
	//
	// Saved web content — browser bookmarks, GitHub stars, hand-saved links.
	//
	// /bookmark/{id} is the detail. It used to be "a bookmark's detail IS the
	// page it points at" — true when a row was just a URL, wrong once a row
	// carries a note, tags, an extraction record and a read-state. The generic
	// /record/… view still renders the raw row for anyone who wants it; this
	// one leads with the note, because that is the only text on the page a
	// person wrote.
	// ========================================================================
	bookmarks: {
		match: (path) => path === '/bookmarks',
		parse: () => ({
			type: 'bookmarks',
			label: 'Bookmarks',
			icon: 'ri:bookmark-line',
			normalizedRoute: '/bookmarks',
		}),
		serialize: () => 'bookmarks',
		deserialize: () => '/bookmarks',
		icon: 'ri:bookmark-line',
		defaultLabel: 'Bookmarks',
		component: BookmarksView,
	},

	// BOOKMARK DETAIL: /bookmark/{id} — singular, matching /notebook/{id}.
	bookmark: {
		match: (path) => /^\/bookmark\/.+$/.test(path),
		parse: (path) => ({
			type: 'bookmark',
			label: 'Bookmark',
			icon: 'ri:bookmark-line',
			entityId: path.match(/^\/bookmark\/(.+)$/)?.[1],
		}),
		serialize: (id) => (id ? `bookmark_${id}` : 'bookmark'),
		deserialize: (serialized) =>
			serialized.startsWith('bookmark_') ? `/bookmark/${serialized.slice(9)}` : '/bookmarks',
		icon: 'ri:bookmark-line',
		defaultLabel: 'Bookmark',
		component: BookmarkDetailView,
		detailComponent: BookmarkDetailView,
	},

	// ========================================================================
	// NOTEBOOK NAMESPACE: /notebooks (list), /notebook/{id} (detail)
	//
	// A Notebook is the "room" a chat lives in — a workspace lens over the graph:
	// a Library of materials, filed chats, entities, and pages. (id may be a
	// legacy `space_…` or a new `nb_…` — both route the same.)
	// ========================================================================
	notebook: {
		match: (path) =>
			path === '/notebooks' ||
			path === '/notebook' ||
			/^\/notebook\/[^/]+$/.test(path),
		parse: (path) => {
			if (path === '/notebooks' || path === '/notebook') {
				return {
					type: 'notebook',
					label: 'Notebooks',
					icon: 'ri:booklet-line',
					normalizedRoute: '/notebooks',
				};
			}
			const match = path.match(/^\/notebook\/([^/]+)$/);
			return {
				type: 'notebook',
				label: 'Notebook',
				icon: 'ri:booklet-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => id || 'notebooks',
		deserialize: (serialized) => {
			if (serialized && serialized !== 'notebooks' && serialized !== 'notebook') {
				return `/notebook/${serialized}`;
			}
			return '/notebooks';
		},
		icon: 'ri:booklet-line',
		defaultLabel: 'Notebooks',
		component: NotebooksListView,
		detailComponent: NotebookDetailView,
	},

	// ========================================================================
	// DAY NAMESPACE: /day, /day/day_{date}
	// ========================================================================
	day: {
		match: (path) => path === '/day' || /^\/day\/day_\d{4}-\d{2}-\d{2}$/.test(path),
		parse: (path) => {
			if (path === '/day') {
				// Default to today - normalize route to include date
				const today = getLocalDateSlug();
				return {
					type: 'day',
					label: 'Today',
					icon: 'ri:calendar-line',
					entityId: `day_${today}`,
					normalizedRoute: `/day/day_${today}`,
				};
			}
			const match = path.match(/^\/day\/(day_\d{4}-\d{2}-\d{2})$/);
			const dateStr = match?.[1]?.replace('day_', '') || '';
			return {
				type: 'day',
				label: dateStr,
				icon: 'ri:calendar-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `day_${id}` : 'day'),
		deserialize: (serialized) => {
			if (serialized.startsWith('day_')) {
				return `/day/${serialized}`;
			}
			return '/day';
		},
		icon: 'ri:calendar-line',
		defaultLabel: 'Today',
		component: WikiDetailView,
		detailComponent: WikiDetailView,
	},

	// ========================================================================
	// YEAR NAMESPACE: /year, /year/year_{year}
	// ========================================================================
	year: {
		match: (path) => path === '/year' || /^\/year\/year_\d{4}$/.test(path),
		parse: (path) => {
			if (path === '/year') {
				// Default to current year - normalize route to include year
				const currentYear = new Date().getFullYear();
				return {
					type: 'year',
					label: String(currentYear),
					icon: 'ri:calendar-line',
					entityId: `year_${currentYear}`,
					normalizedRoute: `/year/year_${currentYear}`,
				};
			}
			const match = path.match(/^\/year\/(year_\d{4})$/);
			const yearStr = match?.[1]?.replace('year_', '') || '';
			return {
				type: 'year',
				label: yearStr,
				icon: 'ri:calendar-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `year_${id}` : 'year'),
		deserialize: (serialized) => {
			if (serialized.startsWith('year_')) {
				return `/year/${serialized}`;
			}
			return '/year';
		},
		icon: 'ri:calendar-line',
		defaultLabel: 'Year',
		component: WikiDetailView,
		detailComponent: WikiDetailView,
	},

	// ========================================================================
	// NARRATIVE IDENTITY: /narrative-identity
	// ========================================================================
	'narrative-identity': {
		match: (path) =>
			path === '/narrative-identity' ||
			/^\/narrative-identity\/(past|present|future)$/.test(path),
		parse: () => ({
			type: 'narrative-identity',
			label: 'Narrative Identity',
			icon: 'ri:quill-pen-line',
		}),
		serialize: () => 'narrative-identity',
		deserialize: () => '/narrative-identity',
		icon: 'ri:quill-pen-line',
		defaultLabel: 'Narrative Identity',
		component: NarrativeIdentityView,
	},

	// ========================================================================
	// SOURCE NAMESPACE: /sources, /sources/<section>, /sources/<credential_id>
	//   - `/sources` (and `/source` legacy alias) → Overview
	//   - `/sources/catalog`, `/sources/activity` → the other two sections
	//   - `/sources/<id>` → CredentialDetailView for one connection
	// "Source" in the URL is user-facing vocabulary; under the hood a connection
	// is a credential (OAuth/API-key) or a paired device (iOS/Mac).
	//
	// Sections are matched before ids — see SOURCES_SECTIONS for why the two
	// must not be decided in two places.
	// ========================================================================
	source: {
		match: (path) =>
			path === '/sources' || path === '/source' || /^\/sources\/[^/]+$/.test(path),
		parse: (path) => {
			if (path === '/sources' || path === '/source') {
				return {
					type: 'source',
					label: 'Sources',
					icon: 'ri:database-2-line',
					normalizedRoute: '/sources',
				};
			}
			const m = path.match(/^\/sources\/([^/]+)$/);
			const seg = m?.[1] ?? '';
			if ((SOURCES_SECTIONS as readonly string[]).includes(seg)) {
				return {
					type: 'source',
					label: `Sources · ${seg[0].toUpperCase()}${seg.slice(1)}`,
					icon: 'ri:database-2-line',
				};
			}
			return {
				type: 'source',
				label: 'Source',
				icon: 'ri:database-2-line',
				entityId: seg,
			};
		},
		serialize: (id) => (id ? id : 'sources'),
		deserialize: (serialized) => (serialized === 'sources' ? '/sources' : `/sources/${serialized}`),
		icon: 'ri:database-2-line',
		defaultLabel: 'Sources',
		// No `detailComponent`. TabContent decides list-vs-detail with a generic
		// `/^\/[a-z]+\/([^/]+)$/` on the route, which cannot tell `/sources/catalog`
		// (a section) from `/sources/cred_abc` (a credential) — it called both
		// details and rendered "Credential not found" on the catalog. SourcesView
		// owns the whole namespace and dispatches the detail itself, against the
		// same SOURCES_SECTIONS list the matcher above uses.
		component: SourcesView,
	},

	// ========================================================================
	// APPLETS: /applets (legacy /actions/* still resolves)
	// ========================================================================
	applets: {
		match: (path) =>
			path === '/applets' ||
			path === '/actions' || /^\/actions\/(actions|templates|history)$/.test(path),
		parse: () => ({
			type: 'applets',
			label: 'Applets',
			icon: 'ri:flashlight-line',
		}),
		serialize: () => 'applets',
		deserialize: () => '/applets',
		icon: 'ri:flashlight-line',
		defaultLabel: 'Applets',
		component: AppletsView,
	},

	// ========================================================================
	// APPLET VIEW: /applet/applet_{id}/view — the applet's face, full-page.
	// Must precede `applet` (whose match ends at $, so order is belt-and-braces).
	// ========================================================================
	'applet-view': {
		match: (path) => /^\/(?:applet|action)\/applet_[^/]+\/view$/.test(path),
		parse: (path) => {
			const match = path.match(/^\/(?:applet|action)\/(applet_[^/]+)\/view$/);
			return {
				type: 'applet-view',
				label: 'Applet',
				icon: 'ri:layout-2-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `${id}__view` : 'applet-view'),
		deserialize: (serialized) => {
			const id = serialized.replace(/__view$/, '');
			if (id.startsWith('applet_')) return `/applet/${id}/view`;
			return '/applets';
		},
		icon: 'ri:layout-2-line',
		defaultLabel: 'Applet',
		component: AppletView,
	},

	// ========================================================================
	// APPLET DETAIL: /applet/applet_{id} — settings, prompt, runs (no face).
	// ========================================================================
	applet: {
		match: (path) => /^\/(applet|action)\/applet_[^/]+$/.test(path),
		parse: (path) => {
			const match = path.match(/^\/(?:applet|action)\/(applet_[^/]+)$/);
			return {
				type: 'applet',
				label: 'Applet',
				icon: 'ri:flashlight-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `applet_${id}` : 'applet'),
		deserialize: (serialized) => {
			if (serialized.startsWith('applet_')) return `/applet/${serialized}`;
			return '/applets';
		},
		icon: 'ri:flashlight-line',
		defaultLabel: 'Applet',
		component: AppletDetailView,
		detailComponent: AppletDetailView,
	},

	// ========================================================================
	// DEVELOPERS: /developers
	// Tab group containing SQL, Terminal, and Lake sub-views (selected via #hash).
	// ========================================================================
	developers: {
		match: (path) =>
			path === '/developers' || /^\/developers\/(sql|terminal|lake)$/.test(path),
		parse: () => ({
			type: 'developers',
			label: 'Developers',
			icon: 'ri:code-s-slash-line',
		}),
		serialize: () => 'developers',
		deserialize: () => '/developers',
		icon: 'ri:code-s-slash-line',
		defaultLabel: 'Developers',
		component: DevelopersView,
	},

	// ========================================================================
	// ONTOLOGY NAMESPACE: /ontologies, /ontologies/{name}
	// ========================================================================
	ontology: {
		match: (path) => path === '/ontologies' || /^\/ontologies\/[a-z_]+$/.test(path),
		parse: (path) => {
			if (path === '/ontologies') {
				return {
					type: 'ontology',
					label: 'Ontologies',
					icon: 'ri:table-line',
				};
			}
			const match = path.match(/^\/ontologies\/([a-z_]+)$/);
			const name = match?.[1] || '';
			const displayName = name
				.replace(/_/g, ' ')
				.replace(/\b\w/g, (c) => c.toUpperCase());
			return {
				type: 'ontology',
				label: displayName,
				icon: 'ri:table-line',
				entityId: name,
			};
		},
		serialize: (id) => (id ? `ontology_${id}` : 'ontologies'),
		deserialize: (serialized) => {
			if (serialized.startsWith('ontology_')) return `/ontologies/${serialized.slice(9)}`;
			return '/ontologies';
		},
		icon: 'ri:table-line',
		defaultLabel: 'Ontologies',
		component: OntologyIndexView,
		detailComponent: OntologyDetailView,
	},
	record: {
		// /record/<ontology>/<id> — a single raw life-graph record. The ontology
		// is a lowercase_underscore name; the id is everything after it.
		match: (path) => /^\/record\/[a-z0-9_]+\/.+$/.test(path),
		parse: (path) => {
			const m = path.match(/^\/record\/([a-z0-9_]+)\/(.+)$/);
			const ontology = m?.[1] ?? '';
			const recordId = m?.[2] ?? '';
			const label = ontology.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
			return {
				type: 'record',
				label: label || 'Record',
				icon: 'ri:database-2-line',
				// entityId carries both segments so this reads as a detail view;
				// DataView re-parses the full route anyway.
				entityId: recordId ? `${ontology}/${recordId}` : undefined,
			};
		},
		serialize: (id) => (id ? `record_${id}` : 'record'),
		deserialize: (serialized) =>
			serialized.startsWith('record_') ? `/record/${serialized.slice(7)}` : '/record',
		icon: 'ri:database-2-line',
		defaultLabel: 'Record',
		component: DataView,
		detailComponent: DataView,
	},

	// ========================================================================
	// ASSET: /drive/file_{id} — single-file viewer (open density for a file ref)
	// Matched before `drive` so an id-addressed file opens the viewer, while
	// path-addressed routes (/drive/Documents/…) still open the browser.
	// ========================================================================
	asset: {
		match: (path) => /^\/drive\/file_[^/]+$/.test(path),
		parse: (path) => {
			const match = path.match(/^\/drive\/(file_[^/]+)$/);
			return {
				type: 'asset',
				label: 'File',
				icon: 'ri:file-line',
				entityId: match?.[1],
			};
		},
		serialize: (id) => (id ? `asset_${id}` : 'asset'),
		deserialize: (serialized) => {
			// serialized arrives as `asset_file_{id}` (from KNOWN_TYPES dispatch) or a
			// bare `file_{id}` legacy token; strip the type prefix and rebuild the route.
			const fileId = serialized.startsWith('asset_') ? serialized.slice(6) : serialized;
			return fileId.startsWith('file_') ? `/drive/${fileId}` : '/drive';
		},
		icon: 'ri:file-line',
		defaultLabel: 'File',
		component: AssetView,
		detailComponent: AssetView,
	},

	// ========================================================================
	// STORAGE NAMESPACE: /storage, /storage/{drive,streams,media,trash},
	//                    /storage/drive/{path}
	//
	// One surface for the four kinds of bytes the box holds — files you filed,
	// the raw evidence your devices sent, the assets the app made, and what you
	// deleted. They used to be scattered (/drive, /trash, /developers/lake) and
	// the lake was a stub rendering zeros.
	//
	// The base is /storage rather than /drive on purpose: Drive's sub-paths are
	// USER FOLDER NAMES, so a tab at /drive/streams would be ambiguous with a
	// folder someone actually named "streams" — and that folder would silently
	// become unreachable. Under /storage, drive paths live at /storage/drive/…
	// and can't collide with a tab.
	//
	// /drive and /trash still resolve (below) so existing links and bookmarks
	// keep working.
	// ========================================================================
	storage: {
		match: (path) => path === '/storage' || path.startsWith('/storage/'),
		parse: (path) => {
			const sub = path.match(/^\/storage\/(streams|media|trash)$/)?.[1];
			if (sub === 'streams') {
				return { type: 'storage', label: 'Streams', icon: 'ri:database-2-line' };
			}
			if (sub === 'media') {
				return { type: 'storage', label: 'App Media', icon: 'ri:image-2-line' };
			}
			if (sub === 'trash') {
				return { type: 'storage', label: 'Trash', icon: 'ri:delete-bin-line' };
			}
			// Drive, possibly deep inside a folder: /storage/drive/Documents/2026
			const storagePath = path.replace(/^\/storage\/drive\/?/, '');
			if (!storagePath) {
				return { type: 'storage', label: 'Drive', icon: 'ri:hard-drive-2-line' };
			}
			return {
				type: 'storage',
				label: storagePath.split('/').pop() || 'File',
				icon: 'ri:file-line',
				storagePath,
			};
		},
		serialize: (id) => (id ? `storage_${encodeURIComponent(id)}` : 'storage'),
		deserialize: (serialized) => {
			if (serialized.startsWith('storage_')) {
				return `/storage/${decodeURIComponent(serialized.slice(8))}`;
			}
			return '/storage';
		},
		icon: 'ri:hard-drive-2-line',
		defaultLabel: 'Drive',
		component: StorageView,
	},

	// ========================================================================
	// LEGACY: /drive, /drive/{path}, /trash — kept so old links resolve.
	// Both now open the Storage surface on the right tab.
	// ========================================================================
	drive: {
		match: (path) => path === '/drive' || path.startsWith('/drive/'),
		parse: (path) => {
			const storagePath = path.replace(/^\/drive\/?/, '');
			if (!storagePath) {
				return { type: 'drive', label: 'Drive', icon: 'ri:hard-drive-2-line' };
			}
			return {
				type: 'drive',
				label: storagePath.split('/').pop() || 'File',
				icon: 'ri:file-line',
				storagePath,
			};
		},
		serialize: (id) => (id ? `drive_${encodeURIComponent(id)}` : 'drive'),
		deserialize: (serialized) => {
			if (serialized.startsWith('drive_')) {
				return `/storage/drive/${decodeURIComponent(serialized.slice(6))}`;
			}
			return '/storage';
		},
		icon: 'ri:hard-drive-2-line',
		defaultLabel: 'Drive',
		component: StorageView,
	},

	trash: {
		match: (path) => path === '/trash',
		parse: () => ({
			type: 'trash',
			label: 'Trash',
			icon: 'ri:delete-bin-line',
		}),
		serialize: () => 'trash',
		deserialize: () => '/storage/trash',
		icon: 'ri:delete-bin-line',
		defaultLabel: 'Trash',
		component: StorageView,
	},

	// ========================================================================
	// SETTINGS NAMESPACE: /virtues[/{section}[/{sub}]]
	// One room (SettingsView) — You, Assistant, Connections, Billing, Box,
	// Developer — as a two-level route-driven sub-nav. Legacy flat pages
	// (/virtues/account, /virtues/system/*, /virtues/telemetry, ...) resolve
	// here and self-heal to their new home inside the room shell.
	// ========================================================================
	virtues: {
		match: (path) => path === '/virtues' || path.startsWith('/virtues/'),
		parse: (path) => {
			const page = path === '/virtues' ? 'you' : path.replace('/virtues/', '');
			return {
				type: 'virtues',
				label: 'Settings',
				icon: 'ri:settings-4-line',
				virtuesPage: page,
			};
		},
		serialize: (id) => (id ? `virtues_${id}` : 'virtues'),
		deserialize: (serialized) => {
			if (serialized.startsWith('virtues_')) {
				return `/virtues/${serialized.slice(8)}`;
			}
			return '/virtues/you';
		},
		icon: 'ri:settings-4-line',
		defaultLabel: 'Settings',
		component: SettingsView,
	},
};

/**
 * Get the appropriate component for a tab type and whether it's a detail view.
 */
// biome-ignore lint/suspicious/noExplicitAny: Component props vary by tab type
export function getComponent(type: TabType, hasEntityId: boolean): Component<any> {
	const def = tabRegistry[type];
	if (hasEntityId && def.detailComponent) {
		return def.detailComponent;
	}
	return def.component;
}

/**
 * Get the component for /virtues/* pages. There is now a single Settings room;
 * it dispatches to the right section from the route and self-heals legacy
 * flat paths on mount.
 */
// biome-ignore lint/suspicious/noExplicitAny: Component props vary by page
export function getVirtuesComponent(_page: string): Component<any> {
	return SettingsView;
}


/**
 * Parse a route string into tab metadata using the registry.
 */
export function parseRoute(route: string): ParsedRoute {
	const url = new URL(route, 'http://localhost');
	const path = url.pathname;
	const params = url.searchParams;

	// Try to match against registry in priority order
	// Note: Order matters for overlapping patterns
	const orderedTypes: TabType[] = [
		// Landing surface (exact /home; no overlap with '/')
		'home',
		// Specific patterns first
		'source', // Source list and detail views
		'applets', // Applets list page (must come before singular 'applet')
		'applet-view', // Applet full-page face (must come before 'applet')
		'applet', // Applet detail page
		'developers', // Developers tab group (SQL/Terminal/Lake)
		'ontology', // Ontology data browsing
		'record', // /record/<ontology>/<id> — single raw record
		'virtues', // Has /virtues/* pattern
		'storage', // /storage — Drive surface (unified bytes view)
		'asset', // /drive/file_{id} — must precede 'drive' (which matches all /drive/*)
		'drive', // Has /drive/* pattern
		'trash', // Drive trash
		'chat-history', // Chat history list (before 'chat')
		// Entity namespaces
		'chat', // Also matches /
		'page',
		'wiki', // Wiki room (overview + entity sections; also legacy /entities)
		'person',
		'place',
		'org',
		'notebook',
		// Detail before the room: /bookmark/{id} and /bookmarks are distinct
		// paths, but keeping the pair adjacent is how the next person notices
		// that BOTH have to be listed here. An entry in `tabRegistry` alone is
		// unreachable — this array is what routing actually walks.
		'bookmark',
		'bookmarks',
		'day',
		'year',
		'narrative-identity',
	];

	for (const type of orderedTypes) {
		const def = tabRegistry[type];
		if (def.match(path, params)) {
			return def.parse(path, params);
		}
	}

	// Fallback to chat
	return {
		type: 'chat',
		label: 'New Chat',
		icon: 'ri:chat-1-line',
	};
}

