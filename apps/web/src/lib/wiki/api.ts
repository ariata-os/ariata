/**
 * Wiki API Client
 *
 * Fetches wiki pages from the backend API.
 * Backend wiki pages are views of entities/narratives.
 */

// ============================================================================
// API Response Types (match Rust backend types)
// ============================================================================

export interface WikiPersonApi {
	id: string;
	name: string;
	content: string | null;
	article: string | null;
	article_updated_at: string | null;
	picture: string | null;
	cover_image: string | null;
	emails: string[];
	phones: string[];
	birthday: string | null; // ISO date string
	/** When they died, if they have; precision the person gave (year/month/day). */
	died_on: string | null;
	died_precision: string | null;
	/** The authored line about what this person means — one sentence, verbatim. */
	bond: string | null;
	instagram: string | null;
	facebook: string | null;
	linkedin: string | null;
	x: string | null;
	relationship_category: string | null;
	nickname: string | null;
	notes: string | null;
	/** Surfaces this entity also answers to (migration 0037). */
	aliases: string[];
	/** Is the record keeping this article up to date? Off unless asked. */
	article_auto_update?: boolean;
	first_seen: string | null;
	last_seen: string | null;
	seen_count: number | null;
	created_at: string;
	updated_at: string;
}

export interface WikiPlaceApi {
	id: string;
	name: string;
	content: string | null;
	article: string | null;
	article_updated_at: string | null;
	cover_image: string | null;
	category: string | null;
	address: string | null;
	latitude: number | null;
	longitude: number | null;
	seen_count: number | null;
	first_seen: string | null;
	last_seen: string | null;
	created_at: string;
	updated_at: string;
	/** Is the record keeping this article up to date? Off unless asked. */
	article_auto_update?: boolean;
}

export interface WikiOrganizationApi {
	id: string;
	name: string;
	content: string | null;
	article: string | null;
	article_updated_at: string | null;
	cover_image: string | null;
	organization_type: string | null;
	relationship_type: string | null;
	role_title: string | null;
	start_date: string | null;
	end_date: string | null;
	seen_count: number | null;
	first_seen: string | null;
	last_seen: string | null;
	created_at: string;
	updated_at: string;
	/** Is the record keeping this article up to date? Off unless asked. */
	article_auto_update?: boolean;
	aliases?: string[];
}


export interface WikiDayApi {
	id: string;
	date: string; // ISO date string
	start_timezone: string | null;
	/** The day's prose, from wiki_day_prose. The article page is its only home (0106). */
	article?: string | null;
	epigraph: string | null;
	last_edited_by: string | null;
	cover_image: string | null;
	data_quality: {
		coverage: { who: number; whom: number; what: number; when: number; where: number; why: number; how: number };
		overall: number;
		note: string;
	} | null;
	new_entity_count: number;
	new_topic_count: number;
	readiness_score: number | null;
	readiness_details: { hrv: number; rhr: number; sleep_duration: number; deep_rem: number; consistency: number } | null;
	sleep_cycles: Array<{
		start_time: string;
		end_time: string;
		dominant_stage: string;
		avg_hr: number | null;
		autonomic_z: number | null;
	}>;
	created_at: string;
	updated_at: string;
}

export interface IdResolution {
	entity_type: string;
	id: string;
}

// ============================================================================
// List Item Types
// ============================================================================

export interface WikiPersonListItem {
	id: string;
	name: string;
	picture: string | null;
	relationship_category: string | null;
	last_seen: string | null;
	/** Records mentioning this entity. The index's sort key — see wiki.rs. */
	ref_count: number;
}

export interface WikiPlaceListItem {
	id: string;
	name: string;
	category: string | null;
	address: string | null;
	seen_count: number | null;
	/** Records mentioning this entity. The index's sort key — see wiki.rs. */
	ref_count: number;
}

export interface WikiOrganizationListItem {
	id: string;
	name: string;
	organization_type: string | null;
	relationship_type: string | null;
	/** Records mentioning this entity. The index's sort key — see wiki.rs. */
	ref_count: number;
}


// ============================================================================
// API Functions
// ============================================================================

type FetchFn = typeof fetch;

/**
 * Parse an entity ID to extract the type.
 * IDs follow the format: {type}_{hash} (e.g., person_abc123)
 */
export function parseEntityId(id: string): IdResolution | null {
	const parts = id.split('_');
	if (parts.length < 2) return null;
	return {
		entity_type: parts[0],
		id: id
	};
}

// --- Person ---

export async function getPersonById(
	id: string,
	fetchFn: FetchFn = fetch
): Promise<WikiPersonApi | null> {
	const res = await fetchFn(`/api/wiki/person/${encodeURIComponent(id)}`);
	if (!res.ok) return null;
	return res.json();
}

