/**
 * API Client for Rust Core Library
 *
 * All calls to /api/* are proxied to Rust backend (localhost:8000)
 * via Vite proxy (see vite.config.ts).
 */

import { sanitizeUrl } from '$lib/utils/urlUtils';

const API_BASE = '/api';

// ============================================================================
// Shared request layer
// ============================================================================

/**
 * Error thrown by the shared `request()` helper when a response is not ok.
 * Carries the HTTP `status` so callers can branch on it (e.g. 402 = wallet
 * expired / subscription lapsed, 401 = unknown key) instead of parsing a
 * stringified statusText. `body` holds the parsed JSON error payload when the
 * server returned one.
 */
export class ApiError extends Error {
	readonly status: number;
	readonly body: unknown;
	constructor(status: number, message: string, body?: unknown) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.body = body;
	}
}

type QueryValue = string | number | boolean | null | undefined;

/**
 * Core fetch wrapper for JSON endpoints under `/api`. Serializes an optional
 * query object, throws {@link ApiError} (with status) on non-2xx, and returns
 * the parsed JSON body (or `undefined` for empty/204 responses).
 *
 * Not for binary/blob endpoints or progress-tracked uploads — those keep their
 * own `fetch`/XHR (see uploadMedia, downloadDriveFile).
 */
export async function request<T>(
	path: string,
	init?: RequestInit & { query?: Record<string, QueryValue> },
): Promise<T> {
	let url = `${API_BASE}${path}`;
	if (init?.query) {
		const qs = new URLSearchParams();
		for (const [key, value] of Object.entries(init.query)) {
			if (value !== undefined && value !== null) qs.set(key, String(value));
		}
		const q = qs.toString();
		if (q) url += `?${q}`;
	}

	const res = await fetch(url, init);

	if (!res.ok) {
		let body: unknown;
		let message = res.statusText || `HTTP ${res.status}`;
		try {
			const text = await res.text();
			if (text) {
				try {
					body = JSON.parse(text);
					const parsed = body as { error?: string; message?: string };
					message = parsed?.error || parsed?.message || message;
				} catch {
					body = text;
					message = text;
				}
			}
		} catch {
			/* keep statusText fallback */
		}
		throw new ApiError(res.status, message, body);
	}

	if (res.status === 204) return undefined as T;
	const text = await res.text();
	return (text ? JSON.parse(text) : undefined) as T;
}

/** GET a JSON endpoint, with an optional query object. */
export function apiGet<T>(path: string, query?: Record<string, QueryValue>): Promise<T> {
	return request<T>(path, { query });
}

/** Send a JSON body (POST/PUT/PATCH/DELETE) and parse the JSON response. */
export function apiSend<T>(method: string, path: string, jsonBody?: unknown): Promise<T> {
	return request<T>(path, {
		method,
		headers: jsonBody !== undefined ? { 'Content-Type': 'application/json' } : undefined,
		body: jsonBody !== undefined ? JSON.stringify(jsonBody) : undefined,
	});
}

// ============================================================================
// Actions — new schema (post cutover + PR 2 endpoints)
// ============================================================================

export type AppletTrigger = 'cron' | 'manual' | 'tool' | 'api' | 'webhook' | 'message';

export interface AppletRun {
	id: string;
	applet_id: string | null;
	/** `budget_exceeded` is not a failure: a spend ceiling the owner set in
	 *  `config.limits` was reached, so the run stopped deliberately. It stays
	 *  out of the needs-attention strip for that reason. */
	status: 'running' | 'success' | 'error' | 'cancelled' | 'skipped' | 'budget_exceeded';
	started_at: string;
	completed_at: string | null;
	records_processed: number;
	error: string | null;
	trigger: AppletTrigger;
	result_summary: string | null;
	/** What the user said, for `message` runs. This plus `result_summary` is
	 *  the exchange — the conversation lives on the run, not in a thread. */
	message: string | null;
	/** What this run spent with the model, in micros-USD — summed from the
	 *  gateway's authoritative per-call figures. Zero for runs that called no
	 *  model, and for every run recorded before spend was attributed. */
	cost_micros: number;
	created_at: string;
}

export interface AppletLastRun {
	status: string;
	started_at: string | null;
	completed_at?: string | null;
	records_processed: number | null;
	error: string | null;
	summary?: string | null;
}

/**
 * Which of the four things made this applet — the facet lists and filters use.
 *
 * Derived server-side, not stored: `owner` is the write-authority field
 * (reconcile overwrites and GCs `system` rows, leaves `user`/`ai` alone), and
 * every source fan-out row is `system` under it. Faceting on `owner` therefore
 * files a Gmail sync with the embedding indexer; `origin` splits them.
 */
export type AppletOrigin = 'source' | 'system' | 'user' | 'ai';

export interface Applet {
	id: string;
	owner: 'system' | 'user' | 'ai';
	name: string;
	/** The applet's one-sentence intent, from its manifest — what it is for,
	 *  in the user's terms. The list row's plain-English line and the detail
	 *  headline. Null only for a row whose manifest omits it. */
	description: string | null;
	agent: string | null;
	schedule: string | null;
	enabled: boolean;
	config: Record<string, unknown>;
	condition: string | null;
	triggers: AppletTrigger[];
	memory: string | null;
	/** Set when the applet is one source connection's fan-out (OAuth/API-key). */
	credential_id: string | null;
	/** Set when the applet is one paired device's ingest webhook (iOS/Mac). */
	device_id: string | null;
	origin: AppletOrigin;
	/** Polyglot escape: explicit argv to spawn instead of resolving a Cargo
	 *  binary by `function_name`. Null when the action uses the function_name
	 *  shortcut. */
	command: string[] | null;
	/** Lifecycle: null = forever · "once" = archive after first success ·
	 *  anything else = SQL boolean checked after each success. */
	until: string | null;
	/** Set when the lifecycle completed; archived applets are disabled. */
	archived_at: string | null;
	/** The scheduled slot this applet currently owes. Null when it has no
	 *  schedule, or the scheduler has not seen it yet. Far enough in the past
	 *  means it was expected and didn't run. */
	next_due_at: string | null;
	/** The last scheduled slot actually handled — fired, or consciously
	 *  skipped. Not the same as the last run's `started_at`: a catch-up run at
	 *  07:12 handles the 06:00 slot. */
	last_slot_at: string | null;
	/** True when the applet folder ships a face/ (sandboxed-iframe HTML UI). */
	has_face: boolean;
	/** The last 10 run statuses, newest first — the card's pulse. Comes with
	 *  the row so the list does not fetch it per applet. */
	pulse: AppletRun['status'][];
	/** The last successful run's summary — what the applet last produced. */
	last_success_summary: string | null;
	created_at: string;
	updated_at: string;
	last_run: AppletLastRun | null;
}

export interface AppletDetail extends Applet {
	recent_runs?: AppletRun[];
}

export async function mintFaceToken(
	appletId: string
): Promise<{ token: string; expires_in_seconds: number }> {
	return request(`/applets/${encodeURIComponent(appletId)}/face-token`);
}

export async function listApplets(): Promise<Applet[]> {
	const res = await fetch(`${API_BASE}/applets`);
	if (!res.ok) throw new Error(`Failed to list applets: ${res.statusText}`);
	return res.json();
}

