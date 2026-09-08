<!--
	WikiView.svelte

	The wiki room: the wikipedia of one life. Sections are route-driven and
	deep-linkable, navigated from the SIDEBAR — the room swaps the rail for its
	own rows the way Settings and Developer do.

	There used to be a SubNav strip across the top carrying the same eight
	links. Once the sidebar grew them it was the same list twice on one screen,
	and the top copy cost a band of vertical space on every wiki page to say
	what the rail already said.

	  /wiki           Overview — the front page: standfirst, activity, on
	                  this day, the latest entry, and the index.
	  /wiki/days      Days — the temporal spine, a year calendar + chronicle.
	  /wiki/entities  Entities — one index; person/place/org are filters.
	  /wiki/identity  Narrative identity — user-authored, essay register.

	Legacy routes (/wiki/people, /wiki/places, /wiki/orgs, /wiki/unlinked,
	/entities) fold into the entities section, presetting its type filter.

	The overview is set as an essay with a marginalia rail: the main column
	carries the text and the charts; the margin carries the numbers and
	asides. Charts and inline components stay crisp; the prose stays serif.
-->

<script lang="ts">
	import type { Tab } from '$lib/tabs/types';
	import { WIKI_SECTION_RE } from '$lib/tabs/registry';
	import { windowShellStore } from '$lib/stores/window-shell.svelte';
	import {
		ActivityHeatmap,
		ChaptersSection,
		DaysChronicle,
		NarrativeIdentitySection,
	} from '$lib/components/wiki';
	import UniversalDataGrid, {
		type Column,
	} from '$lib/components/datagrid/UniversalDataGrid.svelte';
	import type { FilterDef } from '$lib/components/datagrid/types';
	import { onMount } from 'svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { getLocalDateSlug, formatLongDate } from '$lib/utils/dateUtils';
	import {
		listPeople,
		listPlaces,
		listOrganizations,
		listDays,
		listDayActivity,
		listOnThisDay,
		getNarrativeIdentity,
		getLifeline,
		listHistory,
		countOpenNotes,
		type WikiPersonListItem,
		type WikiPlaceListItem,
		type WikiOrganizationListItem,
		type OnThisDayApi,
		type HistoryEntry,
	} from '$lib/wiki/api';
	import { toActivityLevels } from '$lib/wiki/activity';
	import { getProfile } from '$lib/api/client';
	import WikiHistory from '$lib/components/wiki/WikiHistory.svelte';
	import LifelineCanvas from '$lib/components/wiki/LifelineCanvas.svelte';
	import NotesRail from '$lib/components/wiki/NotesRail.svelte';
	import { contextMenu } from '$lib/stores/contextMenu.svelte';
	import { getKeepMenuItems } from '$lib/utils/contextMenuItems';
	import { reclassifyPersonAsOrg, createPerson, deleteEntity } from '$lib/wiki/api';

	let { tab, active }: { tab: Tab; active: boolean } = $props();

	type Section = 'overview' | 'stories' | 'days' | 'years' | 'entities' | 'identity' | 'chapters' | 'history' | 'lifeline';


	// The active section comes from the route; the sidebar rail does the linking.
	// Legacy per-type routes land in the unified entities section.
	const LEGACY_TYPE: Record<string, 'person' | 'place' | 'org'> = {
		people: 'person',
		places: 'place',
		orgs: 'org',
	};

	const routeSegment = $derived(
		tab.route.match(WIKI_SECTION_RE)?.[1] ?? (tab.route === '/entities' ? 'entities' : null)
	);

	/**
	 * The only segments that are NOT their own section.
	 *
	 * This was written the other way round — an allowlist of sections that
	 * render themselves, with everything else folding into Entities — which
	 * meant every new section silently became Entities until someone remembered
	 * to add it. Lifeline and History both did exactly that: the route matched,
	 * the tab opened, and the entity index appeared.
	 *
	 * Inverted, the default is "a section is itself" and only these four
	 * historical aliases are special. Adding a section now requires no edit
	 * here at all.
	 */
	const FOLDS_INTO_ENTITIES = ['people', 'places', 'orgs', 'unlinked'] as const;

	const section = $derived<Section>(
		routeSegment == null
			? 'overview'
			: (FOLDS_INTO_ENTITIES as readonly string[]).includes(routeSegment)
				? 'entities'
				: (routeSegment as Section)
	);

	// --- Years ---
	//
	// Derived, not stored: there is no years endpoint, so the index is grouped
	// from day activity. Only years with recorded days appear — an empty year
	// is not a year of your life you'd want listed.

	interface YearRow {
		year: number;
		recorded: number;
		narrated: number;
	}

	let years = $state<YearRow[]>([]);
	let yearsLoaded = $state(false);

	async function loadYears() {
		if (yearsLoaded) return;
		// Wide enough to cover the record; the endpoint returns only real days.
		const end = new Date();
		const start = new Date(end.getFullYear() - 10, 0, 1);
		const activity = await listDayActivity(
			getLocalDateSlug(start),
			getLocalDateSlug(end)
		);

		const byYear = new Map<number, YearRow>();
		for (const d of activity) {
			if (!d.event_count) continue;
			const y = Number(d.date.slice(0, 4));
			const row = byYear.get(y) ?? { year: y, recorded: 0, narrated: 0 };
			row.recorded += 1;
			if (d.narrated) row.narrated += 1;
			byYear.set(y, row);
		}
		years = [...byYear.values()].sort((a, b) => b.year - a.year);
		yearsLoaded = true;
	}

	$effect(() => {
		if (section === 'years') void loadYears();
	});

	/** Type filter preset when arriving via a legacy per-type route. */
	const entityTypePreset = $derived(
		routeSegment ? (LEGACY_TYPE[routeSegment] ?? null) : null
	);

	/** What the rail highlights: legacy per-type routes read as Entities. */

	// --- Unified entity list (also feeds the overview index) ---

	interface UnifiedEntity {
		id: string;
		name: string;
		entityType: 'person' | 'place' | 'org';
		subtitle: string | null;
		route: string;
		refCount: number;
		isSelf: boolean;
	}

	const typeConfig = {
		person: { icon: 'ri:user-line', label: 'Person', plural: 'People', legacy: 'people' },
		place: { icon: 'ri:map-pin-line', label: 'Place', plural: 'Places', legacy: 'places' },
		org: { icon: 'ri:building-line', label: 'Organization', plural: 'Organizations', legacy: 'orgs' },
	} as const;

	let allEntities = $state<UnifiedEntity[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let counts = $derived({
		person: allEntities.filter((e) => e.entityType === 'person').length,
		place: allEntities.filter((e) => e.entityType === 'place').length,
		org: allEntities.filter((e) => e.entityType === 'org').length,
	});

	async function loadAllEntities() {
		loading = true;
		error = null;
		try {
			// The owner is in the graph like anyone else — a real box carries a
			// person row for them, built from contacts and email. The profile
			// says which one (0080); without it the index lists you as a
			// stranger among your own contacts.
			const [people, places, orgs, profile] = await Promise.all([
				listPeople(),
				listPlaces(),
				listOrganizations(),
				getProfile().catch(() => null),
			]);
			const selfId = profile?.self_person_id ?? null;

			const unified: UnifiedEntity[] = [
				...people.map((p: WikiPersonListItem): UnifiedEntity => ({
					id: p.id,
					name: p.name,
					entityType: 'person',
					subtitle: p.relationship_category,
					route: `/person/${p.id}`,
					refCount: p.ref_count ?? 0,
					isSelf: p.id === selfId,
				})),
				...places.map((p: WikiPlaceListItem): UnifiedEntity => ({
					id: p.id,
					name: p.name,
					entityType: 'place',
					subtitle: p.category || p.address,
					route: `/place/${p.id}`,
					refCount: p.ref_count ?? 0,
					isSelf: false,
				})),
				...orgs.map((o: WikiOrganizationListItem): UnifiedEntity => ({
					id: o.id,
					name: o.name,
					entityType: 'org',
					subtitle: o.organization_type || o.relationship_type,
					route: `/org/${o.id}`,
					refCount: o.ref_count ?? 0,
					isSelf: false,
				})),
			];

			// By mentions, not by name. Each of the three endpoints already returns
			// its own list in this order; the merge has to re-apply it or the
			// interleave silently restores the alphabet — which is what this
			// index did before, and why 573 contacts arrived with no order at
			// all. Name is the tie-break, so the long tail of 0-mention rows is
			// still browsable.
			unified.sort((a, b) => b.refCount - a.refCount || a.name.localeCompare(b.name));
			allEntities = unified;
		} catch (e) {
			console.error('Failed to load entities:', e);
			error = e instanceof Error ? e.message : 'Failed to load entities';
		} finally {
			loading = false;
		}
	}

	const entityColumns: Column<UnifiedEntity>[] = [
		{
			key: 'name',
			label: 'Name',
			icon: 'ri:group-line',
			width: '45%',
			minWidth: '200px',
		},
		{
			key: 'entityType',
			label: 'Type',
			icon: 'ri:price-tag-3-line',
			width: '20%',
			minWidth: '120px',
			getValue: (item) =>
				item.isSelf ? `${typeConfig[item.entityType].label} · You` : typeConfig[item.entityType].label,
		},
		{
			key: 'subtitle',
			label: 'Details',
			icon: 'ri:information-line',
			width: '25%',
			minWidth: '140px',
			hideOnMobile: true,
		},
		{
			key: 'refCount',
			label: 'Mentions',
			icon: 'ri:link',
			width: '10%',
			minWidth: '90px',
			hideOnMobile: true,
			getValue: (item) => (item.refCount ? item.refCount.toLocaleString() : '—'),
		},
	];

	/**
	 * Row menu: the grid's default items, plus the one correction this index
	 * exists to make possible.
	 *
	 * A person row is not always a person. `extract_name_from_email()` mints one
	 * for any unseen sender, so the index arrives holding Gusto, Slack and The
	 * Plaid Team alongside real contacts. Ranking by mentions sinks them, but
	 * sinking is not fixing — this is the fix, and it belongs on the row rather
	 * than behind a detail page nobody opens for a company.
	 *
	 * Only offered for people: place and org rows have nowhere to go.
	 */
	function entityContextMenu(entity: UnifiedEntity, e: MouseEvent) {
		e.preventDefault();
		const items = [
			{
				id: 'open-beside',
				label: 'Open beside',
				icon: 'ri:layout-column-line',
				action: () => {
					windowShellStore.openRouteBeside(entity.route, entity.name);
				},
			},
			...getKeepMenuItems({
				url: entity.route,
				label: entity.name,
				icon: typeConfig[entity.entityType].icon,
			}),
		];

		// The owner is a person by definition — the server refuses this anyway
		// (migration 0080), but offering it and then failing is worse than not
		// offering it.
		if (entity.entityType === 'person' && !entity.isSelf) {
			items.push({
				id: 'reclassify-org',
				label: 'This is an organization',
				icon: 'ri:building-line',
				dividerBefore: true,
				action: () => void reclassifyEntity(entity),
			});
		}
		// Deleting takes the refs, article, notes and any pins with it — see
		// purge_subject. Not offered for the owner.
		if (!entity.isSelf) {
			items.push({
				id: 'delete-entity',
				label: 'Delete',
				icon: 'ri:delete-bin-line',
				dividerBefore: entity.entityType !== 'person',
				action: () => void removeEntity(entity),
			});
		}
		contextMenu.show({ x: e.clientX, y: e.clientY }, items);
	}

	async function removeEntity(entity: UnifiedEntity) {
		// Irreversible and it takes the edges with it, so it asks first.
		if (!confirm(`Delete ${entity.name}? This also removes its records links, notes and pins.`)) return;
		try {
			await deleteEntity(entity.entityType, entity.id);
			await loadAllEntities();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Could not delete';
		}
	}

	async function addPerson() {
		const name = prompt('Name of the person to add:')?.trim();
		if (!name) return;
		try {
			const made = await createPerson(name);
			await loadAllEntities();
			windowShellStore.openTabFromRoute(made.route, { label: name, focusExisting: true });
		} catch (err) {
			error = err instanceof Error ? err.message : 'Could not create that person';
		}
	}

	async function reclassifyEntity(entity: UnifiedEntity) {
		try {
			await reclassifyPersonAsOrg(entity.id);
			// Reload rather than patch in place: the row changes id, type and
			// route all at once, so a local edit would leave a row pointing at
			// a person route that no longer resolves.
			await loadAllEntities();
		} catch (err) {
			console.error('[WikiView] reclassify failed:', err);
			error = err instanceof Error ? err.message : 'Could not reclassify';
		}
	}

	// One filter instead of three tabs: person/place/org narrow the same index.
	const entityFilters = $derived<FilterDef<UnifiedEntity>[]>([
		{
			id: 'type',
			label: 'Type',
			kind: 'multi',
			field: 'entityType',
			defaultValue: entityTypePreset ? [entityTypePreset] : null,
			options: [
				{ value: 'person', label: 'People', icon: typeConfig.person.icon },
				{ value: 'place', label: 'Places', icon: typeConfig.place.icon },
				{ value: 'org', label: 'Organizations', icon: typeConfig.org.icon },
			],
		},
	]);

	// --- Overview data ---

	let activityData = $state<Map<string, number>>(new Map());
	let loadingActivity = $state(true);
	let activityStats = $state<{ recorded: number; narrated: number; stubs: number }>({
		recorded: 0,
		narrated: 0,
		stubs: 0,
	});
	let onThisDay = $state<OnThisDayApi[]>([]);
	let latestEntry = $state<{ slug: string; label: string; epigraph: string | null } | null>(null);
	let standfirst = $state<string | null>(null);

	// The lifeline strip: the whole record flattened to one row (§17.1), plus
	// what the same response tells us for free — when the record starts, and
	// which lanes have gone quiet ("where it's thin").
	let stripDensity = $state<number[]>([]);
	let recordSince = $state<string | null>(null);
	let thinLanes = $state<string[]>([]);

	// What changed: the review loop's front door — recent article edits and
	// the open-note count. Without this the machine writes into a room nobody
	// visits.
	let recentEdits = $state<HistoryEntry[]>([]);
	let openNoteCount = $state(0);

	// One source of truth for the window: fetch exactly what the heatmap draws.
	const HEATMAP_WEEKS = 26;

	onMount(async () => {
		loadAllEntities();

		try {
			const endDate = new Date();
			const startDate = new Date();
			startDate.setDate(startDate.getDate() - HEATMAP_WEEKS * 7);
			const recentStart = new Date();
			recentStart.setDate(recentStart.getDate() - 45);

			const [activity, otd, recent, identity, lifeline, edits, openNotes] =
				await Promise.all([
					listDayActivity(getLocalDateSlug(startDate), getLocalDateSlug(endDate)),
					listOnThisDay(),
					listDays(getLocalDateSlug(recentStart), getLocalDateSlug(endDate)),
					getNarrativeIdentity(),
					// No window: the whole record, which is the point of the strip.
					getLifeline(560),
					listHistory(6),
					countOpenNotes(),
				]);

			if (lifeline && lifeline.lanes.length) {
				const n = lifeline.lanes[0]?.density.length ?? 0;
				const sum = new Array(n).fill(0);
				for (const l of lifeline.lanes) {
					const p = l.peak || 1;
					for (let i = 0; i < n; i++) sum[i] += l.density[i] / p;
				}
				stripDensity = sum;
				recordSince = new Date(lifeline.from).toLocaleDateString('en-US', {
					month: 'long',
					year: 'numeric',
				});

				// A lane that was collecting and has gone quiet for over a week.
				const fromMs = new Date(lifeline.from).getTime();
				const toMs = new Date(lifeline.to).getTime();
				const quiet: string[] = [];
				for (const l of lifeline.lanes) {
					if (!l.first_seen) continue; // never collected — the console's story
					let last = -1;
					for (let i = l.density.length - 1; i >= 0; i--) {
						if (l.density[i] > 0) {
							last = i;
							break;
						}
					}
					if (last < 0) continue;
					const lastMs = fromMs + ((last + 1) / l.density.length) * (toMs - fromMs);
					if (Date.now() - lastMs > 7 * 86_400_000) {
						const label = l.id.charAt(0).toUpperCase() + l.id.slice(1);
						const since = new Date(lastMs).toLocaleDateString('en-US', {
							month: 'short',
							day: 'numeric',
						});
						quiet.push(`No ${label.toLowerCase()} data since ${since}`);
					}
				}
				thinLanes = quiet.slice(0, 3);
			}
			recentEdits = edits;
			openNoteCount = openNotes;

			activityData = toActivityLevels(activity);
			activityStats = {
				recorded: activity.filter((d) => d.event_count > 0).length,
				narrated: activity.filter((d) => d.narrated).length,
				stubs: activity.filter((d) => d.event_count > 0 && !d.narrated).length,
			};

			onThisDay = otd;

			// recent is date DESC; the latest narrated day is the featured entry.
			const featured = recent.find((d) => d.article);
			if (featured) {
				latestEntry = {
					slug: featured.date,
					label: new Date(featured.date + 'T12:00:00').toLocaleDateString('en-US', {
						weekday: 'long',
						month: 'long',
						day: 'numeric',
						year: 'numeric',
					}),
					epigraph: featured.epigraph,
				};
			}

			// The identity's first PROSE line is the front page's standfirst —
			// headings are skipped, since the document now arrives whole
			// ("## Where you have been" would otherwise be the standfirst).
			const firstLine = identity?.content
				?.split('\n')
				.filter((l) => !l.trim().startsWith('#'))
				.map((l) => l.replace(/^[>*\-\s]+/, '').trim())
				.find((l) => l.length > 0);
			if (firstLine) {
				standfirst =
					firstLine.length > 180 ? firstLine.slice(0, 177) + '…' : firstLine;
			}
		} catch (e) {
			console.error('Failed to load overview data:', e);
		} finally {
			loadingActivity = false;
		}
	});

	// Handle day click from heatmap / chronicle / links
	function openDay(slug: string) {
		windowShellStore.openTabFromRoute(`/day/day_${slug}`);
	}

	function openEntity(entity: UnifiedEntity) {
		windowShellStore.openTabFromRoute(entity.route);
	}

	// Overview cards switch this pane to the matching section.
	function goTo(path: string) {
		windowShellStore.updateTab(tab.id, { route: path });
	}

	// Today's formatted date
	const today = new Date();
	const todaySlug = getLocalDateSlug(today);
	const todayFormatted = formatLongDate(today);

	function yearOf(slug: string): string {
		return slug.slice(0, 4);
	}
</script>

<div class="wiki-view">
	<main class="content">
		{#if section === 'overview'}
			<div class="ovw">
				<header class="mast">
					<h1>Wiki</h1>
					<p class="standfirst">
						{standfirst ??
							'A record of your life — its days, its people and places, and the story they add up to.'}
					</p>
					<!-- Computed, never written (§17.2): every number here is SQL,
					     so the line is always current and never goes stale. -->
					<p class="record-line">
						{counts.person}
						{counts.person === 1 ? 'person' : 'people'} · {counts.place}
						{counts.place === 1 ? 'place' : 'places'} · {counts.org}
						{counts.org === 1 ? 'organization' : 'organizations'}{#if recordSince}&nbsp;— records
							since {recordSince}{/if}
					</p>
					<p class="today-line">
						Today's entry is
						<button onclick={() => openDay(todaySlug)} class="today-link">
							{todayFormatted}
						</button>
					</p>
				</header>

				{#if stripDensity.length}
					<!-- The whole span at maximum zoom-out, one row (§17.1): the
					     shape of the record before you read a word of it. Click
					     lands in the console. -->
					<button
						class="strip"
						onclick={() => goTo('/wiki/lifeline')}
						aria-label="Open the lifeline"
					>
						<svg
							viewBox="0 0 {stripDensity.length} 40"
							preserveAspectRatio="none"
							class="strip-svg"
						>
							{#each stripDensity as d, i}
								{#if d > 0}
									{@const peak = Math.max(...stripDensity)}
									{@const h = Math.max(1.5, Math.sqrt(d / peak) * 36)}
									<rect x={i} y={40 - h} width="0.8" height={h} />
								{/if}
							{/each}
						</svg>
						<span class="strip-caption">The whole record — open the lifeline →</span>
					</button>
				{/if}

				<section class="sec">
					<div class="sec-main">
						<h2>Activity</h2>
						{#if loadingActivity}
							<p class="quiet">Loading activity…</p>
						{:else}
							<ActivityHeatmap
								{activityData}
								onDayClick={(_d, slug) => openDay(slug)}
							/>
						{/if}
					</div>
					<aside class="sec-aside">
						{#if !loadingActivity}
							<dl class="stat-stack">
								<div>
									<dt>Days recorded</dt>
									<dd>{activityStats.recorded}</dd>
								</div>
								<div>
									<dt>Narrated</dt>
									<dd>{activityStats.narrated}</dd>
								</div>
								<div>
									<dt>Awaiting narration</dt>
									<dd>{activityStats.stubs}</dd>
								</div>
							</dl>
							<p class="aside-note">The last six months, day by day.</p>
						{/if}
					</aside>
				</section>

				<section class="sec">
					<div class="sec-main">
						<h2>On this day</h2>
						{#if onThisDay.length === 0}
							<p class="quiet">
								No earlier years share this date yet — the record is young.
							</p>
						{:else}
							<ul class="otd">
								{#each onThisDay as entry (entry.date)}
									<li>
										<button class="otd-row" onclick={() => openDay(entry.date)}>
											<span class="otd-year">{yearOf(entry.date)}</span>
											{#if entry.epigraph}
												<span class="otd-epigraph">{entry.epigraph}</span>
											{:else if entry.narrated}
												<span class="otd-epigraph">A narrated day</span>
											{:else}
												<span class="otd-stub">
													{entry.event_count}
													{entry.event_count === 1 ? 'event' : 'events'}, unwritten
												</span>
											{/if}
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
					<aside class="sec-aside">
						<p class="aside-note">{todayFormatted.replace(/,\s*\d{4}$/, '')} in earlier years.</p>
					</aside>
				</section>

				{#if latestEntry}
					<section class="sec">
						<div class="sec-main">
							<h2>The latest entry</h2>
							<button class="featured" onclick={() => openDay(latestEntry!.slug)}>
								<span class="featured-date">{latestEntry.label}</span>
								{#if latestEntry.epigraph}
									<blockquote class="featured-epigraph">
										{latestEntry.epigraph}
									</blockquote>
								{/if}
								<span class="featured-open">Read the entry →</span>
							</button>
						</div>
						<aside class="sec-aside">
							<p class="aside-note">The most recent day the nightly narration has written.</p>
						</aside>
					</section>
				{/if}

				<!-- What changed: the front door to the review loop (§17.4).
				     The entity index that used to sit here duplicated the
				     sidebar and turned the front page into a table of contents;
				     the masthead's computed line carries the counts now. -->
				<section class="sec">
					<div class="sec-main">
						<h2>What changed</h2>
						{#if recentEdits.length === 0}
							<p class="quiet">
								No article edits yet. When an article is written or
								maintained, the edit lands here — with its diff, in History.
							</p>
						{:else}
							<ul class="wc">
								{#each recentEdits as e (e.route + e.version_number)}
									<li>
										<button class="wc-row" onclick={() => windowShellStore.openTabFromRoute(e.route)}>
											<span class="wc-title">{e.title}</span>
											<span class="wc-meta">
												{e.author === 'ai' ? 'the record' : 'you'} ·
												{new Date(e.at).toLocaleDateString('en-US', {
													month: 'short',
													day: 'numeric',
												})}
											</span>
										</button>
									</li>
								{/each}
							</ul>
							<button class="wc-all" onclick={() => goTo('/wiki/history')}>
								All history →
							</button>
						{/if}
					</div>
					<aside class="sec-aside">
						<dl class="stat-stack">
							<div>
								<dt>Open notes</dt>
								<dd>{openNoteCount}</dd>
							</div>
						</dl>
						{#if thinLanes.length || activityStats.stubs > 0}
							<!-- Where it's thin (§17.5): a record that says where it
							     is incomplete is more trustworthy than one that
							     presents itself as finished. -->
							<ul class="thin">
								{#each thinLanes as t (t)}
									<li>{t}</li>
								{/each}
								{#if activityStats.stubs > 0}
									<li>
										{activityStats.stubs}
										{activityStats.stubs === 1 ? 'day' : 'days'} with events, unwritten
									</li>
								{/if}
							</ul>
						{/if}
					</aside>
				</section>
			</div>
		{:else if section === 'lifeline'}
			<!-- Full bleed, and not by preference. Every other section here is
			     prose and belongs in a 42rem measure; a lifeline is a viewport
			     onto eight years, and every pixel of width is a week you can
			     actually see. Putting it in the reading column threw away a
			     third of the record's resolution. -->
			<div class="bleed">
				<LifelineCanvas />
			</div>
		{:else if section === 'history'}
			<div class="measure">
				<WikiHistory />
			</div>
		{:else if section === 'years'}
			<div class="measure">
				{#if !yearsLoaded}
					<p class="quiet">Loading…</p>
				{:else if years.length === 0}
					<p class="quiet">No recorded days yet, so there are no years to show.</p>
				{:else}
					<ul class="years">
						{#each years as y (y.year)}
							<li>
								<a href="/wiki/days">{y.year}</a>
								<span class="quiet">
									{y.recorded} day{y.recorded === 1 ? '' : 's'} · {y.narrated} narrated
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{:else if section === 'days'}
			<div class="days-wrap">
				<DaysChronicle onOpenDay={openDay} />
			</div>
		{:else if section === 'entities'}
			<div class="grid-wrap">
				{#key entityTypePreset}
					<UniversalDataGrid
						items={allEntities}
						columns={entityColumns}
						entityType="entities"
						{loading}
						{error}
						filters={entityFilters}
						emptyIcon="ri:group-line"
						emptyMessage="No people, places, or organizations yet"
						loadingMessage="Loading…"
						searchPlaceholder="Search people, places, organizations…"
						onItemClick={openEntity}
						onItemContextMenu={entityContextMenu}
						onRetry={loadAllEntities}
					>
						{#snippet toolbarActions()}
							<!-- People arrived only by resolution before this: from a
							     contact sync or an email sender. The people who matter
							     most are often the ones you never email. -->
							<button type="button" class="add-entity" onclick={addPerson}>
								<Icon icon="ri:user-add-line" width="14" />
								<span>New person</span>
							</button>
						{/snippet}
						{#snippet tableRow(entity: UnifiedEntity)}
							<td class="col-name">
								<div class="name-cell">
									<Icon icon={typeConfig[entity.entityType].icon} width="16" />
									<span class="name-text">{entity.name}</span>
								</div>
							</td>
							<td class="col-type">
								<span class="badge badge-muted">{typeConfig[entity.entityType].label}</span>
							</td>
							<td class="col-details hide-mobile">
								{#if entity.subtitle}
									<span class="subtitle-text">{entity.subtitle}</span>
								{:else}
									<span class="empty-cell">—</span>
								{/if}
							</td>
						{/snippet}

						{#snippet card(entity: UnifiedEntity)}
							<div class="card-content">
								<Icon icon={typeConfig[entity.entityType].icon} width="28" />
								<span class="card-name">{entity.name}</span>
								<span class="badge badge-muted">{typeConfig[entity.entityType].label}</span>
							</div>
						{/snippet}
					</UniversalDataGrid>
				{/key}
			</div>
		{:else if section === 'identity'}
			<div class="identity-wrap">
				<NarrativeIdentitySection />
				<!-- Where the assistant's proposals wait. It may suggest an
				     addition; it may never make one. -->
				<NotesRail subjectType="narrative_identity" subjectId="nar_identity_001" />
			</div>
		{:else if section === 'chapters'}
			<div class="identity-wrap">
				<ChaptersSection />
			</div>
		{/if}
	</main>
</div>

<style>
	.wiki-view {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	/* Mirrors the Page shell's box model, not just its numbers: padding on the
	   OUTER scroller, measure centred INSIDE it. Padding within the centred
	   block instead (what this used to do) lands the text ~48px further in
	   than every neighbouring room, so the columns don't line up when you move
	   between tabs even though both claim 72rem. */
	/* This room hand-rolls the Page shell, so it hand-rolls the shell's phone
	   gutter too — 3rem a side leaves 279px of a 375px screen, and the room's
	   own grids then push the page sideways. Mobile-first at the same 768px
	   step `Page.svelte` uses, so the two shells change measure on exactly the
	   same pixel. */
	.content {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		/* Bottom room is the home indicator's safe-area only — the phone has no
		   bottom chrome anymore. Same measure as `Page.svelte`. */
		padding: 2rem 1.25rem calc(1.5rem + env(safe-area-inset-bottom));
	}

	@media (min-width: 768px) {
		.content {
			padding: 3rem;
		}
	}

	/* Two measures, not five — the same rule the Page shell states, applied to
	   this room's hand-rolled wraps. `wide` (72rem) for anything gridded,
	   `prose` (48rem) for reading, both on the shell's 3rem padding. These
	   sections used to run 72 / 54 / 50 / 44rem at 1.5rem padding, so moving
	   between the room's own tabs — and between the room and its neighbours —
	   shifted the column every time. */
	.grid-wrap,
	.days-wrap {
		max-width: 72rem;
		width: 100%;
		margin: 0 auto;
	}

	.identity-wrap {
		max-width: 48rem;
		width: 100%;
		margin: 0 auto;
	}

	.add-entity {
		display: inline-flex;
		align-items: center;
		gap: 0.3125rem;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.8125rem;
		color: var(--color-foreground-subtle);
		cursor: pointer;
	}

	.add-entity:hover {
		color: var(--color-foreground);
		background: var(--color-surface-hover);
	}

	/* ===== Overview: essay column + marginalia rail ===== */

	.ovw {
		max-width: 72rem;
		width: 100%;
		margin: 0 auto;
		padding-bottom: 1rem;
	}

	.mast {
		margin-bottom: 2.5rem;
	}

	.mast h1 {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 2rem;
		font-weight: 500;
		letter-spacing: -0.01em;
		color: var(--color-foreground);
		margin: 0 0 0.625rem;
	}

	.standfirst {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 1.0625rem;
		line-height: 1.5;
		color: var(--color-foreground-muted);
		margin: 0 0 1rem;
		max-width: 40rem;
	}

	.record-line {
		font-size: 0.875rem;
		color: var(--color-foreground-subtle);
		font-variant-numeric: tabular-nums;
		margin: 0 0 0.375rem;
	}

	.today-line {
		font-size: 0.875rem;
		color: var(--color-foreground-muted);
		margin: 0;
	}

	/* The lifeline strip: one row, hairline-quiet, the whole span. */
	.strip {
		display: block;
		width: 100%;
		margin: 0 0 2.5rem;
		padding: 0;
		background: none;
		border: none;
		border-bottom: 1px solid var(--color-border);
		cursor: pointer;
		text-align: left;
	}

	.strip-svg {
		display: block;
		width: 100%;
		height: 40px;
	}

	.strip-svg rect {
		fill: var(--color-foreground-muted);
		fill-opacity: 0.5;
	}

	.strip:hover .strip-svg rect {
		fill: var(--color-primary);
		fill-opacity: 0.55;
	}

	.strip-caption {
		display: block;
		padding: 0.375rem 0 0.5rem;
		font-size: 0.8125rem;
		color: var(--color-foreground-subtle);
	}

	.strip:hover .strip-caption {
		color: var(--color-primary);
	}

	.today-link {
		color: var(--color-primary);
		background: none;
		border: none;
		padding: 0;
		font: inherit;
		font-weight: 500;
		cursor: pointer;
	}

	.today-link:hover {
		text-decoration: underline;
	}

	/* Each section is one grid row: the essay column and its margin. */
	.sec {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 11rem;
		gap: 2.25rem;
		padding: 1.75rem 0;
		border-top: 1px solid var(--color-border);
	}

	.sec-main h2 {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 1.25rem;
		font-weight: 400;
		color: var(--color-foreground);
		margin: 0 0 1rem;
	}

	/* Marginalia: quiet, small, aligned to the section's first baseline. */
	.sec-aside {
		padding-top: 0.375rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.aside-note {
		font-size: 0.6875rem;
		line-height: 1.5;
		letter-spacing: 0.02em;
		color: var(--color-foreground-subtle);
		margin: 0;
	}

	.stat-stack {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.stat-stack div {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}

	.stat-stack dt {
		font-size: 0.6875rem;
		letter-spacing: 0.02em;
		color: var(--color-foreground-subtle);
		flex: 1;
	}

	.stat-stack dd {
		margin: 0;
		font-size: 0.8125rem;
		font-variant-numeric: tabular-nums;
		color: var(--color-foreground);
	}

	.quiet {
		font-size: 0.875rem;
		color: var(--color-foreground-subtle);
		margin: 0;
	}

	/* Years: a plain index, set to the reading measure. */
	.measure {
		max-width: 42rem;
		padding: 1.5rem 0;
	}

	/* The one section that is not a document. Fills the room's width and its
	   remaining height, so lanes get room to be read rather than sitting in a
	   200px band under a page of white. */
	.bleed {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		min-height: 0;
		padding: 1rem 0;
	}

	.years {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.years li {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		align-items: baseline;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.years a {
		color: var(--color-foreground);
		text-decoration: none;
	}

	.years a:hover {
		text-decoration: underline;
	}

	/* On this day */
	.otd {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.otd-row {
		display: flex;
		align-items: baseline;
		gap: 1rem;
		width: 100%;
		padding: 0.4375rem 0;
		background: none;
		border: none;
		border-bottom: 1px solid color-mix(in srgb, var(--color-border) 45%, transparent);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.otd-row:last-child {
		border-bottom: none;
	}

	.otd-year {
		flex: none;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		color: var(--color-foreground-muted);
	}

	.otd-row:hover .otd-year {
		color: var(--color-primary);
	}

	.otd-epigraph {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 0.9375rem;
		color: var(--color-foreground);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.otd-stub {
		font-size: 0.8125rem;
		color: var(--color-foreground-subtle);
	}

	/* Featured entry */
	.featured {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.625rem;
		width: 100%;
		background: none;
		border: none;
		padding: 0;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.featured-date {
		font-size: 0.75rem;
		letter-spacing: 0.04em;
		color: var(--color-foreground-muted);
	}

	.featured-epigraph {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 1.1875rem;
		line-height: 1.5;
		color: var(--color-foreground);
		margin: 0;
		padding-left: 1rem;
		border-left: 2px solid var(--color-border);
	}

	.featured-open {
		font-size: 0.8125rem;
		color: var(--color-primary);
	}

	.featured:hover .featured-open {
		text-decoration: underline;
	}

	/* What changed */
	.wc {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.wc-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 1rem;
		width: 100%;
		padding: 0.4375rem 0;
		background: none;
		border: none;
		border-bottom: 1px solid var(--color-border);
		cursor: pointer;
		font: inherit;
		text-align: left;
	}

	.wc-title {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 0.9375rem;
		color: var(--color-foreground);
	}

	.wc-row:hover .wc-title {
		color: var(--color-primary);
	}

	.wc-meta {
		font-size: 0.8125rem;
		color: var(--color-foreground-subtle);
		white-space: nowrap;
	}

	.wc-all {
		margin-top: 0.625rem;
		background: none;
		border: none;
		padding: 0;
		font-size: 0.8125rem;
		color: var(--color-primary);
		cursor: pointer;
	}

	/* Where it's thin */
	.thin {
		list-style: none;
		margin: 0.875rem 0 0;
		padding: 0;
		font-size: 0.8125rem;
		color: var(--color-foreground-subtle);
	}

	.thin li {
		margin-bottom: 0.25rem;
	}

	/* Unified entity grid cells */
	.name-cell {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-foreground-subtle);
	}

	.name-text {
		font-weight: 500;
		color: var(--color-foreground);
	}

	.subtitle-text {
		color: var(--color-foreground-muted);
		font-size: 0.8125rem;
		text-transform: capitalize;
	}

	.empty-cell {
		color: var(--color-foreground-subtle);
	}

	.col-name {
		width: 45%;
		min-width: 200px;
		padding: 0.625rem 0.75rem;
		padding-left: 0;
	}

	.col-type {
		width: 20%;
		min-width: 120px;
		padding: 0.625rem 0.75rem;
	}

	.col-details {
		width: 35%;
		min-width: 140px;
		padding: 0.625rem 0.75rem;
		padding-right: 0;
	}

	/* Card mode */
	.card-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		text-align: center;
		color: var(--color-foreground-subtle);
	}

	.card-name {
		font-weight: 600;
		font-size: 0.9375rem;
		color: var(--color-foreground);
		line-height: 1.3;
	}

	@media (max-width: 768px) {
		.hide-mobile {
			display: none;
		}
	}

	/* The margin collapses before the essay column does. */
	@media (max-width: 880px) {
		.sec {
			grid-template-columns: 1fr;
			gap: 0.875rem;
		}

		.sec-aside {
			padding-top: 0;
		}

		.stat-stack {
			flex-direction: row;
			gap: 1.25rem;
		}
	}

	/* Touch sizing, last in the sheet so it wins on source order against the
	   base rules it restates (same selectors, same specificity — a media query
	   adds none).

	   `.today-link` is deliberately not here: it is a link inside a sentence,
	   and a 44pt box around it would reach into the lines above and below and
	   swallow their taps. Inline prose links are the honest exception to the
	   floor; a list row is not. */
	@media (max-width: 768px), (pointer: coarse) {
		.wc-row {
			padding: 0.75rem 0;
		}
	}

</style>