/**
 * File a person as an organization instead.
 *
 * Returns the new org route — the person route stops resolving the moment this
 * succeeds, so callers must navigate or reload rather than keep the old id.
 */
export async function reclassifyPersonAsOrg(
	id: string,
	fetchFn: FetchFn = fetch
): Promise<{ id: string; route: string }> {
	const res = await fetchFn(`/api/entities/people/${id}/reclassify-as-org`, { method: 'POST' });
	if (!res.ok) throw new Error(`Failed to reclassify: ${res.statusText}`);
	return res.json();
}

/**
 * Whether merging two buckets adds their values.
 *
 * `total` grows with the zoom level and stands on zero; `rate` is an average
 * and floats between its own floor and ceiling. The distinction decides both
 * the arithmetic and the drawing.
 */
export type MeasureKind = 'total' | 'rate';

/** One entry in a lane's measure menu. */
export interface MeasureInfo {
	id: string;
	label: string;
	unit: string;
	kind: MeasureKind;
}

/** One lane of the lifeline: a registry domain and its per-bucket series. */
export interface LifelineLane {
	id: string;
	sources: string[];
	density: number[];
	peak: number;
	/** Smallest non-empty bucket — the baseline a `rate` is drawn against. */
	floor: number;
	/** When this lane started collecting. Before it, the lane wasn't watching. */
	first_seen: string | null;
	/** Which measure produced `density`; `records` when none was chosen. */
	measure: string;
	measure_label: string;
	unit: string;
	kind: MeasureKind;
	available: MeasureInfo[];
}

export interface LifelineData {
	from: string;
	to: string;
	buckets: number;
	lanes: LifelineLane[];
}

/** Per-lane series over a window. The server buckets; the client only draws. */
export async function getLifeline(
	buckets: number,
	from?: string,
	to?: string,
	expand?: string[],
	measures?: Record<string, string>,
	fetchFn: FetchFn = fetch
): Promise<LifelineData | null> {
	const p = new URLSearchParams({ buckets: String(buckets) });
	// Omitting the window asks the server for the whole record. A lifeline
	// defaulted to the last year is not a lifeline — this corpus starts in 2017.
	if (from) p.set('from', from);
	if (to) p.set('to', to);
	if (expand?.length) p.set('expand', expand.join(','));
	const pairs = Object.entries(measures ?? {}).map(([lane, id]) => `${lane}:${id}`);
	if (pairs.length) p.set('measures', pairs.join(','));
	const res = await fetchFn(`/api/wiki/lifeline?${p}`);
	if (!res.ok) return null;
	return res.json();
}

/**
 * Time of day against date — the day-clock raster.
 *
 * `cells` is flat and row-major by column: `cells[col * 24 + hour]`. 28,800
 * numbers as objects would be a quarter-megabyte of punctuation.
 */
export interface Clock {
	from: string;
	to: string;
	columns: number;
	cells: number[];
	peak: number;
	/** Busiest cell per column, for normalising a day against its own shape. */
	column_peak: number[];
	timezone: string;
}

/**
 * The day-clock.
 *
 * One timezone for the whole raster, deliberately: rendering each record in the
 * zone it was recorded in would straighten the sleep band back out and destroy
 * the most legible thing on the chart. Fixed to one zone, a fortnight abroad is
 * a visible dislocation.
 */
export async function getClock(
	from: string,
	to: string,
	columns: number,
	tz: string,
	fetchFn: FetchFn = fetch
): Promise<Clock | null> {
	const p = new URLSearchParams({ from, to, buckets: String(columns), tz });
	const res = await fetchFn(`/api/wiki/lifeline/clock?${p}`);
	if (!res.ok) return null;
	return res.json();
}

/** A place a window was spent, found by clustering arrivals. */
export interface Stay {
	lat: number;
	lon: number;
	visits: number;
	minutes: number;
	first: string | null;
	last: string | null;
}

/** Where a window was spent. `bbox` is `[latMin, latMax, lonMin, lonMax]`. */
export interface Ground {
	bbox: [number, number, number, number] | null;
	/** The trace, thinned server-side: `[lat, lon]` pairs in time order. */
	track: [number, number][];
	track_total: number;
	stays: Stay[];
}

/**
 * The ground under a window.
 *
 * A separate request from the lanes because it is answering a different
 * question — where, not when — and because it is only wanted when someone is
 * actually looking at location.
 */