export async function getApplet(id: string): Promise<Applet> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}`);
	if (!res.ok) throw new Error(`Failed to get applet: ${res.statusText}`);
	return res.json();
}

// ─────────────────────────────────────────────────────────────────────────────
// Local content search (⌘K)
// ─────────────────────────────────────────────────────────────────────────────

/** One content hit — a matched chunk inside an indexed record. */
export interface LocalSearchHit {
	ontology: string;
	record_id: string;
	score: number;
	title: string | null;
	preview: string | null;
	author: string | null;
	timestamp: string | null;
}

export interface LocalSearchResponse {
	hits: LocalSearchHit[];
	/** Echoed back so callers can drop responses that lost a race. */
	query: string;
}

/**
 * Content search across everything the box has indexed. Never leaves the box —
 * `searchWeb` (Exa) is the separate, explicit thing.
 *
 * POST, not GET: the query is what someone is searching their own life for, and
 * a GET would put it in the URL, browser history, and every access log along the
 * way.
 */
export async function searchLocal(
	q: string,
	opts: { limit?: number; signal?: AbortSignal } = {},
): Promise<LocalSearchResponse> {
	const res = await fetch(`${API_BASE}/search/local`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ q, limit: opts.limit }),
		signal: opts.signal,
	});
	if (!res.ok) throw new Error(`Search failed: ${res.statusText}`);
	return res.json();
}

// ─────────────────────────────────────────────────────────────────────────────
// Box updates (Settings → Box)
// ─────────────────────────────────────────────────────────────────────────────

/** A release already downloaded, verified and preflighted on the box. */
export interface StagedRelease {
	/** Slot directory name, e.g. `v0.3.1-a1b2c3d`. */
	slot_id: string;
	/** Version from the release's own BUILD.json, when it carries one. */
	version: string | null;
	/** Commit the release was built from, when known. */
	sha: string | null;
}

export interface UpdateStatus {
	/** The BUILD COUNTER (crate version). Not what the box is running — see
	 *  `running_version`. Kept because it is the compat coordinate. */
	current: string;
	/** The stored update PREFERENCE: what the box will be offered next. */
	channel: string;
	/** RELEASE IDENTITY — what this box is actually running, from the baked
	 *  build tag: `edge`, `staging.4`, `v0.3.0`, `dev`. */
	running_version: string;
	/** Channel of the running BUILD: `stable` | `staging` | `edge` | `dev`. */
	running_channel: string;
	/** The running build is off a later track than the channel being followed,
	 *  so that channel's newest tag is probably behind this box. */
	running_ahead: boolean;
	latest: string | null;
	update_available: boolean;
	/**
	 * Set when the box has already fetched the update. Installing it is then a
	 * restart plus migrations rather than a download — seconds, not minutes —
	 * which is the difference the UI has to tell the truth about.
	 */
	staged: StagedRelease | null;
	check_error: string | null;
}

export async function getUpdateStatus(): Promise<UpdateStatus> {
	const res = await fetch(`${API_BASE}/system/update`);
	if (!res.ok) throw new Error(`Failed to get update status: ${res.statusText}`);
	return res.json();
}

export async function setUpdateChannel(channel: 'stable' | 'prerelease'): Promise<void> {
	const res = await fetch(`${API_BASE}/system/update/channel`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ channel }),
	});
	if (!res.ok) throw new Error(`Failed to set channel: ${res.statusText}`);
}

/** One line of the census: a thing the box holds, and how many of it. */
export interface CensusLine {
	id: string;
	/** Plural, lowercase, already in the words a person would use. */
	label: string;
	count: number;
}

export interface Census {
	/** Only non-empty lines. A box with nothing connected returns []. */
	lines: CensusLine[];
	total: number;
	earliest: string | null;
	latest: string | null;
	span_days: number;
	/** The record's first named senders, in the order it met them —
	 *  chronology, never significance. Empty when none are presentable. */
	earliest_names: string[];
	/** `"YYYY-MM-DD"` of the first day the box narrated; null until one exists. */
	first_day: string | null;
}

/** What the box actually holds, counted — the reveal's first movement. */
export async function getCensus(): Promise<Census> {
	const res = await fetch(`${API_BASE}/census`);
	if (!res.ok) throw new Error(`Failed to read the census: ${res.statusText}`);
	return res.json();
}

export interface NarrativeDraft {
	document: string;
	/** Proposed only. Nothing binds the assistant until it is confirmed. */
	proposed_rules: string[];
}

/** Draft the document from the answers. Spends money; POST, never on load. */
export async function draftNarrative(): Promise<NarrativeDraft> {
	const res = await fetch(`${API_BASE}/narrative/draft`, { method: 'POST' });
	if (!res.ok) {
		let detail = res.statusText;
		try {
			const b = await res.json();
			if (b?.error) detail = b.error;
		} catch {
			/* status text is all we have */
		}
		throw new Error(detail);
	}
	return res.json();
}

/**
 * Replace the rule set with exactly what was confirmed.
 *
 * A replace, not an append: this is the screen where someone sees every rule
 * their box obeys, so leaving it has to mean the list says what they saw.
 */
export async function saveNarrativeRules(rules: string[]): Promise<void> {
	const res = await fetch(`${API_BASE}/narrative/rules`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ rules }),
	});
	if (!res.ok) throw new Error(`Couldn't save your rules: ${res.statusText}`);
}

export interface ReopenOnboardingResponse {
	devices: number;
	credentials: number;
}

/**
 * Revoke every paired device, keeping data, sources, subscription and box
 * identity. The box becomes unclaimed: it advertises over Bluetooth again and
 * the panel returns to a setup screen.
 *
 * This device is revoked too — the caller loses its own session, by design.
 */
export async function reopenOnboarding(): Promise<ReopenOnboardingResponse> {
	const res = await fetch(`${API_BASE}/pair/reopen-onboarding`, { method: 'POST' });
	if (!res.ok) {
		let detail = res.statusText;
		try {
			const body = await res.json();
			if (body?.error) detail = body.error;
		} catch {
			/* non-JSON body — the status text is all we have */
		}
		throw new Error(detail);
	}
	return res.json();
}

export interface ApplyUpdateResponse {
	unit: string;
	/** Whether this activated an already-downloaded release (fast) or had to
	 *  fetch one first (slow). */
	staged: boolean;
	detail: string;
}

/**
 * Start an upgrade. Resolves as soon as the box has *accepted* the job (202),
 * not when it finishes — the upgrade restarts the box, so there is no response
 * to wait for. Watch `boxReachable` for the box going away and coming back.
 *
 * The error text is the box's own, because "update failed" with nothing behind
 * it is what sends someone to SSH in to find out why.
 */
export async function applyUpdate(): Promise<ApplyUpdateResponse> {
	const res = await fetch(`${API_BASE}/system/update/apply`, { method: 'POST' });
	if (!res.ok) {
		let detail = res.statusText;
		try {
			const body = await res.json();
			if (body?.error) detail = body.error;
			else if (body?.message) detail = body.message;
		} catch {
			/* non-JSON body — the status text is all we have */
		}
		throw new Error(detail);
	}
	return res.json();
}

// ─────────────────────────────────────────────────────────────────────────────
// Display — the box's attached screen (Settings → Display)
// ─────────────────────────────────────────────────────────────────────────────

/** The face the panel wears in its ambient slot. */
export interface DisplayFaceConfig {
	kind: 'builtin' | 'applet';
	/** Which built-in, when kind is 'builtin': 'record' | 'matte'. */
	builtin: string;
	applet_id?: string | null;
}

export interface DisplayPanelInfo {
	/** DRM connector name, e.g. `card0-HDMI-A-1`. */
	connector: string;
	mode_width?: number;
	mode_height?: number;
}

/**
 * What the glass is showing, redacted for the LAN: the mirror may say THAT
 * the panel is on its setup screen, never what the words are.
 */
export interface DisplayGlassState {
	claimed: boolean;
	online: boolean;
	connectivity: string;
	devices: number;
	box_name: string;
	data_disk_fault?: string | null;
	record?: Array<{ label: string; count: number }>;
	record_since?: string | null;
	updating: boolean;
	/** Hours is holding the glass dark right now (backlight off). */
	asleep: boolean;
}

/** The screen's hours. Times are box-local "HH:MM:SS"; absent = never sleeps. */
export interface DisplayHours {
	sleep_start?: string | null;
	sleep_end?: string | null;
}

export interface DisplaySettings {
	/** This box has the kiosk stack installed at all. */
	attached: boolean;
	unit_state: 'active' | 'installed but not running' | 'not installed';
	panel?: DisplayPanelInfo | null;
	zoom_derived?: number | null;
	zoom_override?: number | null;
	face: DisplayFaceConfig;
	hours: DisplayHours;
	state: DisplayGlassState;
}

export function getDisplaySettings(): Promise<DisplaySettings> {
	return apiGet('/system/display');
}

export function setDisplayFace(face: {
	kind: 'builtin' | 'applet';
	builtin?: string;
	applet_id?: string;
}): Promise<DisplayFaceConfig> {
	return apiSend('PUT', '/system/display/face', face);
}

export function restartDisplay(): Promise<{ restarted: boolean }> {
	return apiSend('POST', '/system/display/restart');
}

/** Set the sleep schedule ("HH:MM" box-local), or both null for never. */
export function setDisplayHours(hours: {
	sleep_start: string | null;
	sleep_end: string | null;
}): Promise<DisplayHours> {
	return apiSend('PUT', '/system/display/hours', hours);
}

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar pins
// ─────────────────────────────────────────────────────────────────────────────

export interface Pin {
	id: string;
	url: string;
	label: string | null;
	icon: string | null;
	sort_order: number;
	pinned_at: string;
	/** A `--cat-*` token key ('orange', 'emerald'…), never a hex. See CAT_COLORS. */
	color: string | null;
}

export async function listPins(): Promise<Pin[]> {
	const res = await fetch(`${API_BASE}/pins`);
	if (!res.ok) throw new Error(`Failed to list pins: ${res.statusText}`);
	return res.json();
}

export async function createPin(req: {
	url: string;
	label?: string | null;
	icon?: string | null;
	color?: string | null;
}): Promise<Pin> {
	const res = await fetch(`${API_BASE}/pins`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(req)
	});
	if (!res.ok) throw new Error(`Failed to pin: ${res.statusText}`);
	return res.json();
}

export async function updatePin(
	id: string,
	req: { label?: string | null; icon?: string | null; sort_order?: number; color?: string | null }
): Promise<Pin> {
	const res = await fetch(`${API_BASE}/pins/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(req)
	});
	if (!res.ok) throw new Error(`Failed to update pin: ${res.statusText}`);
	return res.json();
}

export async function deletePin(id: string): Promise<void> {
	const res = await fetch(`${API_BASE}/pins/${encodeURIComponent(id)}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`Failed to delete pin: ${res.statusText}`);
}

export async function reorderPins(urls: string[]): Promise<void> {
	const res = await fetch(`${API_BASE}/pins/reorder`, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ urls })
	});
	if (!res.ok) throw new Error(`Failed to reorder pins: ${res.statusText}`);
}

/** POST /api/admin/reconcile — re-reads manifests and upserts applet rows. */
export async function adminReconcile(): Promise<{ upserted: number }> {
	const res = await fetch(`${API_BASE}/admin/reconcile`, { method: 'POST' });
	if (!res.ok) throw new Error(`Reconcile failed: ${res.statusText}`);
	return res.json();
}

/**
 * POST /api/admin/applets/import-git — clones a repo into `actions/<slug>/`
 * and runs the standard scanner. Any folder under the slug containing a
 * `manifest.toml` becomes an action. Returns added/updated/removed ids.
 */
