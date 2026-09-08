<!--
	ChapterLifeline.svelte

	The lifeline as a plate, cropped to its life-level view — the horizontal
	of the opening's example table: one wire from α to Ω, the same fictional
	life's chapters as boxes on it, their names in the dimension lane above,
	the age ruler below, now marked, one planned chapter drafted in dashes.

	Ported from the "Alpha to Omega" prototype (the full zoomable plate lives
	there; this is the static crop the interview opening earns — enough to
	show what a partition of a life LOOKS like before someone writes theirs).
	The life is the repo's reserved fictional one (Sarah, Nick, Maya, David
	Okafor); nothing here is a real person's.
-->

<script lang="ts">
	// ── the fictional life, verbatim from the prototype ──
	const YR = 365.25 * 24 * 3600e3;
	const BIRTH = new Date(1997, 3, 2).getTime();
	const NOW = new Date(2026, 7, 17).getTime();
	const BOX = new Date(2025, 1, 9).getTime(); // the record begins

	interface Chapter {
		t0: number;
		t1: number | null;
		label: string;
		ep: string;
	}
	const CHAPTERS: Chapter[] = [
		// A made-up life, the same one the opening's table lists (ChatView).
		{ t0: BIRTH, t1: new Date(2003, 7, 20).getTime(), label: "Childhood on the coast", ep: "three towns before the first classroom" },
		{ t0: new Date(2003, 7, 20).getTime(), t1: new Date(2009, 5, 10).getTime(), label: "Grade school, inland", ep: "snow days and the lake" },
		{ t0: new Date(2009, 5, 10).getTime(), t1: new Date(2016, 7, 20).getTime(), label: "The band years", ep: "the garage after hours" },
		{ t0: new Date(2016, 7, 20).getTime(), t1: new Date(2020, 4, 12).getTime(), label: "College", ep: "everything new at once" },
		{ t0: new Date(2020, 4, 12).getTime(), t1: new Date(2021, 8, 1).getTime(), label: "Locked down", ep: "a year at a desk, itching" },
		{ t0: new Date(2021, 8, 1).getTime(), t1: new Date(2023, 6, 1).getTime(), label: "The first shop", ep: "the first real build" },
		{ t0: new Date(2023, 6, 1).getTime(), t1: new Date(2025, 5, 1).getTime(), label: "The workshop", ep: "two years of hard problems" },
		{ t0: new Date(2025, 5, 1).getTime(), t1: null, label: "Out on my own", ep: "the shop with my name on it" },
	];
	const PLANNED = { t0: NOW + 10 * (YR / 12), t1: NOW + 4.2 * YR, label: "The shop, grown" };

	/* The plate's palette — one muted color per chapter, cycled. */
	const CH_COLORS = ["#B07514", "#2E6B43", "#1E4E8C", "#1E3159", "#6C7185", "#7E5A2E"];

	// ── geometry: the prototype's sheet, cropped to one static window ──
	const PX0 = 60, PX1 = 1220;
	const BASE = 148;   // the wire
	const DIM = 100;    // the dimension lane's hairline
	const BH = 16;      // box half-height
	// now sits at 72% of the sheet, as the prototype boots
	const LO = BIRTH;
	const HI = BIRTH + (NOW - BIRTH) / 0.72;
	const X = (t: number) => PX0 + ((t - LO) / (HI - LO)) * (PX1 - PX0);

	/* Deterministic coverage strips — dense and live after the box arrived,
	   faint imported traces before it. Same idea as the prototype's covSegs,
	   simplified to what reads at this size. */
	function rnd(a: number, b: number): number {
		let h = (2166136261 ^ Math.imul(a | 0, 374761393) ^ Math.imul(b | 0, 668265263)) | 0;
		h = Math.imul(h ^ (h >>> 13), 1274126177);
		return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
	}
	function coverage(ci: number, t0: number, t1: number): { x: number; w: number; o: number }[] {
		const segs: { x: number; w: number; o: number }[] = [];
		const xa = X(t0), xb = X(Math.min(t1, NOW));
		const n = Math.max(3, Math.round((xb - xa) / 9));
		for (let i = 0; i < n; i++) {
			const fa = xa + ((xb - xa) * i) / n;
			const t = t0 + ((t1 - t0) * i) / n;
			const live = t >= BOX;
			const r = rnd(ci * 97 + i, 5);
			if (!live && r < 0.55) continue; // imported traces are sparse
			segs.push({ x: fa, w: (xb - xa) / n - 1.4, o: live ? 0.5 : 0.16 + r * 0.14 });
		}
		return segs;
	}

	const yearOf = (t: number) => new Date(t).getFullYear();
	/* Birthday ticks: the same tick read in both coordinate systems —
	   the world's calendar above, the life's age below. */
	const ages = Array.from({ length: 40 }, (_, i) => i + 1)
		.map((age) => {
			const d = new Date(BIRTH);
			d.setFullYear(d.getFullYear() + age);
			return { age, t: d.getTime() };
		})
		.filter((a) => X(a.t) >= PX0 && X(a.t) <= PX1);

	// The planned chapter's span, clamped to the sheet.
	const plannedX0 = X(PLANNED.t0);
	const plannedX1 = Math.min(X(PLANNED.t1), PX1);

	let readout = $state<string | null>(null);
</script>