export async function getGround(
	from: string,
	to: string,
	fetchFn: FetchFn = fetch
): Promise<Ground | null> {
	const p = new URLSearchParams({ from, to });
	const res = await fetchFn(`/api/wiki/lifeline/ground?${p}`);
	if (!res.ok) return null;
	return res.json();
}

/** One row inside a window, rendered by its ontology's own declarations. */
export interface LifelineRecord {
	id: string;
	ontology: string;
	lane: string;
	kind: string;
	label: string | null;
	preview: string | null;
	at: string;
}

export interface LifelineFeed {
	records: LifelineRecord[];
	has_more: boolean;
}

/**
 * The records inside a window.
 *
 * The reason to draw a timeline is that a range on it can hand back the rows —
 * a panel of sums is the answer chat already gives badly.
 */
export async function getFeed(
	from: string,
	to: string,
	opts: { lanes?: string[]; limit?: number; offset?: number } = {},
	fetchFn: FetchFn = fetch
): Promise<LifelineFeed | null> {
	const p = new URLSearchParams({ from, to });
	if (opts.lanes?.length) p.set('lanes', opts.lanes.join(','));
	if (opts.limit) p.set('limit', String(opts.limit));
	if (opts.offset) p.set('offset', String(opts.offset));
	const res = await fetchFn(`/api/wiki/lifeline/feed?${p}`);
	if (!res.ok) return null;
	return res.json();
}

/** A day or event Virtues has interpreted. */
export interface Interpreted {
	id: string;
	kind: string;
	/** The segmenter's classification — `sleep`, `transit`, `unknown`. */
	tag: string | null;
	label: string | null;
	summary: string | null;
	start: string;
	end: string | null;
}

export interface ProcessedWindow {
	items: Interpreted[];
	/** The span over which ANY interpretation exists, whatever the window. */
	coverage: [string, string] | null;
	days_processed: number;
}

/**
 * What Virtues has made of a window, as opposed to what was collected in it.
 *
 * Raw reaches back to 2017; the interpreted layer covers weeks. `coverage`
 * comes back regardless of the window so an empty answer can say why.
 */
export async function getProcessed(
	from: string,
	to: string,
	fetchFn: FetchFn = fetch
): Promise<ProcessedWindow | null> {
	const p = new URLSearchParams({ from, to });
	const res = await fetchFn(`/api/wiki/lifeline/processed?${p}`);
	if (!res.ok) return null;
	return res.json();
}

/** Create a person by hand — the record only ever discovered them before. */
export async function createPerson(name: string, fetchFn: FetchFn = fetch): Promise<{ id: string; route: string }> {
	const res = await fetchFn('/api/entities/people', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name })
	});
	if (!res.ok) throw new Error('Could not create that person');
	return res.json();
}

/** Delete an entity, and everything that pointed at it. */
export async function deleteEntity(
	entityType: 'person' | 'place' | 'org',
	id: string,
	fetchFn: FetchFn = fetch
): Promise<void> {
	const path = entityType === 'person' ? 'people' : entityType === 'org' ? 'orgs' : 'places';
	const res = await fetchFn(`/api/entities/${path}/${id}`, { method: 'DELETE' });
	if (!res.ok) {
		const body = await res.json().catch(() => null);
		throw new Error(body?.error ?? body?.message ?? 'Could not delete that');
	}
}

/** A note in the margin of a subject. */
export interface WikiNote {
	id: number;
	subject_type: string;
	subject_id: string;
	kind: string;
	body: string;
	author: string;
	source_refs: unknown;
	created_at: string;
	resolution: string | null;
}

/** Open notes on a subject. */
export async function listNotes(
	subjectType: string,
	subjectId: string,
	fetchFn: FetchFn = fetch
): Promise<WikiNote[]> {
	const res = await fetchFn(`/api/wiki/notes/${subjectType}/${subjectId}`);
	if (!res.ok) return [];
	return res.json();
}

/** Leave a note yourself. Human notes need no citation — you were there. */
export async function createNote(
	subjectType: string,
	subjectId: string,
	body: string,
	kind = 'memo',
	fetchFn: FetchFn = fetch
): Promise<WikiNote> {
	const res = await fetchFn(`/api/wiki/notes/${subjectType}/${subjectId}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ body, kind })
	});
	if (!res.ok) throw new Error('Could not save that note');
	return res.json();
}

/** Close a note. Accepting hands the editing back to you. */
export async function resolveNote(
	id: number,
	resolution: 'accepted' | 'dismissed',
	fetchFn: FetchFn = fetch
): Promise<void> {
	const res = await fetchFn(`/api/wiki/notes/${id}/resolve`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ resolution })
	});
	if (!res.ok) throw new Error('Could not close that note');
}

/** One heart-rate sample for the day's Autonomic chart. */
export interface DayHeartRateSample {
	timestamp: string;
	bpm: number;
}

/**
 * The day's raw HR samples, oldest first. Sparse days are normal.
 *
 * `tz` anchors the local-day window. Without it the server falls back to the
 * day's recorded zone, which is right for a past day and wrong for today when
 * the box and the browser disagree about where midnight is.
 */
export async function getDayHeartRate(
	date: string,
	tz?: string,
	fetchFn: FetchFn = fetch
): Promise<DayHeartRateSample[]> {
	const q = tz ? `?tz=${encodeURIComponent(tz)}` : '';
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}/heart-rate${q}`);
	if (!res.ok) return [];
	return res.json();
}