export async function importActionsFromGit(body: {
	url: string;
	ref?: string;
	/** From `/api/sudo/request` — importing runs someone else's code. */
	sudo_request_id: string;
}): Promise<{
	/** Folder the package landed in — host + owner + repo, so two remotes with
	 *  the same repo name can't claim each other's. */
	slug: string;
	/** Commit actually checked out. The server has always resolved this; the
	 *  client used to drop it, which is why "is there a newer version" was
	 *  unanswerable. */
	commit: string | null;
	added: string[];
	updated: string[];
	removed: string[];
}> {
	const res = await fetch(`${API_BASE}/admin/applets/import-git`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) {
		const text = await res.text().catch(() => res.statusText);
		throw new Error(text || `Import failed: ${res.statusText}`);
	}
	return res.json();
}

export interface PatchAppletBody {
	name?: string;
	agent?: string | null;
	schedule?: string | null;
	enabled?: boolean;
	config?: Record<string, unknown>;
	condition?: string | null;
	triggers?: AppletTrigger[];
	memory?: string | null;
}

export async function patchApplet(id: string, patch: PatchAppletBody): Promise<Applet> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(patch)
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to update applet: ${res.statusText}`);
	}
	return res.json();
}

export async function deleteApplet(id: string, dropData = false): Promise<void> {
	const q = dropData ? '?drop_data=true' : '';
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}${q}`, {
		method: 'DELETE'
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to delete applet: ${res.statusText}`);
	}
}

/** One line of an applet's log: consecutive runs that shared an outcome,
 *  counted. `occurrences > 1` means the applet repeated itself — which on a
 *  real box is most of history, since a poll that finds nothing still records
 *  a run. An agent's output varies, so its entries never group. */
export interface AppletLogEntry {
	run_id: string | null;
	status: AppletRun['status'];
	trigger: AppletTrigger | null;
	summary: string | null;
	/** What the user said, for `message` runs. */
	message: string | null;
	error: string | null;
	occurrences: number;
	first_at: string | null;
	last_at: string | null;
	cost_micros: number;
}

export async function getAppletLog(id: string, limit = 50): Promise<AppletLogEntry[]> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}/log?limit=${limit}`);
	if (!res.ok) throw new Error(`Failed to load log: ${res.statusText}`);
	return res.json();
}

/** Say something to an applet — the `message` wake. Returns once the run row
 *  exists; the agent turn continues detached. */
export async function messageApplet(id: string, message: string): Promise<{ run_id: string | null; status: string }> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}/message`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ message })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to send: ${res.statusText}`);
	}
	return res.json();
}

/** The private tables an applet owns — shown on the delete confirm so the user
 *  can choose whether to also drop its data. Empty when it owns none. */
export interface AppletData {
	schema: string | null;
	tables: string[];
}

export async function getAppletData(id: string): Promise<AppletData> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}/data`);
	if (!res.ok) return { schema: null, tables: [] };
	return (await res.json()) as AppletData;
}

export interface TriggerActionResponse {
	run_id: string | null;
	applet_id: string;
	status: string;
	summary: string;
	error: string | null;
}

export async function runApplet(
	id: string,
	payload?: Record<string, unknown>
): Promise<TriggerActionResponse> {
	const res = await fetch(`${API_BASE}/applets/${encodeURIComponent(id)}/run`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(payload ? { payload } : {})
	});
	if (!res.ok && res.status !== 500) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to run applet: ${res.statusText}`);
	}
	return res.json();
}

// ── Applet source ───────────────────────────────────────────────────────────
// Reading the code that ran. Every applet, whatever its provenance.

export interface AppletSourceFile {
	path: string;
	size: number;
	/** False for binary or oversized files — don't offer to open them. */
	readable: boolean;
}

export interface AppletSourceListing {
	dir: string;
	/** Which root the folder resolved in — `shipped` came with the box. */
	origin_root: 'shipped' | 'state';
	files: AppletSourceFile[];
	truncated: boolean;
}

export async function getAppletSource(id: string): Promise<AppletSourceListing> {
	return apiGet<AppletSourceListing>(`/applets/${encodeURIComponent(id)}/source`);
}

/**
 * Copy a shipped applet onto this box so it can be changed. Reverting is
 * deleting the copy; the shipped version is never touched.
 */
export async function forkApplet(id: string): Promise<{ status: string; dir: string }> {
	return apiSend<{ status: string; dir: string }>(
		'POST',
		`/applets/${encodeURIComponent(id)}/fork`
	);
}

export async function getAppletSourceFile(
	id: string,
	path: string
): Promise<{ path: string; text: string }> {
	const safe = path.split('/').map(encodeURIComponent).join('/');
	return apiGet<{ path: string; text: string }>(
		`/applets/${encodeURIComponent(id)}/source/${safe}`
	);
}

export async function listRuns(opts?: {
	limit?: number;
	status?: string;
	applet_id?: string;
}): Promise<AppletRun[]> {
	const params = new URLSearchParams();
	if (opts?.limit != null) params.set('limit', String(opts.limit));
	if (opts?.status) params.set('status', opts.status);
	if (opts?.applet_id) params.set('applet_id', opts.applet_id);
	const qs = params.toString();
	const res = await fetch(`${API_BASE}/runs${qs ? `?${qs}` : ''}`);
	if (!res.ok) throw new Error(`Failed to list runs: ${res.statusText}`);
	return res.json();
}

// ============================================================================
// Credentials
// ============================================================================

export interface DeviceInfo {
	device_id: string;
	device_name: string;
	device_model: string;
	os_version: string;
	app_version: string | null;
}

/**
 * Mirrors the CHECK constraint on `credentials.status`. `reauth_required` and
 * `error` are the states most worth surfacing — a Plaid `ITEM_LOGIN_REQUIRED`
 * arrives as one — and they were missing here, so the two rows that need a
 * Reconnect button were the two that rendered as unstyled raw text.
 */
export type CredentialStatus =
	| 'pending'
	| 'active'
	| 'revoked'
	| 'reauth_required'
	| 'error';

export interface Credential {
	id: string;
	provider: string;
	name: string;
	auth_type: string;
	status: CredentialStatus;
	/** The provider's own words for why a broken credential broke, when we
	 *  have them. Absent on healthy rows. */
	status_reason?: string | null;
	is_active: boolean;
	device_info: DeviceInfo | null;
	last_seen_at: string | null;
	created_at: string;
	applet_count: number;
	/** Tier-2 init-sync lifecycle for active credentials:
	 *  'connected' → 'backfilling' → 'live'. Absent for pending/revoked. */
	sync_state?: 'connected' | 'backfilling' | 'live';
}

export async function listCredentials(): Promise<Credential[]> {
	const res = await fetch(`${API_BASE}/credentials`);
	if (!res.ok) throw new Error(`Failed to list credentials: ${res.statusText}`);
	return res.json();
}

export async function renameCredential(id: string, name: string): Promise<void> {
	const res = await fetch(`${API_BASE}/credentials/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to rename credential: ${res.statusText}`);
	}
}

export async function revokeCredential(id: string): Promise<void> {
	const res = await fetch(`${API_BASE}/credentials/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to revoke credential: ${res.statusText}`);
	}
}

// Source catalog
//
// One tile per [[source]] in actions/templates.toml. Drives the Sources tab grid.
// `auth_kind` tells the UI which connect flow to dispatch on click.
export type SourceAuthKind = 'self_issued_bearer' | 'via_proxy' | 'api_key';


export interface SourceCatalogItem {
	id: string;
	name: string;
	icon: string | null;
	description: string | null;
	auth_kind: SourceAuthKind;
	credential_count: number;
	/** Secret names an `api_key` source expects, in manifest order. Empty for
	 *  every other auth kind. */
	fields: string[];
	/** Where this source's code can be read. Provenance only — the collectors
	 *  ship through the App Store and update themselves; no git ref installs
	 *  them. Null when the code is entirely the box's. */
	repo?: string | null;
	repo_ref?: string | null;
	/** Ontologies this source can produce, by display name — what connecting it
	 *  would actually give you. */
	provides?: string[];
	/** The life-domains those fall in (`health`, `financial`, …). */
	domains?: string[];
}

/**
 * Fetch the source catalog (one tile per `[[source]]` in templates.toml).
 */
export async function listSourceCatalog(): Promise<SourceCatalogItem[]> {
	const res = await fetch(`${API_BASE}/sources`);
	if (!res.ok) throw new Error(`Failed to list sources: ${res.statusText}`);
	return res.json();
}

/** One ingest stream's freshness. `status` is worst-first from the API. */
export interface StreamHealth {
	name: string;
	display_name: string;
	/** never = source never connected · live = data in last 24h ·
	 *  stalled = was flowing this week, silent for a day · idle = nothing in 7d ·
	 *  blocked = the applet writing this stream is failing, so the emptiness has
	 *  a known cause and must not be drawn as quiet. */
	status: 'never' | 'live' | 'stalled' | 'idle' | 'blocked';
	total: number;
	count_24h: number;
	count_7d: number;
	last_event: string | null;
	last_ingest: string | null;
	/** Sources that declare they write this stream, by display name. */
	provided_by: string[];
	/** At least one of those sources is connected. Distinguishes "nothing
	 *  provides this" from "provided, but switched off or not yet delivering". */
	connected: boolean;
	/** Why the stream is `blocked`, from the failing applet's last run. */
	blocked_reason?: string | null;
	/** No source writes it, yet rows exist — the box computes it from other
	 *  streams, so "connect something" is the wrong advice. */
	derived: boolean;
}

/**
 * Per-stream ingest freshness, worst-first. The signal that was missing while
 * messages, the calendar sync, and finance each went dark unnoticed.
 */