<figure class="lifeline">
	<svg viewBox="0 0 1280 232" role="img" aria-label="The same fictional life as the table, drawn on one wire: chapters as spans from birth toward now, named above, aged below.">
		<!-- the wire IS the life: it begins at α and runs toward Ω -->
		<line x1={PX0} y1={BASE} x2={PX1} y2={BASE} class="wire" />
		<circle cx={PX0} cy={BASE} r="3" class="alpha-dot" />
		<text x={PX0 - 14} y={BASE + 5} text-anchor="end" class="t-omega">α</text>
		<text x={PX1 + 14} y={BASE + 5} text-anchor="start" class="t-omega">Ω</text>

		<!-- the age ruler: years above the ticks, the life's age below -->
		{#each ages as a (a.age)}
			<line x1={X(a.t)} y1={BASE + BH + 14} x2={X(a.t)} y2={BASE + BH + 19} class="tick" class:fut={a.t > NOW} />
			{#if a.age % 5 === 0}
				<text x={X(a.t)} y={BASE + BH + 10} text-anchor="middle" class="t-age" class:fut={a.t > NOW}>{yearOf(a.t)}</text>
				<text x={X(a.t)} y={BASE + BH + 30} text-anchor="middle" class="t-age" class:fut={a.t > NOW}>{a.age}</text>
			{/if}
		{/each}

		<!-- chapters: boxes on the wire, coverage inside, names in the lane above -->
		{#each CHAPTERS as c, ci (c.t0)}
			{@const x0 = X(c.t0)}
			{@const x1 = X(c.t1 ?? NOW)}
			{@const color = CH_COLORS[ci % CH_COLORS.length]}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<g
				class="blk"
				onmouseenter={() => (readout = `${c.label} — ${c.ep}`)}
				onmouseleave={() => (readout = null)}
			>
				<rect x={x0} y={BASE - BH} width={x1 - x0} height={BH * 2} class="box" style={`--ch:${color}`} />
				{#each coverage(ci, c.t0, c.t1 ?? NOW) as s, si (si)}
					<rect x={s.x} y={BASE - BH + 3} width={Math.max(s.w, 1)} height={BH * 2 - 6} fill={color} fill-opacity={s.o} />
				{/each}
				<!-- the dimension lane: a hairline with end ticks, the drafted name above -->
				<line x1={x0 + 1} y1={DIM} x2={c.t1 === null ? x1 : x1 - 1} y2={DIM} class="dim" />
				<line x1={x0 + 1} y1={DIM} x2={x0 + 1} y2={DIM + 5} class="dim" />
				{#if c.t1 !== null}
					<line x1={x1 - 1} y1={DIM} x2={x1 - 1} y2={DIM + 5} class="dim" />
				{/if}
				<text x={(x0 + x1) / 2} y={DIM - 7} text-anchor="middle" class="t-dim" class:t-dim-tight={x1 - x0 < 92}>
					{c.label}
				</text>
			</g>
		{/each}

		<!-- the planned chapter: drafted, not lived — dashed, in intent red -->
		<line x1={plannedX0} y1={DIM} x2={plannedX1} y2={DIM} class="dim planned" />
		<text x={(plannedX0 + plannedX1) / 2} y={DIM - 7} text-anchor="middle" class="t-dim planned-t">{PLANNED.label}</text>

		<!-- now: the one claret vertical -->
		<line x1={X(NOW)} y1={DIM - 26} x2={X(NOW)} y2={BASE + BH + 6} class="now" />
		<text x={X(NOW)} y={DIM - 32} text-anchor="middle" class="t-now">now</text>
	</svg>
	<figcaption class="readout" class:idle={!readout}>
		{readout ?? "The same life, horizontally — every day falls inside exactly one chapter."}
	</figcaption>
</figure>

<style>
	.lifeline {
		margin: 1.25rem 0 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		padding: 1rem 0.75rem 0.5rem;
		background: var(--color-background);
	}

	svg {
		display: block;
		width: 100%;
		height: auto;
	}

	.wire {
		stroke: var(--color-foreground);
		stroke-width: 1.2;
	}

	.alpha-dot {
		fill: var(--color-foreground);
	}

	.t-omega {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 20px;
		fill: var(--color-foreground);
	}

	.tick {
		stroke: var(--color-foreground);
		stroke-opacity: 0.45;
		stroke-width: 0.6;
	}

	.tick.fut {
		stroke-opacity: 0.22;
	}

	.t-age {
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 11px;
		fill: var(--color-foreground-subtle);
		font-variant-numeric: tabular-nums;
	}

	.t-age.fut {
		opacity: 0.5;
	}

	.box {
		fill: var(--ch);
		fill-opacity: 0.07;
		stroke: var(--color-foreground);
		stroke-opacity: 0.35;
		stroke-width: 0.8;
	}

	.blk:hover .box {
		fill-opacity: 0.14;
		stroke-opacity: 0.6;
	}

	.dim {
		stroke: var(--color-foreground-subtle);
		stroke-width: 0.7;
	}

	.dim.planned {
		stroke: #9a2b2e;
		stroke-dasharray: 3 4;
	}

	.t-dim {
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 12.5px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		fill: var(--color-foreground-muted);
	}

	.t-dim-tight {
		font-size: 9.5px;
		letter-spacing: 0.04em;
	}

	.planned-t {
		fill: #9a2b2e;
		fill-opacity: 0.85;
	}

	.now {
		stroke: #9a2b2e;
		stroke-width: 1;
	}

	.t-now {
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 11px;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		fill: #9a2b2e;
	}

	.readout {
		margin: 0.375rem 0.25rem 0.25rem;
		font-size: 0.8125rem;
		color: var(--color-foreground-muted);
		min-height: 1.2em;
	}

	.readout.idle {
		color: var(--color-foreground-subtle);
	}
</style>
