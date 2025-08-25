<script lang="ts">
	import { getTimelineContext } from "./TimelineContext.svelte";
	import { createDateForHour } from "$lib/utils/timezone";

	export let selectedDate: string; // YYYY-MM-DD format
	export let userTimezone: string;

	const { timeToPixel } = getTimelineContext();

	// Generate hour markers
	const hours = Array.from({ length: 24 }, (_, i) => i);

	// Helper to format hour display
	function formatHour(hour: number): string {
		if (hour === 0) return "00:00";
		if (hour < 10) return `0${hour}:00`;
		return `${hour}:00`;
	}

	// Helper to determine if hour should have darker styling
	function isMajorHour(hour: number): boolean {
		return hour === 0 || hour === 6 || hour === 12 || hour === 18;
	}
</script>

<div class="timeline-legend">
	{#each hours as hour}
		{@const [year, month, day] = selectedDate.split('-').map(Number)}
		{@const dateForHour = createDateForHour(year, month, day, hour, userTimezone)}
		{@const x = timeToPixel(dateForHour)}
		<div
			class="hour-label"
			class:major={isMajorHour(hour)}
			style="left: {x}px"
		>
			{formatHour(hour)}
		</div>
	{/each}
</div>

<style>
	@reference "../../../../app.css";

	.timeline-legend {
		@apply relative h-full pointer-events-none;
	}

	.hour-label {
		@apply absolute top-1/2 -translate-y-1/2 pl-0.5 text-[10px] font-mono text-neutral-500 whitespace-nowrap select-none;
	}

	/* .hour-label.major {
		@apply  text-neutral-700;
	} */
</style>