/** One stream's arrivals, one entry per day, oldest first and zero-filled. */
export interface StreamDays {
	name: string;
	display_name: string;
	/** The provider that actually wrote these rows (`source_provider` on the
	 *  data), not the manifest's claim. One entry per (stream, provider). */
	provider: string;
	days: number[];
}

/**
 * Daily arrival counts per stream. A scalar "last seen" cannot show whether
 * streams stopped *together* — nineteen rows reading the same date is one
 * event, not nineteen — and on a day axis that becomes a visible cliff.
 */
export interface StreamDaysResponse {
	/** UTC date that `days[0]` refers to. Labels come from this, not from a
	 *  local `new Date()` — deriving them locally shifts every tick by a day for
	 *  anyone west of UTC in the evening. */
	start: string;
	days: number;
	streams: StreamDays[];
}

export async function getStreamDays(days = 84): Promise<StreamDaysResponse> {
	return apiGet<StreamDaysResponse>(`/streams/days?days=${days}`);
}

export async function getStreamHealth(): Promise<StreamHealth[]> {
	return apiGet<StreamHealth[]>('/streams/health');
}


// ─────────────────────────────────────────────────────────────────────────────
// Source-connect flows (drive the 5 thin handlers in virtues-core/src/api/source_auth.rs)
// ─────────────────────────────────────────────────────────────────────────────

// ─── Unified pairing (`/api/pair/*`) ─────────────────────────────────────────
// One token mechanism for every device: the owner's authenticated session mints
// a token, the new device redeems it at `/api/pair/consume`. The phone scans the
// QR (`/pair#t=<token>`); the Mac app / collector takes the token directly.

export interface PairMintResponse {
	id: string;
	token: string;
	pair_url: string;
	qr_svg: string;
	expires_at: string;
}

/** POST /api/pair/mint — auth'd. Mint a `pending` token to add a device. */
export async function pairMint(intendedKind?: string): Promise<PairMintResponse> {
	const res = await fetch(`${API_BASE}/pair/mint`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ intended_kind: intendedKind ?? null })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `pair_mint failed: ${res.statusText}`);
	}
	return res.json();
}

/** DELETE /api/devices/:id — best-effort, no confirmation.
 *
 *  For rows WE created that the user never completed — the handoff enrolls a
 *  device before the phone has scanned anything, so an abandoned sheet would
 *  otherwise leave a live allowlisted key on the box. The interactive path is
 *  `revokeDeviceFlow`, which confirms and reports; this one is cleanup and
 *  stays quiet (409 on a last-remaining device is a legitimate refusal here,
 *  not something to surface). */
export async function revokeDevice(id: string): Promise<void> {
	await fetch(`${API_BASE}/devices/${encodeURIComponent(id)}`, { method: 'DELETE' }).catch(() => {
		/* benign — the row may already be gone */
	});
}

/** POST /api/pair/deny/:id — auth'd. Cancel an outstanding token (e.g. modal close). */
export async function pairDeny(id: string): Promise<void> {
	await fetch(`${API_BASE}/pair/deny/${encodeURIComponent(id)}`, { method: 'POST' }).catch(
		() => {
			/* benign — token may have already been consumed/expired */
		}
	);
}

export interface PairStatusResponse {
	status: string; // pending | authorized | consumed | denied | expired
	consumed_by_device: string | null;
	consumed_by_label: string | null;
}

/** GET /api/pair/status/:id — auth'd. Poll for the new device redeeming. */
export async function pairStatus(id: string): Promise<PairStatusResponse> {
	const res = await fetch(`${API_BASE}/pair/status/${encodeURIComponent(id)}`);
	if (!res.ok) throw new Error(`pair_status failed: ${res.statusText}`);
	return res.json();
}

export interface ChatImportResponse {
	status: string;
	summary: string;
	run_id: string | null;
}

/**
 * POST /api/chat-import/upload — multipart upload of a chat export (Tier 3
 * one-time import). Parsed + ingested box-side; returns the "Imported N
 * messages" summary once the run completes.
 */
export async function uploadChatImport(
	file: File,
	provider: string
): Promise<ChatImportResponse> {
	const form = new FormData();
	form.append('provider', provider);
	form.append('file', file);
	const res = await fetch(`${API_BASE}/chat-import/upload`, {
		method: 'POST',
		body: form
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `chat_import upload failed: ${res.statusText}`);
	}
	return res.json();
}

/** What chat-import has already landed on the box (empty = never imported). */
export interface ChatImportStatus {
	messages: number;
	conversations: number;
	/** Distinct providers seen in the imported rows, e.g. ["claude"]. */
	providers: string[];
}

/**
 * GET /api/chat-import/status — connected-state for the chat-import source,
 * which mints no credential; the imported rows themselves are the evidence.
 */
export async function getChatImportStatus(): Promise<ChatImportStatus> {
	const res = await fetch(`${API_BASE}/chat-import/status`);
	if (!res.ok) throw new Error(`Failed to read chat-import status: ${res.statusText}`);
	return res.json();
}

export interface MintCollectorResponse {
	token: string;
	expires_at: string;
}

/**
 * POST /api/pair/mint-collector — auth'd. Mint + self-authorize a token for
 * installing the local collector on THIS machine (handed to
 * `installCollector(token)` via the Tauri bridge).
 */
export async function mintCollectorToken(): Promise<MintCollectorResponse> {
	const res = await fetch(`${API_BASE}/pair/mint-collector`, { method: 'POST' });
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `mint_collector failed: ${res.statusText}`);
	}
	return res.json();
}

export interface OauthStartResponse {
	redirect_url: string;
}

/** POST /api/connect/:source_id/start — sign state, return proxy redirect URL. */
export async function oauthStart(
	source_id: string,
	opts: { existing_credential_id?: string; return_url?: string } = {}
): Promise<OauthStartResponse> {
	const res = await fetch(
		`${API_BASE}/connect/${encodeURIComponent(source_id)}/start`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(opts)
		}
	);
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `oauth_start failed: ${res.statusText}`);
	}
	return res.json();
}

export interface ApiKeyCompleteResponse {
	credential_id: string;
}

/** POST /api/connect/:source_id/complete — encrypt + store a pasted token. */
export async function apikeyComplete(
	source_id: string,
	name: string,
	fields: Record<string, string>
): Promise<ApiKeyCompleteResponse> {
	const res = await fetch(
		`${API_BASE}/connect/${encodeURIComponent(source_id)}/complete`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ name, fields })
		}
	);
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `apikey_complete failed: ${res.statusText}`);
	}
	return res.json();
}

// Device Pairing
import type {
	PairingInitResponse,
	PairingStatus,
	PendingPairing
} from '$lib/types/device-pairing';

/**
 * Initiate device pairing via the unified `/api/pair/mint` flow.
 *
 * Returns a `PairingInitResponse` whose `source_id` is the pair-token id (polled
 * via {@link getPairingStatus}), plus the QR/token redemption payload the new
 * device scans (`qr_svg` encodes `/pair#t=<token>`) or types (`token`).
 *
 * @param deviceType - "ios" → `mobile_app`, otherwise `desktop_app`.
 * @param _name - Display name (the device labels itself at consume time).
 */
export async function initiatePairing(
	deviceType: string,
	_name: string
): Promise<PairingInitResponse> {
	const intendedKind = deviceType === 'ios' ? 'mobile_app' : 'desktop_app';
	const minted = await pairMint(intendedKind);
	return {
		source_id: minted.id,
		token: minted.token,
		qr_svg: minted.qr_svg,
		pair_url: minted.pair_url
	};
}

/**
 * Poll pairing status by the token id. Maps the unified token lifecycle onto
 * the modal's `pending | active | revoked` shape: `consumed` → active (the new
 * device redeemed), `denied`/`expired` → revoked, everything else → pending.
 */
export async function getPairingStatus(sourceId: string): Promise<PairingStatus> {
	const s = await pairStatus(sourceId);
	if (s.status === 'consumed') {
		return {
			status: 'active',
			device_info: {
				device_id: s.consumed_by_device ?? '',
				device_name: s.consumed_by_label ?? '',
				device_model: '',
				os_version: ''
			}
		};
	}
	if (s.status === 'denied' || s.status === 'expired') {
		return { status: 'revoked' };
	}
	return { status: 'pending' };
}


// Profile
export interface Profile {
	preferred_name?: string | null;
	/** `"YYYY-MM-DD"` — serde's NaiveDate wire form, same as an HTML date input. */
	birth_date?: string | null;
	occupation?: string | null;
	employer?: string | null;
	theme?: string | null;
	update_check_hour?: number | null;
	home_timezone?: string | null;
	home_place_id?: string | null;
	/** Which wiki_people row is the owner (migration 0080). */
	self_person_id?: string | null;
	home_city?: string | null;
	home_country?: string | null;
	onboarding_status?: string | null;
	/** Getting-started sections dismissed from Home. Send the whole array —
	 *  the box remembers it verbatim; the client owns the merge. */
	getting_started_dismissed?: string[];
}

export async function getProfile(): Promise<Profile> {
	const res = await fetch(`${API_BASE}/profile`);
	if (!res.ok) throw new Error(`Failed to get profile: ${res.statusText}`);
	return res.json();
}

export async function updateProfile(profile: Partial<Profile>): Promise<Profile> {
	const res = await fetch(`${API_BASE}/profile`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(profile)
	});
	if (!res.ok) throw new Error(`Failed to update profile: ${res.statusText}`);
	return res.json();
}

// =============================================================================
// Drive - Personal File Storage
// =============================================================================

