<script lang="ts">
	import { getTimelineContext } from "./TimelineContext.svelte";
	import { createDateForHour } from "$lib/utils/timezone";

	let { selectedDate, userTimezone }: { selectedDate: string; userTimezone: string } = $props()

	const { timeToPixel, state } = getTimelineContext();
	const containerWidth = $derived($state.containerWidth);

	// Generate hour markers
	const hours = Array.from({ length: 24 }, (_, i) => i);

	// Helper to determine if hour should have darker line
	// function isMajorHour(hour: number): boolean {
	// 	return hour === 0 || hour === 6 || hour === 12 || hour === 18;
	// }
</script>

<div class="timeline-grid">
	<!-- Hour lines -->
	<!-- {#each hours as hour}
		{@const x = timeToPixel(new Date(2024, 0, 1, hour, 0, 0))}
		<div
			class="hour-line"
			class:major={isMajorHour(hour)}
			style="left: {x}px"
		></div>
	{/each} -->

	<!-- Hour columns with alternating backgrounds -->
	{#each hours as hour}
		{@const [year, month, day] = selectedDate.split('-').map(Number)}
		{@const startX = timeToPixel(createDateForHour(year, month, day, hour, userTimezone))}
		{@const endX = hour < 23 ? timeToPixel(createDateForHour(year, month, day, hour + 1, userTimezone)) : containerWidth}
		{@const isEven = hour % 2 === 0}
		<div
			class="hour-column"
			class:even={isEven}
			class:odd={!isEven}
			style="left: {startX}px; width: {endX - startX}px"
		></div>
	{/each}
</div>

<style>
	@reference "../../../../app.css";

	.timeline-grid {
		@apply absolute inset-0 pointer-events-none z-0;
	}

	.hour-column {
		@apply absolute top-0 bottom-0;
	}

	.hour-column.even {
		@apply bg-white;
	}

	.hour-column.odd {
		@apply bg-stone-100;
	}
</style>