/** The article join row for a subject. `page_id` is what the editor opens. */
export interface WikiArticleApi {
	id: string;
	subject_type: string;
	subject_id: string;
	page_id: string;
	auto_update: boolean;
	source_ref_count: number;
}

/** A subject's article row, or null when no article exists yet. */
export async function getArticle(
	subjectType: string,
	subjectId: string,
	fetchFn: FetchFn = fetch
): Promise<WikiArticleApi | null> {
	const res = await fetchFn(
		`/api/wiki/articles/${subjectType}/${encodeURIComponent(subjectId)}`
	);
	if (!res.ok) return null;
	return res.json();
}

/** Open notes across the whole record — the Overview's what-changed count. */
export async function countOpenNotes(fetchFn: FetchFn = fetch): Promise<number> {
	const res = await fetchFn('/api/wiki/notes-open-count');
	if (!res.ok) return 0;
	const j = await res.json();
	return typeof j.open === 'number' ? j.open : 0;
}

/** One edit to some article, for the History room. */
export interface HistoryEntry {
	subject_type: string;
	subject_id: string;
	route: string;
	title: string;
	author: string;
	at: string;
	version_number: number;
}

/** One line of a diff. `kind` is 'add' | 'del' | 'ctx'. */
export interface DiffLine {
	kind: string;
	text: string;
}

/** One edit to one article, with what changed. */
export interface ArticleRevision {
	version_number: number;
	author: string;
	at: string;
	diff: DiffLine[];
	is_current: boolean;
}

/** Every recent edit to any article, newest first. */
export async function listHistory(
	limit = 50,
	fetchFn: FetchFn = fetch
): Promise<HistoryEntry[]> {
	const res = await fetchFn(`/api/wiki/history?limit=${limit}`);
	if (!res.ok) return [];
	return res.json();
}

/** One article's edit history, with diffs. */
export async function getArticleHistory(
	subjectType: string,
	subjectId: string,
	fetchFn: FetchFn = fetch
): Promise<ArticleRevision[]> {
	const res = await fetchFn(`/api/wiki/articles/${subjectType}/${subjectId}/history`);
	if (!res.ok) return [];
	return res.json();
}

/** One piece of prose that mentions a subject. */
export interface SubjectBacklink {
	page_id: string;
	title: string;
	/** The subject's route if it is an article, else the page route. */
	route: string;
	is_article: boolean;
}

/** Everything whose prose links to this subject. Derived at read time. */
export async function getSubjectBacklinks(
	subjectType: string,
	subjectId: string,
	fetchFn: FetchFn = fetch
): Promise<SubjectBacklink[]> {
	const res = await fetchFn(`/api/wiki/subjects/${subjectType}/${subjectId}/backlinks`);
	if (!res.ok) return [];
	return res.json();
}

/**
 * Write a subject's first article, now.
 *
 * Synchronous by design — one model call the user is waiting on. Returns the
 * created article; the caller should re-fetch the entity to pick up the prose.
 */
export async function writeArticle(
	subjectType: string,
	subjectId: string,
	fetchFn: FetchFn = fetch
): Promise<{ id: string; page_id: string }> {
	const res = await fetchFn(`/api/wiki/articles/${subjectType}/${subjectId}`, { method: 'POST' });
	if (!res.ok) {
		const body = await res.json().catch(() => null);
		throw new Error(body?.error ?? body?.message ?? `Could not write the article`);
	}
	return res.json();
}

/** Turn maintenance on or off. Off means the AI never touches this article. */
export async function setArticleAutoUpdate(
	subjectType: string,
	subjectId: string,
	autoUpdate: boolean,
	fetchFn: FetchFn = fetch
): Promise<void> {
	const res = await fetchFn(`/api/wiki/articles/${subjectType}/${subjectId}/auto-update`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ auto_update: autoUpdate })
	});
	if (!res.ok) throw new Error('Could not change that');
}