export interface DriveFile {
	id: string;
	path: string;
	filename: string;
	mime_type: string | null;
	size_bytes: number;
	is_folder: boolean;
	parent_id: string | null;
	sha256_hash: string | null;
	/** Universal-extraction state: pending | extracting | done | no_text | failed | skipped */
	extraction_status: string;
	deleted_at: string | null;
	created_at: string;
	updated_at: string;
}

/** Queue a file for (re-)extraction (retry after a failure, or after
 * installing a missing extractor). */
export async function reextractDriveFile(fileId: string): Promise<DriveFile> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}/reextract`, { method: 'POST' });
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || 'Failed to queue extraction');
	}
	return res.json();
}

// ── Annotations (document highlights + margin notes, researcher-plan D2) ──

export interface AnnotationRect {
	x: number;
	y: number;
	w: number;
	h: number;
}

export interface Annotation {
	id: string;
	file_id: string;
	page_num: number | null;
	quote_text: string;
	quote_prefix: string;
	quote_suffix: string;
	rects: AnnotationRect[];
	color: string;
	note_md: string;
	created_at: string;
	updated_at: string;
}

export async function listAnnotations(fileId: string): Promise<Annotation[]> {
	const res = await fetch(`${API_BASE}/annotations?file_id=${encodeURIComponent(fileId)}`);
	if (!res.ok) throw new Error(`Failed to list annotations: ${res.statusText}`);
	return res.json();
}

/** A highlight enriched with its file's name, for the notebook Highlights tab. */
/** A file's highlights as markdown (blockquote + citation ref each). */
export async function exportFileAnnotations(fileId: string): Promise<string> {
	const res = await fetch(`${API_BASE}/annotations/export?file_id=${encodeURIComponent(fileId)}`);
	if (!res.ok) throw new Error(`Failed to export annotations: ${res.statusText}`);
	return res.text();
}

/** Trigger a client-side download of markdown text. */
export function downloadMarkdown(filename: string, markdown: string): void {
	const url = URL.createObjectURL(new Blob([markdown], { type: 'text/markdown' }));
	const a = document.createElement('a');
	a.href = url;
	a.download = filename.endsWith('.md') ? filename : `${filename}.md`;
	a.click();
	URL.revokeObjectURL(url);
}

export async function createAnnotation(body: {
	file_id: string;
	page_num?: number | null;
	quote_text: string;
	quote_prefix?: string;
	quote_suffix?: string;
	rects?: AnnotationRect[];
	color?: string;
	note_md?: string;
}): Promise<Annotation> {
	const res = await fetch(`${API_BASE}/annotations`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) {
		const e = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(e.error || 'Failed to create annotation');
	}
	return res.json();
}

export async function updateAnnotation(
	id: string,
	body: { color?: string; note_md?: string }
): Promise<Annotation> {
	const res = await fetch(`${API_BASE}/annotations/${id}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Failed to update annotation: ${res.statusText}`);
	return res.json();
}

export async function deleteAnnotation(id: string): Promise<void> {
	const res = await fetch(`${API_BASE}/annotations/${id}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`Failed to delete annotation: ${res.statusText}`);
}

export interface DriveUsage {
	/** Total bytes used (drive_bytes + data_lake_bytes) */
	total_bytes: number;
	/** User-uploaded files in /home/user/drive/ */
	drive_bytes: number;
	/** ELT archives in /home/user/data-lake/ */
	data_lake_bytes: number;
	/** Total capacity of the box's disk */
	quota_bytes: number;
	/** Free space actually left on that disk (other data lives there too) */
	available_bytes: number;
	/** Number of user files */
	file_count: number;
	/** Number of user folders */
	folder_count: number;
	/** Usage percentage (total_bytes / quota_bytes * 100) */
	usage_percent: number;
}

/**
 * Get drive storage usage and quota information
 */
export async function getDriveUsage(): Promise<DriveUsage> {
	const res = await fetch(`${API_BASE}/drive/usage`);
	if (!res.ok) throw new Error(`Failed to get drive usage: ${res.statusText}`);
	return res.json();
}

export interface BackupVolumeStatus {
	id: string;
	name: string;
	/** Whether the drive is plugged in right now, resolved live. */
	attached: boolean;
	last_ok_at: string | null;
	age_seconds: number | null;
	last_error: string | null;
}

export interface BackupStatus {
	/** none | never | ok | stale | failing */
	state: string;
	/** Seconds since the newest successful backup on any volume. */
	age_seconds: number | null;
	volumes: BackupVolumeStatus[];
}

/**
 * Backup freshness. The one number that matters is `age_seconds` — if the box
 * died now, that is how much would be lost.
 */
export async function getBackupStatus(): Promise<BackupStatus> {
	const res = await fetch(`${API_BASE}/backup/status`);
	if (!res.ok) throw new Error(`Failed to get backup status: ${res.statusText}`);
	return res.json();
}

/**
 * List files in a directory
 * @param path - Directory path (empty string for root)
 */
export async function listDriveFiles(path: string = ''): Promise<DriveFile[]> {
	const params = new URLSearchParams();
	if (path) params.set('path', path);

	const res = await fetch(`${API_BASE}/drive/files?${params}`);
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to list files: ${res.statusText}`);
	}
	return res.json();
}

/**
 * Get file metadata by ID
 */
export async function getDriveFile(fileId: string): Promise<DriveFile> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}`);
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to get file: ${res.statusText}`);
	}
	return res.json();
}

/** One raw life-graph record (the data viewer / citation target). */
export interface OntologyRecord {
	ontology: string;
	record_id: string;
	display_name: string;
	table_name: string;
	timestamp_column: string;
	/** The full row as a plain object (all columns). */
	row: Record<string, unknown>;
}

/** Fetch a single raw record by ontology + id — backs the data viewer. */
export async function getRecord(ontology: string, recordId: string): Promise<OntologyRecord> {
	const res = await fetch(
		`${API_BASE}/records/${encodeURIComponent(ontology)}/${encodeURIComponent(recordId)}`
	);
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to get record: ${res.statusText}`);
	}
	return res.json();
}

/**
 * Upload a file to drive
 * @param path - Target folder path
 * @param file - File to upload
 * @param onProgress - Optional progress callback (0-100)
 */
export async function uploadDriveFile(
	path: string,
	file: File,
	onProgress?: (percent: number) => void
): Promise<DriveFile> {
	const formData = new FormData();
	formData.append('file', file);
	formData.append('path', path);
	formData.append('filename', file.name);

	// Use XMLHttpRequest for progress tracking
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open('POST', `${API_BASE}/drive/upload`);

		xhr.upload.onprogress = (e) => {
			if (e.lengthComputable && onProgress) {
				onProgress(Math.round((e.loaded / e.total) * 100));
			}
		};

		xhr.onload = () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				resolve(JSON.parse(xhr.responseText));
			} else {
				try {
					const error = JSON.parse(xhr.responseText);
					reject(new Error(error.error || `Upload failed: ${xhr.statusText}`));
				} catch {
					reject(new Error(`Upload failed: ${xhr.statusText}`));
				}
			}
		};

		xhr.onerror = () => reject(new Error('Upload failed: network error'));
		xhr.send(formData);
	});
}

/**
 * Download a file from drive
 */
export async function downloadDriveFile(fileId: string): Promise<{ file: DriveFile; blob: Blob }> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}/download`);
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to download file: ${res.statusText}`);
	}

	// Get filename from Content-Disposition header
	const contentDisposition = res.headers.get('Content-Disposition');
	let filename = 'download';
	if (contentDisposition) {
		const match = contentDisposition.match(/filename="?([^";\n]+)"?/);
		if (match) filename = match[1];
	}

	const blob = await res.blob();
	return {
		file: { filename } as DriveFile,
		blob
	};
}

/**
 * Delete a file or folder
 */
export async function deleteDriveFile(fileId: string): Promise<void> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}`, { method: 'DELETE' });
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to delete file: ${res.statusText}`);
	}
}

/**
 * Create a folder
 */
export async function createDriveFolder(path: string, name: string): Promise<DriveFile> {
	const res = await fetch(`${API_BASE}/drive/folders`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ path, name })
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to create folder: ${res.statusText}`);
	}
	return res.json();
}

/**
 * Move or rename a file/folder
 */
export async function moveDriveFile(fileId: string, newPath: string): Promise<DriveFile> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}/move`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ new_path: newPath })
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to move file: ${res.statusText}`);
	}
	return res.json();
}

/**
 * List files in trash
 */
export async function listDriveTrash(): Promise<DriveFile[]> {
	const res = await fetch(`${API_BASE}/drive/trash`);
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to list trash: ${res.statusText}`);
	}
	return res.json();
}

/**
 * Restore a file from trash
 */
export async function restoreDriveFile(fileId: string): Promise<DriveFile> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}/restore`, {
		method: 'POST'
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to restore file: ${res.statusText}`);
	}
	return res.json();
}

/**
 * Permanently delete a file (skip trash)
 */
export async function purgeDriveFile(fileId: string): Promise<void> {
	const res = await fetch(`${API_BASE}/drive/files/${fileId}/purge`, {
		method: 'DELETE'
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to permanently delete file: ${res.statusText}`);
	}
}

/**
 * Empty entire trash (permanently delete all trashed files)
 */
export async function emptyDriveTrash(): Promise<{ deleted_count: number }> {
	const res = await fetch(`${API_BASE}/drive/trash/empty`, {
		method: 'POST'
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to empty trash: ${res.statusText}`);
	}
	return res.json();
}

// =============================================================================
// Media - Content-addressed storage for page-embedded media
// =============================================================================

