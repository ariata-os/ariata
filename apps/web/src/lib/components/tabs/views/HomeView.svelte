<!--
	HomeView.svelte — the home page.

	Every other room here is one-directional: the wiki is you reading the
	record, chat is you interrogating it, the day page is its account of you
	written overnight. Home is the only place the record and the person are in
	the room at the same time, with the day still open — so the page is built
	as turns of that meeting rather than as a dashboard.

	  speaks   one counted observation, no model involved
	  shows    today, from the raw streams, scrubbable down to the rows
	  opens    the work you had in your hands last
	  asks     the one thing only the owner can answer
	  answers  the one line you choose to keep

	A rule is the box talking; a card is you answering. That is the only
	decoration the page has, and it is load-bearing — so a new block on this
	page has to pick a side before it picks a style.

	Today is drawn from raw streams because the interpreted dayline is composed
	overnight — rather than wait for prose, this draws the record itself while
	it is still arriving. Nothing is written to fill a slot: a section with
	nothing to say does not render.

	Live means polling: there is no ingest broadcast in the box yet, so the deck
	refetches on a 30s beat and stops entirely while the tab is hidden.
-->
<script lang="ts">
	import { onMount } from "svelte";
	import {
		getDayByDate,
		listNotes,
		createNote,
		getDayTimeline,
		getTodayStreams,
		getDayHeartRate,
		getLifeline,
		getWeatherNow,
		getFeed,
		type LifelineRecord,
		type WikiDayApi,
		type WikiNote,
		type TimelineDayPoint,
		type TodayStreamsView,
		type DayHeartRateSample,
		type LifelineData,
		type WeatherNow,
	} from "$lib/wiki/api";
	import { getStreamHealth, type StreamHealth } from "$lib/api/client";
	import Icon from "$lib/components/Icon.svelte";
	import { notebookStore } from "$lib/stores/notebook.svelte";
	import { pagesStore } from "$lib/stores/pages.svelte";
	import { chatSessions } from "$lib/stores/chatSessions.svelte";
	import { windowShellStore } from "$lib/stores/window-shell.svelte";
	import GettingStarted from "$lib/components/home/GettingStarted.svelte";
	import DayDeck from "$lib/components/home/DayDeck.svelte";

	// Which page this is — written by GettingStarted, read here. While any
	// getting-started section remains ("focus"), those sections ARE the page:
	// no subtitle, no day stepper, no deck of silent tracks sharing the
	// screen with them. Home's own furniture exists only at "settled", when
	// getting started has retired entirely.
	let gsPhase = $state<"loading" | "focus" | "settled">("loading");
	import DayGround from "$lib/components/home/DayGround.svelte";
	import DayNovelty from "$lib/components/home/DayNovelty.svelte";
	import PlaceAsk from "$lib/components/home/PlaceAsk.svelte";

	// A `TEMP-VERIFY` global fetch patch lived here until 2026-08-05: it
	// rewrote every `/api/` call to `http://127.0.0.1:7117` from this module's
	// body. Removed because it overrode all three real paths at once — the
	// vite dev proxy (`/api` → :8000, vite.config.ts), same-origin on the
	// box-served desktop app, and the mobile shell's injected origin
	// (`initBackendFromShell`, lib/config/backend.ts). It patched
	// `window.fetch` globally, so the blast radius was the whole app, not this
	// view. It also reached TestFlight in 1.2.5.
	//
	// If you need this view pointed at a real box while developing, set
	// `BACKEND_URL` for the vite proxy rather than patching fetch.

	// ---- the day's window ----
	// The browser's zone decides which day this is and where its midnights
	// fall; the same zone is sent to the server so both ends agree. Day length
	// is measured rather than assumed — a DST day is 23 or 25 hours long.
	function ymd(d: Date): string {
		return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
	}
	const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const _now = new Date();
	const todayDate = ymd(_now);
	const yesterdayDate = ymd(new Date(_now.getTime() - 86400000));
	const dayStartMs = new Date(`${todayDate}T00:00:00`).getTime();
	const dayEndMs = new Date(`${ymd(new Date(_now.getTime() + 86400000))}T00:00:00`).getTime();

	// ---- clock ----
	let nowMs = $state(Date.now());
	let clock = $state("");
	let dateline = $state("");
	function tick() {
		const d = new Date();
		nowMs = d.getTime();
		const ap = d.getHours() >= 12 ? "pm" : "am";
		clock = `${d.getHours() % 12 || 12}:${String(d.getMinutes()).padStart(2, "0")} ${ap}`;
		dateline = d.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });
	}

	// ---- state ----
	let today = $state<WikiDayApi | null>(null);
	let streams = $state<TodayStreamsView | null>(null);
	let heart = $state<DayHeartRateSample[]>([]);
	let life = $state<LifelineData | null>(null);
	let points = $state<TimelineDayPoint[]>([]);
	let health = $state<Record<string, StreamHealth>>({});
	let weather = $state<WeatherNow | null>(null);
	let notes = $state<WikiNote[]>([]);
	let scrubMs = $state<number | null>(null);
	let pinnedMs = $state<number | null>(null);

	// ---- the moment: the rows that actually fall where you clicked ----
	// A timeline that cannot hand back its rows is a picture of a life rather
	// than a way into one. The window is ±15 minutes, matching the deck's own
	// bucket, so what you point at is what you get.
	const MOMENT_MS = 15 * 60_000;
	let moment = $state<{ at: number; records: LifelineRecord[]; more: boolean } | null>(null);
	let momentLoading = $state(false);

	$effect(() => {
		const at = pinnedMs;
		if (at == null) {
			moment = null;
			return;
		}
		let dropped = false;
		momentLoading = true;
		getFeed(new Date(at - MOMENT_MS).toISOString(), new Date(at + MOMENT_MS).toISOString(), { limit: 24 })
			.then((f) => {
				if (dropped) return;
				moment = { at, records: f?.records ?? [], more: f?.has_more ?? false };
			})
			.catch(() => {
				if (!dropped) moment = { at, records: [], more: false };
			})
			.finally(() => {
				if (!dropped) momentLoading = false;
			});
		return () => {
			dropped = true;
		};
	});

	// ---- the live set: everything the deck redraws from ----
	async function refresh() {
		const from = new Date(dayStartMs).toISOString();
		const to = new Date(dayEndMs).toISOString();
		await Promise.allSettled([
			// getTodayStreams sends the browser zone itself.
			getTodayStreams(todayDate).then((s) => (streams = s)),
			getDayHeartRate(todayDate, tz).then((h) => (heart = h)),
			getDayTimeline(todayDate).then((t) => {
				if (t?.points) points = t.points;
			}),
			// One call carries three tracks: steps, screen time, messages sent.
			getLifeline(96, from, to, undefined, {
				health: "steps",
				activity: "screen",
				communication: "sent",
			}).then((l) => (life = l)),
			getDayByDate(todayDate).then((d) => {
				today = d;
				if (d?.id) return listNotes("day", d.id).then((n) => (notes = n));
			}),
		]);
	}

	onMount(() => {
		tick();
		refresh();
		getWeatherNow().then((w) => (weather = w)).catch(() => {});
		// Arm state changes on the scale of connecting a source, not of a day.
		getStreamHealth()
			.then((rows) => (health = Object.fromEntries(rows.map((r) => [r.name, r]))))
			.catch(() => {});
		notebookStore.load?.();
		if (!pagesStore.pages.length) pagesStore.loadPages();
		if (!chatSessions.sessions.length && !chatSessions.isLoading) chatSessions.load();

		const clockId = setInterval(tick, 10_000);
		let dataId = setInterval(refresh, 30_000);
		// A background tab has nothing to show; it should not keep the box busy.
		function onVisible() {
			clearInterval(dataId);
			if (!document.hidden) {
				tick();
				refresh();
				dataId = setInterval(refresh, 30_000);
			}
		}
		document.addEventListener("visibilitychange", onVisible);
		return () => {
			clearInterval(clockId);
			clearInterval(dataId);
			document.removeEventListener("visibilitychange", onVisible);
		};
	});

	function open(route: string, label?: string) {
		windowShellStore.openTabFromRoute(route, label ? { label } : undefined);
	}

	// ---- the page's subtitle: the day, the weather, the time ----
	/** Under the dateline: the weather, then the clock. */
	const subtitle = $derived.by(() => {
		const wx =
			weather && weather.temperature_c != null
				? `${Math.round((weather.temperature_c * 9) / 5 + 32)}° ${weather.condition}`.trim()
				: null;
		return [wx, clock].filter(Boolean).join(" · ");
	});

	// ---- recents: notebooks, pages and chats blended by recency ----
	// Not "desk" — the sidebar's Desk is the pinned shelf, and two different
	// meanings for one word is one too many.
	type RecentItem = { route: string; title: string; kind: string; ts: number; note?: string };
	const recentItems = $derived.by<RecentItem[]>(() => {
		const nb: RecentItem[] = notebookStore.notebooks.map((n: any) => ({
			route: `/notebook/${n.id}`,
			title: n.name || "Untitled",
			kind: "notebook",
			ts: n.updated_at ? Date.parse(n.updated_at) : 0,
			note: n.current_status ? "live" : undefined,
		}));
		const pg: RecentItem[] = pagesStore.pages.map((p) => ({
			route: `/page/${p.id}`,
			title: p.title || "Untitled",
			kind: "page",
			ts: p.updated_at ? Date.parse(p.updated_at) : 0,
		}));
		const ch: RecentItem[] = chatSessions.sessions.map((c) => ({
			route: `/chat/${c.conversation_id}`,
			title: c.title || "Untitled",
			kind: "chat",
			ts: c.last_updated ? Date.parse(c.last_updated) : 0,
		}));
		return [...nb, ...pg, ...ch].sort((a, b) => b.ts - a.ts).slice(0, 5);
	});

	function ago(iso: string | null): string {
		if (!iso) return "";
		const m = Math.round((Date.now() - Date.parse(iso)) / 60000);
		if (isNaN(m)) return "";
		if (m < 60) return `${m}m`;
		const h = Math.floor(m / 60);
		if (h < 24) return `${h}h`;
		const d = Math.floor(h / 24);
		return d < 7 ? `${d}d` : `${Math.floor(d / 7)}w`;
	}

	// ---- the keep: one line, stored as a note on today ----
	// A line you choose is note-shaped: it is about the day, not the day's own
	// account of itself. Longform belongs in the day's article, which you claim
	// by editing it. Saving stays here — you can keep a second line, and see
	// what you already kept, without losing the page.
	let keepText = $state("");
	let keeping = $state(false);
	let keepError = $state<string | null>(null);
	let keepEl = $state<HTMLTextAreaElement | undefined>(undefined);
	const keptToday = $derived(notes.filter((n) => n.author === "human"));

	/**
	 * Grow to the text, up to a point — past that it wants the day's article.
	 *
	 * `auto` before measuring, so an already-tall box reports its content rather
	 * than latching at whatever height it reached and never shrinking back.
	 */
	function grow() {
		const el = keepEl;
		if (!el) return;
		el.style.height = "auto";
		el.style.height = `${Math.min(el.scrollHeight, 160)}px`;
	}
	// Driven by the value rather than by the input event, so it is correct after
	// a programmatic clear (saving) as well as after typing.
	$effect(() => {
		keepText;
		grow();
	});

	async function keep() {
		const body = keepText.trim();
		if (!body || keeping) return;
		keeping = true;
		keepError = null;
		try {
			const day = today ?? (await getDayByDate(todayDate));
			if (!day?.id) throw new Error("no day");
			const saved = await createNote("day", day.id, body, "memo");
			notes = [...notes, saved];
			keepText = "";
		} catch {
			keepError = "That didn't save. Your server may be offline — try again.";
		} finally {
			keeping = false;
		}
	}
	/** Most rows carry no `kind` of their own; the ontology names them well
	 *  enough once its domain prefix is dropped. */
	function recKind(r: LifelineRecord): string {
		if (r.kind) return r.kind;
		return r.ontology.replace(/^[a-z]+_/, "").replace(/_/g, " ");
	}
	function recTime(iso: string): string {
		const d = new Date(iso);
		return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
	}
	function keptTime(iso: string): string {
		const d = new Date(iso);
		const ap = d.getHours() >= 12 ? "pm" : "am";
		return `${d.getHours() % 12 || 12}:${String(d.getMinutes()).padStart(2, "0")} ${ap}`;
	}