export async function listPeople(fetchFn: FetchFn = fetch): Promise<WikiPersonListItem[]> {
	const res = await fetchFn("/api/wiki/people");
	if (!res.ok) return [];
	return res.json();
}

export async function updatePerson(
	id: string,
	data: Partial<WikiPersonApi>,
	fetchFn: FetchFn = fetch
): Promise<WikiPersonApi | null> {
	const res = await fetchFn(`/api/wiki/person/${id}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

// --- Place ---

export async function getPlaceById(
	id: string,
	fetchFn: FetchFn = fetch
): Promise<WikiPlaceApi | null> {
	const res = await fetchFn(`/api/wiki/place/${encodeURIComponent(id)}`);
	if (!res.ok) return null;
	return res.json();
}

export async function listPlaces(fetchFn: FetchFn = fetch): Promise<WikiPlaceListItem[]> {
	const res = await fetchFn("/api/wiki/places");
	if (!res.ok) return [];
	return res.json();
}

export async function updatePlace(
	id: string,
	data: Partial<WikiPlaceApi>,
	fetchFn: FetchFn = fetch
): Promise<WikiPlaceApi | null> {
	const res = await fetchFn(`/api/wiki/place/${id}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

// --- Organization ---

export async function getOrganizationById(
	id: string,
	fetchFn: FetchFn = fetch
): Promise<WikiOrganizationApi | null> {
	const res = await fetchFn(`/api/wiki/organization/${encodeURIComponent(id)}`);
	if (!res.ok) return null;
	return res.json();
}

export async function listOrganizations(
	fetchFn: FetchFn = fetch
): Promise<WikiOrganizationListItem[]> {
	const res = await fetchFn("/api/wiki/organizations");
	if (!res.ok) return [];
	return res.json();
}

export async function updateOrganization(
	id: string,
	data: Partial<WikiOrganizationApi>,
	fetchFn: FetchFn = fetch
): Promise<WikiOrganizationApi | null> {
	const res = await fetchFn(`/api/wiki/organization/${id}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

// --- Telos ---



// --- Act ---



// --- Chapter ---



// --- Day ---

export async function getDayByDate(
	date: string,
	fetchFn: FetchFn = fetch
): Promise<WikiDayApi | null> {
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}`);
	if (!res.ok) return null;
	return res.json();
}

export async function updateDay(
	date: string,
	data: Partial<WikiDayApi>,
	fetchFn: FetchFn = fetch
): Promise<WikiDayApi | null> {
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

export async function listDays(
	startDate?: string,
	endDate?: string,
	fetchFn: FetchFn = fetch
): Promise<WikiDayApi[]> {
	const params = new URLSearchParams();
	if (startDate) params.set("start_date", startDate);
	if (endDate) params.set("end_date", endDate);
	const query = params.toString() ? `?${params}` : "";
	const res = await fetchFn(`/api/wiki/days${query}`);
	if (!res.ok) return [];
	return res.json();
}

/** One heatmap cell: how much recorded life a day holds. */
export interface DayActivityApi {
	date: string;
	event_count: number;
	narrated: boolean;
}

export async function listDayActivity(
	startDate: string,
	endDate: string,
	fetchFn: FetchFn = fetch
): Promise<DayActivityApi[]> {
	const res = await fetchFn(
		`/api/wiki/activity?start_date=${startDate}&end_date=${endDate}`
	);
	if (!res.ok) return [];
	return res.json();
}

/** One raw record linked to an entity via refs — the entity page's evidence feed. */
export interface EntityRecordApi {
	source_type: string;
	id: string;
	timestamp: string;
	label: string;
	preview: string | null;
	role: string | null;
	continuous: boolean;
}

export interface EntityRecordsPageApi {
	items: EntityRecordApi[];
	total: number;
}

/** Per-raw-source_type counts across ALL of an entity's records. */
export interface EntityRecordFacetApi {
	source_type: string;
	count: number;
	continuous: boolean;
}

export async function getEntityRecordsPage(
	entityId: string,
	opts: {
		offset: number;
		limit: number;
		search?: string;
		/** Raw source_types to include; empty = all. */
		types?: string[];
		dir?: "asc" | "desc";
	},
	fetchFn: FetchFn = fetch
): Promise<EntityRecordsPageApi> {
	const params = new URLSearchParams();
	params.set("offset", String(opts.offset));
	params.set("limit", String(opts.limit));
	if (opts.search) params.set("search", opts.search);
	if (opts.types?.length) params.set("types", opts.types.join(","));
	if (opts.dir) params.set("dir", opts.dir);
	// no-store: this endpoint predates some deployed boxes, whose SPA fallback
	// used to answer unknown /api paths with cacheable HTML — never let a
	// poisoned cache entry shadow the real data.
	const res = await fetchFn(
		`/api/wiki/entity/${encodeURIComponent(entityId)}/records?${params}`,
		{ cache: "no-store" }
	);
	if (!res.ok) {
		// A server error is not an empty history — let the caller show it.
		throw new Error(`Failed to load entity records (${res.status})`);
	}
	return res.json();
}

export async function getEntityRecordFacets(
	entityId: string,
	fetchFn: FetchFn = fetch
): Promise<EntityRecordFacetApi[]> {
	const res = await fetchFn(
		`/api/wiki/entity/${encodeURIComponent(entityId)}/records/facets`,
		{ cache: "no-store" }
	);
	if (!res.ok) return [];
	return res.json();
}

/** A past year's entry sharing today's month and day. */
export interface OnThisDayApi {
	date: string;
	epigraph: string | null;
	narrated: boolean;
	event_count: number;
}

export async function listOnThisDay(
	date?: string,
	fetchFn: FetchFn = fetch
): Promise<OnThisDayApi[]> {
	const query = date ? `?date=${encodeURIComponent(date)}` : "";
	const res = await fetchFn(`/api/wiki/on-this-day${query}`);
	if (!res.ok) return [];
	return res.json();
}

// --- Narrative identity ---

/** The "In your own words" document, read from its page. Read-only here —
 *  editing happens on the page itself (`page_id`); the writable abridged copy
 *  this used to front was retired 2026-09-01. */
export interface NarrativeIdentityApi {
	content: string;
	updated_at: string;
	/** The document's page; empty until the interview has been written up. */
	page_id: string;
}

export async function getNarrativeIdentity(
	fetchFn: FetchFn = fetch
): Promise<NarrativeIdentityApi | null> {
	const res = await fetchFn(`/api/wiki/narrative-identity`);
	if (!res.ok) return null;
	return res.json();
}

/** One chapter of the life — authored in the interview, never inferred. */
export interface ChapterApi {
	id: string;
	kind: "chapter" | "unknown";
	title: string | null;
	started_at: string;
	ended_at: string | null;
	is_current: boolean;
	changepoint: string | null;
	summary: string | null;
}

export async function getChapters(fetchFn: FetchFn = fetch): Promise<ChapterApi[]> {
	const res = await fetchFn(`/api/wiki/chapters`);
	if (!res.ok) return [];
	const body = await res.json();
	return body.chapters ?? [];
}

// ============================================================================
// Citation Types
// ============================================================================

export interface CitationApi {
	id: string;
	source_type: string;
	source_id: string;
	target_table: string;
	target_id: string;
	citation_index: number;
	label: string | null;
	preview: string | null;
	is_hidden: boolean | null;
	added_by: string | null;
	created_at: string;
	updated_at: string;
}

export interface CreateCitationRequest {
	source_type?: string; // Set from path in handler
	source_id?: string; // Set from path in handler
	target_table: string;
	target_id: string;
	citation_index: number;
	label?: string;
	preview?: string;
	is_hidden?: boolean;
	added_by?: string;
}

export interface UpdateCitationRequest {
	label?: string;
	preview?: string;
	is_hidden?: boolean;
	citation_index?: number;
}

// ============================================================================
// Temporal Event Types
// ============================================================================

export interface TemporalEventApi {
	id: string;
	day_id: string;
	start_time: string;
	end_time: string;
	auto_label: string | null;
	auto_location: string | null;
	user_label: string | null;
	user_location: string | null;
	user_notes: string | null;
	source_ontologies: string[] | null;
	is_unknown: boolean | null;
	is_transit: boolean | null;
	is_user_added: boolean | null;
	is_user_edited: boolean | null;
	// Dayline fields
	novelty_z: number | null;
	topics: string[] | null;
	event_summary: string | null;
	agent_action: string | null;
	is_sleep: boolean | null;
	user_hidden: boolean | null;
	// Autonomic scoring
	avg_hr: number | null;
	autonomic_z: number | null;
	hr_z: number | null;
	// Entity/topic novelty
	entities: string[] | null;
	topic_novelty: Record<string, number> | null;
	entity_novelty: Record<string, number> | null;
	entity_timestamps: Record<string, string> | null;
	created_at: string;
	updated_at: string;
}

export interface CreateTemporalEventRequest {
	day_id: string;
	start_time: string;
	end_time: string;
	auto_label?: string;
	auto_location?: string;
	user_label?: string;
	user_location?: string;
	user_notes?: string;
	source_ontologies?: unknown;
	is_unknown?: boolean;
	is_transit?: boolean;
	is_user_added?: boolean;
	is_user_edited?: boolean;
}

export interface UpdateTemporalEventRequest {
	start_time?: string;
	end_time?: string;
	user_label?: string;
	user_location?: string;
	user_notes?: string;
	is_user_edited?: boolean;
}

// ============================================================================
// Citation API Functions
// ============================================================================

/**
 * Get citations for a wiki page.
 * @param sourceType - The type of wiki page (person, place, organization, telos, act, chapter, day)
 * @param sourceId - The UUID of the wiki page
 */
export async function getCitations(
	sourceType: string,
	sourceId: string,
	fetchFn: FetchFn = fetch
): Promise<CitationApi[]> {
	const res = await fetchFn(`/api/wiki/${sourceType}/${sourceId}/citations`);
	if (!res.ok) return [];
	return res.json();
}

/**
 * Create a citation for a wiki page.
 */
export async function createCitation(
	sourceType: string,
	sourceId: string,
	data: CreateCitationRequest,
	fetchFn: FetchFn = fetch
): Promise<CitationApi | null> {
	const res = await fetchFn(`/api/wiki/${sourceType}/${sourceId}/citations`, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

/**
 * Update a citation.
 */
export async function updateCitation(
	citationId: string,
	data: UpdateCitationRequest,
	fetchFn: FetchFn = fetch
): Promise<CitationApi | null> {
	const res = await fetchFn(`/api/wiki/citations/${citationId}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

/**
 * Delete a citation.
 */
export async function deleteCitation(
	citationId: string,
	fetchFn: FetchFn = fetch
): Promise<boolean> {
	const res = await fetchFn(`/api/wiki/citations/${citationId}`, {
		method: "DELETE",
	});
	return res.ok;
}

// ============================================================================
// Temporal Event API Functions
// ============================================================================

/**
 * Get events for a specific day by date.
 * @param date - The date in YYYY-MM-DD format
 */
export async function getDayEvents(
	date: string,
	fetchFn: FetchFn = fetch
): Promise<TemporalEventApi[]> {
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}/events`);
	if (!res.ok) return [];
	return res.json();
}

/**
 * Create a temporal event.
 */
export async function createTemporalEvent(
	data: CreateTemporalEventRequest,
	fetchFn: FetchFn = fetch
): Promise<TemporalEventApi | null> {
	const res = await fetchFn("/api/wiki/events", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

/**
 * Update a temporal event.
 */
export async function updateTemporalEvent(
	eventId: string,
	data: UpdateTemporalEventRequest,
	fetchFn: FetchFn = fetch
): Promise<TemporalEventApi | null> {
	const res = await fetchFn(`/api/wiki/events/${eventId}`, {
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(data),
	});
	if (!res.ok) return null;
	return res.json();
}

/**
 * Delete a temporal event.
 */
export async function deleteTemporalEvent(
	eventId: string,
	fetchFn: FetchFn = fetch
): Promise<boolean> {
	const res = await fetchFn(`/api/wiki/events/${eventId}`, {
		method: "DELETE",
	});
	return res.ok;
}

// ============================================================================
// Day Sources Types (Ontology records for a day)
// ============================================================================

export interface DaySourceApi {
	source_type: string;
	id: string;
	timestamp: string;
	label: string;
	preview: string | null;
	/** High-frequency measurement streams (heart rate, steps, HRV). Hidden by
	 *  default on the day page behind a filter, since a day holds thousands. */
	continuous: boolean;
}

/**
 * Get all ontology data sources for a specific date.
 * Returns calendar events, emails, location visits, workouts, etc.
 * @param date - The date in YYYY-MM-DD format
 */
export async function getDaySources(
	date: string,
	fetchFn: FetchFn = fetch
): Promise<DaySourceApi[]> {
	// Pass the viewing device's IANA zone so an in-progress "today" is anchored to
	// where the owner currently is (see agents/record/timezone-model.md). Harmless for past
	// days — the server prefers the day's locked start_timezone.
	const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const qs = tz ? `?tz=${encodeURIComponent(tz)}` : "";
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}/sources${qs}`);
	if (!res.ok) return [];
	return res.json();
}

// ============================================================================
// Timeline Day View (chunked location/transit/missing-data stream)
// ============================================================================

export interface TimelineDayLocationChunk {
	type: "location";
	start_time: string;
	end_time: string;
	place_name: string | null;
	latitude: number;
	longitude: number;
	place_id: string | null;
	duration_minutes: number | null;
	place_category: string | null;
}

export type TimelineDayChunk =
	| TimelineDayLocationChunk
	| { type: "transit" }
	| { type: "missing_data" };

export interface TimelineDayPoint {
	latitude: number;
	longitude: number;
	timestamp: string;
}

export interface TimelineDayView {
	date: string;
	chunks: TimelineDayChunk[];
	points: TimelineDayPoint[];
}

// ============================================================================
// Day Chats (in-app Virtues + external AI conversations)
// ============================================================================

export interface DayChatApi {
	id: string;
	source: "virtues" | "external";
	provider: string | null;
	title: string;
	message_count: number;
	started_at: string;
}

/**
 * Get all AI chats (in-app Virtues + external imported) that started on a day.
 * In-app chats are navigable; external chats are display-only.
 * @param date - The date in YYYY-MM-DD format
 */
export async function getDayChats(
	date: string,
	fetchFn: FetchFn = fetch,
): Promise<DayChatApi[]> {
	const res = await fetchFn(`/api/wiki/day/${encodeURIComponent(date)}/chats`);
	if (!res.ok) return [];
	return res.json();
}

/**
 * Get the chunked timeline view for a day (location stops, transit, gaps).
 * @param date - The date in YYYY-MM-DD format
 */
export async function getDayTimeline(
	date: string,
	fetchFn: FetchFn = fetch,
): Promise<TimelineDayView | null> {
	const res = await fetchFn(`/api/timeline/day/${encodeURIComponent(date)}`);
	if (!res.ok) return null;
	return res.json();
}

// ============================================================================
// Today Streams — the three raw record streams, as spans, before synthesis
// ============================================================================

export interface TodayLocationSpan {
	id: string;
	start_time: string;
	end_time: string;
	place_name: string | null;
	place_category: string | null;
	duration_minutes: number | null;
}

export interface TodayCalendarSpan {
	id: string;
	start_time: string;
	end_time: string;
	title: string;
	is_all_day: boolean;
	is_sacred: boolean;
	location_name: string | null;
	calendar_name: string | null;
}

export interface TodayAudioSpan {
	id: string;
	start_time: string;
	end_time: string;
	/** the box flagged this ~5-min chunk as silence */
	is_silent: boolean;
}

export interface TodayStreamsView {
	date: string;
	timezone: string;
	location: TodayLocationSpan[];
	calendar: TodayCalendarSpan[];
	audio: TodayAudioSpan[];
}

/**
 * Get the three raw record streams (location, calendar, audio) for a day, as
 * spans — the "day before synthesis" homepage view. Passes the viewing device's
 * IANA zone so an in-progress "today" is anchored to where the owner is.
 * @param date - The date in YYYY-MM-DD format
 */
export async function getTodayStreams(
	date: string,
	fetchFn: FetchFn = fetch,
): Promise<TodayStreamsView | null> {
	const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const qs = tz ? `?tz=${encodeURIComponent(tz)}` : "";
	const res = await fetchFn(`/api/today/${encodeURIComponent(date)}/streams${qs}`);
	if (!res.ok) return null;
	return res.json();
}

// ============================================================================
// Home-page loops — weather, upcoming calendar, unnamed-place backlog
// ============================================================================

export interface WeatherNow {
	temperature_c: number | null;
	apparent_c: number | null;
	humidity_pct: number | null;
	wind_kph: number | null;
	is_day: boolean | null;
	weather_code: number | null;
	condition: string;
	temp_max_c: number | null;
	temp_min_c: number | null;
	sunrise: string | null;
	sunset: string | null;
	valid_time: string;
}

/** Current weather for the masthead. Null until the weather_sync cron runs. */
export async function getWeatherNow(fetchFn: FetchFn = fetch): Promise<WeatherNow | null> {
	const res = await fetchFn("/api/weather/current");
	if (!res.ok) return null;
	return res.json();
}

export interface UpcomingEvent {
	id: string;
	title: string;
	start_time: string;
	end_time: string;
	is_all_day: boolean;
	location_name: string | null;
	is_sacred: boolean;
}

/** The next few calendar events (holidays/birthdays filtered out). */
export async function getCalendarUpcoming(
	limit = 5,
	fetchFn: FetchFn = fetch,
): Promise<UpcomingEvent[]> {
	const res = await fetchFn(`/api/calendar/upcoming?limit=${limit}`);
	if (!res.ok) return [];
	return res.json();
}

export interface UnnamedPlace {
	id: string;
	name: string;
	ref_count: number;
	latitude: number | null;
	longitude: number | null;
}

/** Places visited but never named — the home "name this place" ask. */
export async function getUnnamedPlaces(
	limit = 3,
	fetchFn: FetchFn = fetch,
): Promise<UnnamedPlace[]> {
	const res = await fetchFn(`/api/places/unnamed?limit=${limit}`);
	if (!res.ok) return [];
	return res.json();
}