export interface MediaFile {
	id: string;
	url: string;
	filename: string;
	mime_type: string | null;
	size_bytes: number;
	width: number | null;
	height: number | null;
	deduplicated: boolean;
}

/**
 * Upload a media file (image, video, audio) for embedding in pages.
 * Uses content-addressed storage - duplicate uploads return existing file.
 */
export async function uploadMedia(
	file: File,
	onProgress?: (percent: number) => void
): Promise<MediaFile> {
	const formData = new FormData();
	formData.append('file', file);
	formData.append('filename', file.name);

	// Use XMLHttpRequest for progress tracking
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open('POST', `${API_BASE}/media/upload`);

		xhr.upload.onprogress = (e) => {
			if (e.lengthComputable && onProgress) {
				onProgress(Math.round((e.loaded / e.total) * 100));
			}
		};

		xhr.onload = () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				resolve(JSON.parse(xhr.responseText));
			} else {
				try {
					const error = JSON.parse(xhr.responseText);
					reject(new Error(error.error || `Upload failed: ${xhr.statusText}`));
				} catch {
					reject(new Error(`Upload failed: ${xhr.statusText}`));
				}
			}
		};

		xhr.onerror = () => reject(new Error('Network error during upload'));
		xhr.send(formData);
	});
}

// =============================================================================
// Chats - Chat Management
// =============================================================================

export interface ChatMessage {
	role: 'user' | 'assistant' | 'system';
	content: string;
	timestamp: string;
}

/**
 * Update a chat (title and/or icon)
 */
export async function updateChat(
	chatId: string,
	updates: {
		title?: string;
		icon?: string | null;
		icon_color?: string | null;
		notebookId?: string | null;
	}
): Promise<{
	conversation_id: string;
	title: string;
	icon?: string | null;
	icon_color?: string | null;
	updated_at: string;
}> {
	const res = await fetch(`${API_BASE}/chats/${chatId}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(updates)
	});

	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to update chat: ${res.statusText}`);
	}

	return res.json();
}

/**
 * Delete a chat
 */
export async function deleteChat(chatId: string): Promise<{ deleted: boolean }> {
	const res = await fetch(`${API_BASE}/chats/${chatId}`, {
		method: 'DELETE'
	});

	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to delete chat: ${res.statusText}`);
	}

	return res.json();
}

// =============================================================================
// Notebooks API — the "room" a chat lives in
//
// A Notebook is a manual collection the user returns to: a project, pet, hobby,
// goal, or topic. It gathers entities, chats, and pages as URL-native members
// and carries a single accent tint plus a catch-up memo (`current_status`).
// A chat lives in at most one Notebook (see `updateChat`'s `notebookId`).
// =============================================================================

/** Core Notebook row (no counts). Returned by create/update. */
export interface Notebook {
	id: string;
	name: string;
	icon: string | null;
	accent_color: string | null;
	current_status: string | null;
	current_status_at: string | null;
	instructions: string | null;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

/** List-view summary — adds member and chat counts. */
export interface NotebookSummary extends Notebook {
	item_count: number;
	chat_count: number;
}

/**
 * What a member is to the notebook.
 *  - `library`    grounds chat (the default for anything you add)
 *  - `manuscript` yours to write — kept out of retrieval, so a draft is never
 *                 cited back at you as if it were a source
 *  - `pin`        nav-only shortcut
 */
export type NotebookItemRole = 'library' | 'manuscript' | 'pin';

/** A single URL-native member of a Notebook. */
export interface NotebookItem {
	url: string;
	sort_order: number;
	role: NotebookItemRole;
	added_at: string;
}

/** One entity referenced across a notebook's members. */
export interface NotebookGraphNode {
	/** Ref URL, e.g. `/person/pe_abc` — also the node's identity. */
	url: string;
	entity_type: string;
	name: string;
	/** Member urls referencing this entity; drives click-to-filter. */
	item_urls: string[];
}

/** Two entities that share at least one member. */
export interface NotebookGraphEdge {
	source: string;
	target: string;
	weight: number;
}

export interface NotebookGraph {
	nodes: NotebookGraphNode[];
	edges: NotebookGraphEdge[];
}

/** GET /api/notebooks/:id — a Notebook plus its ordered members. */
export interface NotebookDetail extends Notebook {
	items: NotebookItem[];
}

/** GET /api/notebooks — all Notebooks with counts. */
export async function listNotebooks(): Promise<{ notebooks: NotebookSummary[] }> {
	const res = await fetch(`${API_BASE}/notebooks`);
	if (!res.ok) throw new Error(`Failed to list notebooks: ${res.statusText}`);
	return res.json();
}

/** GET /api/notebooks/:id — a Notebook with its ordered members. */
export async function getNotebook(id: string): Promise<NotebookDetail> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(id)}`);
	if (!res.ok) throw new Error(`Failed to get notebook: ${res.statusText}`);
	return res.json();
}

/** POST /api/notebooks — create a Notebook. */
export async function createNotebook(body: {
	name: string;
	icon?: string | null;
	accent_color?: string | null;
}): Promise<Notebook> {
	const res = await fetch(`${API_BASE}/notebooks`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Failed to create notebook: ${res.statusText}`);
	return res.json();
}

/**
 * PUT /api/notebooks/:id — update a Notebook. For the nullable fields
 * (`icon`/`accent_color`/`current_status`): omit the key to leave unchanged,
 * send `null` to clear, send a value to set.
 */
export async function updateNotebook(
	id: string,
	patch: {
		name?: string;
		icon?: string | null;
		accent_color?: string | null;
		current_status?: string | null;
		instructions?: string | null;
		sort_order?: number;
	}
): Promise<Notebook> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(id)}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(patch)
	});
	if (!res.ok) throw new Error(`Failed to update notebook: ${res.statusText}`);
	return res.json();
}

/** DELETE /api/notebooks/:id */
export async function deleteNotebook(id: string): Promise<void> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(id)}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`Failed to delete notebook: ${res.statusText}`);
}

// =============================================================================
// View Entity (sidebar smart-section rows — resolved client-side from
// /api/chats and listPages; the folder/view CRUD API was removed.)
// =============================================================================

export interface ViewEntity {
	id: string;
	name: string;
	namespace: string;
	icon: string;
	updated_at?: string;
}

// =============================================================================
// Developer SQL API
// =============================================================================


// =============================================================================
// Notebook Items API — the URL-native members of a Notebook
//
// (Listing comes back inside `getNotebook(id)` as `NotebookDetail.items`; there is
// no separate GET.)
// =============================================================================

/** POST /api/notebooks/:id/items — add a member URL to a Notebook. */
export async function addNotebookItem(notebookId: string, url: string): Promise<NotebookItem> {
	const sanitizedUrl = sanitizeUrl(url);
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(notebookId)}/items`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ url: sanitizedUrl })
	});
	if (!res.ok) throw new Error(`Failed to add notebook item: ${res.statusText}`);
	return res.json();
}

/** DELETE /api/notebooks/:id/items — remove a member URL from a Notebook. */
export async function removeNotebookItem(notebookId: string, url: string): Promise<void> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(notebookId)}/items`, {
		method: 'DELETE',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ url })
	});
	if (!res.ok) throw new Error(`Failed to remove notebook item: ${res.statusText}`);
}

/** PUT /api/notebooks/:id/items/reorder — set the member order by URL. */
export async function reorderNotebookItems(notebookId: string, urls: string[]): Promise<void> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(notebookId)}/items/reorder`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ urls })
	});
	if (!res.ok) throw new Error(`Failed to reorder notebook items: ${res.statusText}`);
}

/** PUT /api/notebooks/:id/items/role — set what a member is to the notebook. */
export async function setNotebookItemRole(
	notebookId: string,
	url: string,
	role: NotebookItemRole
): Promise<NotebookItem> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(notebookId)}/items/role`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ url, role })
	});
	if (!res.ok) throw new Error(`Failed to set member role: ${res.statusText}`);
	return res.json();
}

/**
 * GET /api/notebooks/:id/graph — entities referenced across the members.
 * Built only from things explicitly filed or linked (`[@ref]`); nothing is
 * inferred, so an entity merely mentioned inside a PDF will not appear.
 */
export async function getNotebookGraph(notebookId: string): Promise<NotebookGraph> {
	const res = await fetch(`${API_BASE}/notebooks/${encodeURIComponent(notebookId)}/graph`);
	if (!res.ok) throw new Error(`Failed to load notebook graph: ${res.statusText}`);
	return res.json();
}

// =============================================================================
// Pages API
// =============================================================================

export interface Page {
	id: string;
	title: string;
	content: string;
	notebook_id: string | null;
	icon: string | null;
	/** `--cat-*` token key ('orange', 'emerald'), never a hex. Migration 0079. */
	icon_color: string | null;
	cover_url: string | null;
	tags: string | null; // JSON array string: '["tag1", "tag2"]'
	created_at: string;
	updated_at: string;
}

export interface PageSummary {
	id: string;
	title: string;
	notebook_id: string | null;
	icon: string | null;
	icon_color: string | null;
	cover_url: string | null;
	tags: string | null; // JSON array string: '["tag1", "tag2"]'
	created_at: string;
	updated_at: string;
}

export interface PageListResponse {
	pages: PageSummary[];
	total: number;
	limit: number;
	offset: number;
}

export interface RefSearchResult {
	id: string;
	name: string;
	entity_type: string;
	icon: string;
}

export interface RefSearchResponse {
	results: RefSearchResult[];
}

/**
 * List all pages with optional pagination and workspace filter
 */
export async function listPages(limit?: number, offset?: number, notebook_id?: string): Promise<PageListResponse> {
	const params = new URLSearchParams();
	if (limit !== undefined) params.set('limit', String(limit));
	if (offset !== undefined) params.set('offset', String(offset));
	if (notebook_id !== undefined) params.set('notebook_id', notebook_id);

	const url = params.toString() ? `${API_BASE}/pages?${params}` : `${API_BASE}/pages`;
	const res = await fetch(url);

	if (!res.ok) throw new Error(`Failed to list pages: ${res.statusText}`);
	return res.json();
}

/**
 * Get a single page by ID
 */
export async function getPage(id: string): Promise<Page> {
	const res = await fetch(`${API_BASE}/pages/${id}`);
	if (!res.ok) throw new Error(`Failed to get page: ${res.statusText}`);
	return res.json();
}

/**
 * Create a new page
 */
export async function createPage(
	title: string,
	content: string = '',
	notebook_id: string | null = null,
	options?: { icon?: string; cover_url?: string; tags?: string }
): Promise<Page> {
	const res = await fetch(`${API_BASE}/pages`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ title, content, notebookId: notebook_id, ...options })
	});

	if (!res.ok) throw new Error(`Failed to create page: ${res.statusText}`);
	return res.json();
}

/**
 * Update an existing page
 */
export async function updatePage(
	id: string,
	updates: {
		title?: string;
		content?: string;
		notebook_id?: string | null;
		icon?: string | null;
		icon_color?: string | null;
		cover_url?: string | null;
		tags?: string | null;
	}
): Promise<Page> {
	const res = await fetch(`${API_BASE}/pages/${id}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(updates)
	});

	if (!res.ok) throw new Error(`Failed to update page: ${res.statusText}`);
	return res.json();
}