</script>

<!-- First run: getting started is the page, whole and alone — a spread that
     bleeds to the pane and carries its own title, so it lives OUTSIDE the Page
     shell. It is mounted exactly once and hidden, not destroyed, when the box
     settles: it is what computes `gsPhase`, and re-creating it on a phase
     change made it and this view chase each other (see its header). -->
<div class="host" class:settled={gsPhase === "settled"}>
	<GettingStarted bind:phase={gsPhase} />
</div>

{#if gsPhase === "settled"}
<div class="host">
<div class="spread">
	<section class="work">
		<div class="head">
			<h1 class="title">{dateline}</h1>
			{#if subtitle}<p class="sub">{subtitle}</p>{/if}
		</div>

		<!-- The box speaks — today's rhythm against the trailing twelve weeks. -->
		<DayNovelty {dayStartMs} {nowMs} {tz} />

		<section class="today">
			<div class="tbody" class:solo={points.length <= 1}>
				<DayDeck
					{dayStartMs}
					{dayEndMs}
					{nowMs}
					{streams}
					{heart}
					{life}
					{points}
					sleepCycles={today?.sleep_cycles ?? []}
					{health}
					bind:scrubMs
					bind:pinnedMs
				/>
				{#if points.length > 1}
					<aside class="ground">
						<div class="gcanvas"><DayGround {points} scrubMs={scrubMs ?? pinnedMs} {nowMs} /></div>
					</aside>
				{/if}
			</div>

			{#if pinnedMs != null}
				<div class="moment">
					<div class="mhead">
						<span class="mono mt">{keptTime(new Date(pinnedMs).toISOString())}</span>
						<span class="mlabel">± 15 minutes</span>
						<button class="mclose" type="button" onclick={() => (pinnedMs = null)} aria-label="Close this moment">
							<Icon icon="ri:close-line" width="15" />
						</button>
					</div>
					{#if momentLoading && !moment}
						<p class="mnone">Looking…</p>
					{:else if moment && moment.records.length}
						<ul class="mlist">
							{#each moment.records as r (r.id + r.at)}
								<li>
									<span class="mono rt">{recTime(r.at)}</span>
									<span class="rk">{recKind(r)}</span>
									<span class="rb">
										{r.label ?? "—"}{#if r.preview}<span class="rp">{r.preview}</span>{/if}
									</span>
								</li>
							{/each}
						</ul>
						{#if moment.more}<p class="mnone">More rows fall in this window than are shown.</p>{/if}
					{:else}
						<p class="mnone">The record holds nothing here.</p>
					{/if}
				</div>
			{/if}
		</section>

		{#if recentItems.length}
			<section class="recents">
				<h2 class="kicker">Recent</h2>
				{#each recentItems as it (it.route)}
					<div class="line">
						<!-- The kicker's space has to come from CSS: leading whitespace
						     inside the span is trimmed, which rendered "Untitled— page". -->
						<button class="t" type="button" onclick={() => open(it.route, it.title)}>
							{it.title}{#if it.kind !== "notebook"}<span class="s">— {it.kind}</span>{/if}
						</button>
						<span class="d">{it.note ?? ago(new Date(it.ts).toISOString())}</span>
					</div>
				{/each}
			</section>
		{/if}

		<!-- The box asks. -->
		<PlaceAsk />

		<section class="keep">
			<!-- The card is the writing surface: the question is the only prompt,
			     so clicking anywhere that isn't already a control puts the cursor
			     where you'd expect it. -->
			<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
			<div
				class="card"
				onclick={(e) => {
					if (!(e.target as HTMLElement).closest("button")) keepEl?.focus();
				}}
			>
				<h2 class="kq">What do you want to remember from today?</h2>
				<textarea
					bind:this={keepEl}
					bind:value={keepText}
					disabled={keeping}
					rows="1"
					aria-label="What do you want to remember from today?"
					onkeydown={(e) => {
						// Enter saves. A longer thought still wants a second line,
						// so Shift holds it.
						if (e.key === "Enter" && !e.shiftKey) {
							e.preventDefault();
							keep();
						}
					}}
				></textarea>
				{#if keepText.trim() || keeping}
					<div class="krow">
						<span class="khint">Shift + Enter for a new line</span>
						<button class="ksave" type="button" onclick={keep} disabled={keeping}>
							{keeping ? "Saving…" : "Save"}
						</button>
					</div>
				{/if}

				{#if keepError}<p class="kerr">{keepError}</p>{/if}

				{#if keptToday.length}
					<ul class="kept">
						{#each keptToday as n (n.id)}
							<li><span class="kt mono">{keptTime(n.created_at)}</span><span class="kb">{n.body}</span></li>
						{/each}
					</ul>
					<button class="link kfoot" type="button" onclick={() => open(`/day/day_${todayDate}`, "Today")}>
						In the margin of today's page →
					</button>
				{/if}
			</div>
		</section>
	</section>

	<!-- The two adjacent pages. These lived on the frontispiece until
	     2026-09-08; the painting retires with getting started (it is the
	     one framed object the setup spread and Home shared), and the
	     doors stay. -->
	<nav class="pages" aria-label="Adjacent pages">
		<button class="link" type="button" onclick={() => open(`/day/day_${yesterdayDate}`, "Yesterday")}>Yesterday's page →</button>
		<button class="link" type="button" onclick={() => open(`/day/day_${todayDate}`, "Today")}>Today's page →</button>
	</nav>
</div>
</div>
{/if}

<style>
	.mono { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

	/* The host: the pane's full height and its own scroll, for both the
	   getting-started spread and Home's. Getting started is display:none
	   (not unmounted) once Home takes over — see GettingStarted.svelte. */
	.host { height: 100%; overflow-y: auto; }
	.host.settled { display: none; }

	/* One column. Getting Started is a spread — work on the left, painting on
	   the right — but once it retires Home is the work alone, in the page's
	   measure; the painting was the setup spread's object, not Home's. */
	.spread {
		max-width: 920px;
		min-height: calc(100dvh - var(--chrome-row-h, 40px) - 2 * var(--pane-inset, 12px) - 2px);
	}
	.work { padding: 56px 56px 48px 64px; min-width: 0; }
	.pages { display: flex; gap: 24px; padding: 0 56px 48px 64px; }
	@media (max-width: 640px) { .pages { padding: 0 24px 32px; } }
	.work > * { animation: arrive 0.5s ease both; }
	.work > :nth-child(2) { animation-delay: 60ms; }
	.work > :nth-child(3) { animation-delay: 120ms; }
	@media (max-width: 640px) { .work { padding: 32px 24px; } }
	@keyframes arrive { from { opacity: 0; transform: translateY(6px); } }
	@media (prefers-reduced-motion: reduce) { .work > * { animation: none; } }

	/* the dateline is the title; the weather and the clock under it */
	.title { font-family: var(--font-serif); font-weight: 400; font-size: 36px; line-height: 1.1; margin: 0; color: var(--color-foreground); }
	.sub { font-family: var(--font-sans); font-size: 15px; line-height: 1.5; color: var(--color-foreground-muted); margin: 10px 0 0; }
	.head { margin-bottom: 40px; }

	.kicker { font-family: var(--font-sans); font-size: 13px; color: var(--color-foreground-subtle); margin: 0 0 12px; font-weight: 400; }
	.link { font-family: var(--font-sans); font-size: 14px; font-weight: 500; color: var(--color-primary); background: none; border: 0; padding: 0; cursor: pointer; }
	.link:hover { text-decoration: underline; text-underline-offset: 3px; }

	/* the deck */
	.today { padding-bottom: 48px; }
	/* No fixes today means no map — the deck takes the whole width
	   rather than leaving a column of air where it would have been. */
	.tbody { display: grid; grid-template-columns: minmax(0, 1fr) 170px; gap: 24px; align-items: center; }
	.tbody.solo { grid-template-columns: minmax(0, 1fr); }
	.gcanvas { height: 170px; }
	@media (max-width: 780px) {
		.tbody { grid-template-columns: 1fr; }
		.gcanvas { height: 140px; }
	}

	/* the rows behind a point — aligned to the plot, not the lane-name gutter. */
	.moment { margin-top: 24px; margin-left: 62px; max-width: 720px; }
	.mhead { display: flex; align-items: baseline; gap: 10px; margin-bottom: 8px; }
	.mhead .mt { font-size: 12px; color: var(--color-foreground); }
	.mlabel { font-family: var(--font-sans); font-size: 13px; color: var(--color-foreground-subtle); }
	.mclose { margin-left: auto; display: flex; align-items: center; background: none; border: 0; padding: 3px; border-radius: 6px; color: var(--color-foreground-subtle); cursor: pointer; }
	.mclose:hover { background: var(--hover-bg); color: var(--color-foreground); }
	.mlist { list-style: none; margin: 0; padding: 0; }
	.mlist li { display: flex; gap: 12px; align-items: baseline; padding: 4px 0; font-family: var(--font-sans); font-size: 14px; line-height: 1.45; }
	.mlist .rt { font-size: 12px; color: var(--color-foreground-subtle); flex: none; width: 40px; }
	.mlist .rk { font-size: 13px; color: var(--color-foreground-subtle); flex: none; width: 96px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mlist .rb { color: var(--color-foreground); min-width: 0; }
	/* The gap has to come from CSS: leading whitespace inside the span is
	   trimmed by the compiler, which rendered "Deposit$0.38". */
	.mlist .rp { color: var(--color-foreground-muted); margin-left: 0.4em; }
	.mnone { font-family: var(--font-sans); font-size: 13px; color: var(--color-foreground-subtle); margin: 0; }
	@media (max-width: 640px) { .moment { margin-left: 0; } }

	/* recent — the work you had in your hands last, newest first */
	.recents { max-width: 40em; }
	.line { display: flex; align-items: baseline; gap: 16px; padding: 8px 0; }
	.line .t { font-family: var(--font-serif); font-size: 18px; color: var(--color-foreground); min-width: 0; text-align: left; background: none; border: 0; padding: 0; cursor: pointer; }
	.line .t .s { margin-left: 0.34em; color: var(--color-foreground-subtle); }
	.line .t:hover { color: var(--color-primary); }
	.line .d { margin-left: auto; font-family: var(--font-sans); font-size: 13px; color: var(--color-foreground-subtle); white-space: nowrap; flex: none; }
	/* Apple's 44pt floor on touch: the row's padding moves into the button. */
	@media (max-width: 768px), (pointer: coarse) {
		.line { padding: 0; }
		.line .t { padding: 10px 0; }
	}

	/* the keep */
	.keep { margin-top: 48px; max-width: 40em; }
	.card {
		background: var(--color-surface); border: 1px solid var(--color-border);
		border-radius: 12px; padding: 24px;
		transition: border-color 0.2s;
		/* The card is the field. Its border is the only affordance the question
		   needs, so no placeholder has to explain itself. */
		cursor: text;
	}
	.card:hover { border-color: var(--color-border-strong, var(--color-border)); }
	.card:focus-within { border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border)); }
	.kq { font-family: var(--font-serif); font-size: 18px; font-weight: 400; line-height: 1.4; color: var(--color-foreground); margin: 0 0 12px; }
	.card textarea {
		display: block; width: 100%; resize: none; overflow-y: auto; min-height: 1.6em;
		font-family: var(--font-serif); font-size: 17px; line-height: 1.5;
		color: var(--color-foreground); background: none; border: 0; padding: 0;
	}
	.card textarea:focus { outline: none; }
	.krow { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
	.khint { font-family: var(--font-sans); font-size: 12px; color: var(--color-foreground-subtle); }
	.ksave {
		margin-left: auto; flex: none; cursor: pointer;
		font-family: var(--font-sans); font-size: 14px; font-weight: 500;
		background: none; border: 0; padding: 0;
		color: var(--color-primary);
	}
	.ksave:hover:not(:disabled) { text-decoration: underline; text-underline-offset: 3px; }
	.ksave:disabled { color: var(--color-foreground-disabled); cursor: default; }
	.kerr { font-family: var(--font-sans); font-size: 13px; color: var(--color-error); margin: 12px 0 0; }
	.kept { list-style: none; margin: 20px 0 0; padding: 0; }
	.kept li { display: flex; gap: 16px; align-items: baseline; padding: 6px 0; }
	.kept .kt { font-size: 12px; color: var(--color-foreground-subtle); flex: none; width: 64px; }
	.kept .kb { font-family: var(--font-serif); font-size: 16px; line-height: 1.45; color: var(--color-foreground); }
	.kfoot { margin-top: 12px; }
</style>