/**
 * Delete a page by ID
 */
export async function deletePage(id: string): Promise<void> {
	const res = await fetch(`${API_BASE}/pages/${id}`, {
		method: 'DELETE'
	});

	if (!res.ok) throw new Error(`Failed to delete page: ${res.statusText}`);
}

/**
 * Search entities for autocomplete in the page editor
 * Used when typing [[ to link to entities
 */
export async function searchRefs(query: string): Promise<RefSearchResponse> {
	const res = await fetch(`${API_BASE}/pages/search/refs?q=${encodeURIComponent(query)}`);
	if (!res.ok) throw new Error(`Failed to search entities: ${res.statusText}`);
	return res.json();
}

// Page Sharing
export interface PageShare {
	id: string;
	page_id: string;
	token: string;
	created_at: string;
}

export interface SharedPage {
	title: string;
	content: string;
	icon: string | null;
	cover_url: string | null;
	share_token: string;
}

/**
 * Append a markdown block to a page THROUGH Yjs (researcher-plan D4).
 *
 * Safe when the page is open in an editor: the server applies the insert to the
 * authoritative Yjs doc and broadcasts it, so an open editor merges the block
 * instead of being clobbered by a content replace.
 */
export async function appendToPage(pageId: string, markdown: string): Promise<{ content: string }> {
	const res = await fetch(`${API_BASE}/pages/${encodeURIComponent(pageId)}/append`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ markdown })
	});
	if (!res.ok) {
		const e = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(e.error || 'Failed to append to page');
	}
	return res.json();
}

export async function createPageShare(pageId: string): Promise<PageShare> {
	const res = await fetch(`${API_BASE}/pages/${pageId}/share`, { method: 'POST' });
	if (!res.ok) throw new Error(`Failed to create share: ${res.statusText}`);
	return res.json();
}

export async function getPageShare(pageId: string): Promise<PageShare | null> {
	const res = await fetch(`${API_BASE}/pages/${pageId}/share`);
	if (!res.ok) throw new Error(`Failed to get share: ${res.statusText}`);
	return await res.json() ?? null;
}

export async function deletePageShare(pageId: string): Promise<void> {
	const res = await fetch(`${API_BASE}/pages/${pageId}/share`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`Failed to delete share: ${res.statusText}`);
}

export async function getSharedPage(token: string): Promise<SharedPage> {
	const res = await fetch(`${API_BASE}/s/${token}`);
	if (!res.ok) throw new Error(`Page not found`);
	return res.json();
}

// ============================================================================
// Backlinks / References API
// ============================================================================

/** A page that links TO the queried page (an inbound reference). */
export interface Backlink {
	id: string;
	title: string;
	icon: string | null;
	/** One-line plain-text snippet of the surrounding context. */
	snippet: string;
	updated_at: string;
}

/** Get inbound references (pages that link to the given page). */
export async function getPageBacklinks(pageId: string): Promise<Backlink[]> {
	const res = await fetch(`${API_BASE}/pages/${pageId}/backlinks`);
	if (!res.ok) throw new Error(`Failed to get backlinks: ${res.statusText}`);
	const data = await res.json();
	return data.backlinks ?? [];
}

// ============================================================================
// Ontologies API
// ============================================================================

export interface OntologyColumnInfo {
	name: string;
	data_type: string;
	is_nullable: boolean;
}

export interface OntologyDataResponse {
	table_name: string;
	display_name: string;
	domain: string;
	columns: OntologyColumnInfo[];
	key_columns: string[];
	timestamp_column: string;
	rows: Record<string, unknown>[];
	total_count: number;
	limit: number;
	offset: number;
}

export interface OntologyOverview {
	name: string;
	domain: string;
	record_count: number;
	sample_record: Record<string, unknown> | null;
}

export async function getOntologiesOverview(): Promise<OntologyOverview[]> {
	const res = await fetch(`${API_BASE}/ontologies/overview`);
	if (!res.ok) throw new Error(`Failed to get ontologies overview: ${res.statusText}`);
	return res.json();
}

export async function queryOntologyData(
	tableName: string,
	params?: {
		limit?: number;
		offset?: number;
		sort?: string;
		dir?: string;
		date?: string;
		search?: string;
	},
): Promise<OntologyDataResponse> {
	const searchParams = new URLSearchParams();
	if (params?.limit != null) searchParams.set('limit', String(params.limit));
	if (params?.offset != null) searchParams.set('offset', String(params.offset));
	if (params?.sort) searchParams.set('sort', params.sort);
	if (params?.dir) searchParams.set('dir', params.dir);
	if (params?.date) searchParams.set('date', params.date);
	if (params?.search) searchParams.set('search', params.search);

	const qs = searchParams.toString();
	const res = await fetch(`${API_BASE}/ontologies/${tableName}/data${qs ? `?${qs}` : ''}`);
	if (!res.ok) throw new Error(`Failed to query ontology data: ${res.statusText}`);
	return res.json();
}

// ============================================================================
// Setup state API
// ============================================================================

export interface SetupStep {
	id: string;
	title: string;
	done: boolean;
	/** Server-authored copy for the step's current state — render verbatim. */
	detail?: string;
	/**
	 * Cosmetic hint only (e.g. "ipv6_direct", "byo", or a network class).
	 * Behavior must key off `done`; unknown/missing kinds render like today.
	 */
	kind?: string;
}

/** A collector that is paired and running but missing a permission it needs. */
export interface DegradedCollector {
	device_id: string;
	label?: string | null;
	/** Capability names the collector reports NOT having, e.g. "full_disk_access". */
	denied: string[];
	stale: boolean;
	checked_at?: string | null;
}

export interface SetupState {
	setup: SetupStep[];
	/** The box is up: claimed, and linked to an account on an appliance. */
	setup_complete: boolean;
	onboarding: SetupStep[];
	/** The box has something to keep a record of — at least one source. */
	onboarding_complete: boolean;
	/** Collectors running with a denied permission — a ✓ over one of these lies. */
	degraded?: DegradedCollector[];
	/** The interview has an answer but no document yet — show "underway". */
	interview_started?: boolean;
	/** `new` | `onboarding` | `active`. `active` means finished OR dismissed —
	 *  both stop the redirect. Backed by app_user_profile.onboarding_status,
	 *  which already tracked this life stage before a second flag was added. */
	onboarding_status: string;
}

/**
 * Remember that onboarding was declined — or offer it again with `false`.
 *
 * The app routes on this, so a skip that wasn't persisted would put the same
 * page back in front of someone on every launch.
 */
export async function skipOnboarding(skipped = true): Promise<void> {
	const res = await fetch(`${API_BASE}/setup/skip-onboarding`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ skipped }),
	});
	if (!res.ok) throw new Error(`Failed to record onboarding choice: ${res.statusText}`);
}

export async function getSetupState(): Promise<SetupState> {
	const res = await fetch(`${API_BASE}/setup/state`);
	if (!res.ok) throw new Error(`Failed to get setup state: ${res.statusText}`);
	return res.json();
}


// ============================================================================
// Tier 2 wrappers — routed through the shared request() layer (ApiError + status).
// Inputs are typed precisely; outputs use a generic passthrough (<T = unknown>)
// so each call site reuses the response type it already declares locally.
// ============================================================================

// ── Assistant profile ────────────────────────────────────────────────────────
// ── Assistant memories (What I've learned) ──
// The machine's per-note memory, visible and editable. Every live note rides
// in the system prompt of every conversation.

export interface AssistantMemory {
	id: number;
	/** facts | manner | practices */
	lane: string;
	/** 'ai' until the person edits — then 'human', and the machine can no longer revise it. */
	author: string;
	body: string;
	created_at: string;
	updated_at: string;
}

export async function listAssistantMemories(): Promise<AssistantMemory[]> {
	const res = await fetch(`${API_BASE}/assistant/memories`);
	if (!res.ok) throw new Error(`Failed to load memories: ${res.statusText}`);
	return res.json();
}

/** Rewrite one memory in your own words. It becomes yours. */
export async function editAssistantMemory(id: number, body: string): Promise<AssistantMemory> {
	const res = await fetch(`${API_BASE}/assistant/memories/${id}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ body }),
	});
	if (!res.ok) throw new Error(`Failed to edit memory: ${res.statusText}`);
	return res.json();
}

/** Remove a memory from every future conversation (soft-retired with provenance). */
export async function retireAssistantMemory(id: number): Promise<void> {
	const res = await fetch(`${API_BASE}/assistant/memories/${id}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`Failed to remove memory: ${res.statusText}`);
}

export function getAssistantProfile<T = unknown>(): Promise<T> {
	return apiGet<T>('/assistant-profile');
}
export function updateAssistantProfile<T = unknown>(patch: Record<string, unknown>): Promise<T> {
	return apiSend<T>('PUT', '/assistant-profile', patch);
}

// ── Billing / wallet ─────────────────────────────────────────────────────────
export function getBillingLinkStatus<T = unknown>(): Promise<T> {
	return apiGet<T>('/billing/link/status');
}
export function startBillingLink<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/billing/link/start');
}
export function openBillingPortal<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/billing/portal');
}
/** A Stripe Checkout URL for the account this box is linked to. */
export function subscribeBilling<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/billing/subscribe');
}
export function getBillingState<T = unknown>(): Promise<T> {
	return apiGet<T>('/billing/state');
}
export function setBillingAutoTopup<T = unknown>(enabled: boolean): Promise<T> {
	return apiSend<T>('POST', '/billing/auto-topup', { enabled });
}
export function getBillingUsage<T = unknown>(): Promise<T> {
	return apiGet<T>('/billing/usage');
}
export function getByoKey<T = unknown>(): Promise<T> {
	return apiGet<T>('/settings/byo-key');
}
export function setByoKey<T = unknown>(body: Record<string, unknown>): Promise<T> {
	return apiSend<T>('POST', '/settings/byo-key', body);
}
export function deleteByoKey<T = unknown>(sudoRequestId?: string): Promise<T> {
	return apiSend<T>('DELETE', '/settings/byo-key', { sudo_request_id: sudoRequestId });
}

// ── Personas ─────────────────────────────────────────────────────────────────
export function listPersonas<T = unknown>(): Promise<T> {
	return apiGet<T>('/personas');
}
export function createPersona<T = unknown>(body: { title: string; content: string }): Promise<T> {
	return apiSend<T>('POST', '/personas', body);
}
export function updatePersona<T = unknown>(id: string, updates: object): Promise<T> {
	return apiSend<T>('PUT', `/personas/${encodeURIComponent(id)}`, updates);
}
export function deletePersona<T = unknown>(id: string): Promise<T> {
	return apiSend<T>('DELETE', `/personas/${encodeURIComponent(id)}`);
}
export function unhidePersona<T = unknown>(id: string): Promise<T> {
	return apiSend<T>('POST', `/personas/${encodeURIComponent(id)}/unhide`);
}
export function resetPersonas<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/personas/reset');
}

// ── Chats (extras beyond createChat/updateChat/deleteChat above) ──────────────
export function listChats<T = unknown>(): Promise<T> {
	return apiGet<T>('/chats');
}
export function getChat<T = unknown>(id: string): Promise<T> {
	return apiGet<T>(`/chats/${encodeURIComponent(id)}`);
}
export function getChatUsage<T = unknown>(id: string): Promise<T> {
	return apiGet<T>(`/chats/${encodeURIComponent(id)}/usage`);
}
export function setChatTitle<T = unknown>(body: Record<string, unknown>): Promise<T> {
	return apiSend<T>('POST', '/chats/title', body);
}
export function cancelChat<T = unknown>(chatId: string): Promise<T> {
	return apiSend<T>('POST', '/chat/cancel', { chatId });
}
export function compactChat<T = unknown>(id: string, force = true): Promise<T> {
	return apiSend<T>('POST', `/chats/${encodeURIComponent(id)}/compact`, { force });
}
export function getChatPermissions<T = unknown>(id: string): Promise<T> {
	return apiGet<T>(`/chats/${encodeURIComponent(id)}/permissions`);
}
export function addChatPermission<T = unknown>(id: string, body: Record<string, unknown>): Promise<T> {
	return apiSend<T>('POST', `/chats/${encodeURIComponent(id)}/permissions`, body);
}
export function removeChatPermission<T = unknown>(id: string, entityId: string): Promise<T> {
	return apiSend<T>(
		'DELETE',
		`/chats/${encodeURIComponent(id)}/permissions/${encodeURIComponent(entityId)}`,
	);
}

// ── Models ───────────────────────────────────────────────────────────────────
export function listModels<T = unknown>(): Promise<T> {
	return apiGet<T>('/models');
}
export function getModel<T = unknown>(id: string): Promise<T> {
	return apiGet<T>(`/models/${encodeURIComponent(id)}`);
}
export function getRecommendedModels<T = unknown>(): Promise<T> {
	return apiGet<T>('/models/recommended');
}

// ── Page versions (yjs history) ──────────────────────────────────────────────
export function createPageVersion<T = unknown>(pageId: string, body: Record<string, unknown>): Promise<T> {
	return apiSend<T>('POST', `/pages/${encodeURIComponent(pageId)}/versions`, body);
}
export function listPageVersions<T = unknown>(pageId: string, limit?: number): Promise<T> {
	return apiGet<T>(`/pages/${encodeURIComponent(pageId)}/versions`, { limit });
}
export function getPageVersion<T = unknown>(versionId: string): Promise<T> {
	return apiGet<T>(`/pages/versions/${encodeURIComponent(versionId)}`);
}

// ── Setup (extras beyond getSetupState) ──────────────────────────────────────
export function setupLinkPoll<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/setup/link/poll');
}
export function setupSubscribeStart<T = unknown>(): Promise<T> {
	return apiSend<T>('POST', '/setup/subscribe/start');
}
export function setupLoginStart<T = unknown>(email: string): Promise<T> {
	return apiSend<T>('POST', '/setup/login/start', { email });
}

// ── Sudo (privilege elevation) ───────────────────────────────────────────────
export function requestSudo<T = unknown>(action: string, actionPayload?: unknown): Promise<T> {
	return apiSend<T>('POST', '/sudo/request', { action, action_payload: actionPayload });
}
export function getSudoStatus<T = unknown>(id: string): Promise<T> {
	return apiGet<T>(`/sudo/status/${encodeURIComponent(id)}`);
}

// ── Data lake ────────────────────────────────────────────────────────────────
export function getLakeSummary<T = unknown>(): Promise<T> {
	return apiGet<T>('/lake/summary');
}
export function getLakeStreams<T = unknown>(): Promise<T> {
	return apiGet<T>('/lake/streams');
}

// ── System / telemetry / usage ───────────────────────────────────────────────
export function getSystemTelemetry<T = unknown>(): Promise<T> {
	return apiGet<T>('/system/telemetry');
}
/** One row of the box-local AI-call log. Metadata only — never content. */
export interface AiCallRow {
	id: string;
	created_at: string;
	feature: string | null;
	model: string | null;
	prompt_tokens: number;
	completion_tokens: number;
	reasoning_tokens: number;
	/**
	 * Micros-USD, authoritative ONLY when `route === 'wallet'`. Our gateway is
	 * the only upstream that reports a price, so a BYO row is 0-as-unknown —
	 * render it as "your key", never as "$0.00".
	 */
	cost_micros: number;
	/** `wallet` | `byo` — which purse paid. */
	route: string;
}

/** GET /api/telemetry/ai-calls — one page of the call log, newest first. */
export function getAiCallsPage(opts: {
	offset: number;
	limit: number;
	search?: string;
	dir?: 'asc' | 'desc';
}): Promise<{ items: AiCallRow[]; total: number }> {
	return apiGet<{ items: AiCallRow[]; total: number }>('/telemetry/ai-calls', {
		offset: opts.offset,
		limit: opts.limit,
		search: opts.search,
		dir: opts.dir
	});
}
export function getUsageSummary<T = unknown>(): Promise<T> {
	return apiGet<T>('/usage/summary');
}
export function getSubscription<T = unknown>(fresh = false): Promise<T> {
	return apiGet<T>('/subscription', fresh ? { fresh: 1 } : undefined);
}

// ── Narrative identity (wiki) ────────────────────────────────────────────────
// Read-only: the document is edited on its page. (The PUT died with the
// retired abridged copy, 2026-09-01.)
export function getNarrativeIdentity<T = unknown>(): Promise<T> {
	return apiGet<T>('/wiki/narrative-identity');
}

// ── Devices / pairing (extras beyond pairMint/pairDeny/pairStatus) ────────────
export function listDevices<T = unknown>(): Promise<T> {
	return apiGet<T>('/devices');
}

// ── Misc singletons ──────────────────────────────────────────────────────────
export function getDeveloperTables<T = unknown>(): Promise<T> {
	return apiGet<T>('/developer/tables');
}
export function getDriveMedia<T = unknown>(): Promise<T> {
	return apiGet<T>('/drive/media');
}
export function searchUnsplash<T = unknown>(body: Record<string, unknown>): Promise<T> {
	return apiSend<T>('POST', '/unsplash/search', body);
}
export function getServerInfo<T = unknown>(): Promise<T> {
	return apiGet<T>('/app/server-info');
}
