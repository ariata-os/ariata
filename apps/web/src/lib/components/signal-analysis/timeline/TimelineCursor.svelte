<script lang="ts">
	import { getTimelineContext } from "./TimelineContext.svelte";

	const { state, hoveredTimeFormatted } = getTimelineContext();

	$: showCursor = $state.isHovering || $state.isCursorLocked;
	$: cursorX = $state.mouseX;
	$: isLocked = $state.isCursorLocked;
</script>

{#if showCursor}
	<div
		class="timeline-cursor"
		class:locked={isLocked}
		style="left: {cursorX}px"
	>
		<!-- Time badge -->
		<div class="time-badge" class:locked={isLocked}>
			{$hoveredTimeFormatted}
			{#if isLocked}
				<span class="lock-indicator">🔒</span>
			{/if}
		</div>

		<!-- Cursor line -->
		<div class="cursor-line"></div>

		<!-- Click hint -->
		{#if !isLocked && $state.isHovering}
			<div class="click-hint">Click to lock</div>
		{/if}
	</div>
{/if}

<style>
	@reference "../../../../app.css";

	.timeline-cursor {
		@apply absolute top-0 bottom-0 pointer-events-none z-[100] transition-opacity duration-200;
	}

	.cursor-line {
		@apply w-px h-full bg-blue-500 opacity-80;
		box-shadow: 0 0 2px rgba(59, 130, 246, 0.5);
	}

	.timeline-cursor.locked .cursor-line {
		@apply bg-blue-800 opacity-100 w-0.5 -ml-px;
	}

	.time-badge {
		@apply absolute top-2 left-1/2 -translate-x-1/2 bg-neutral-800 text-white px-2.5 py-1 rounded text-xs font-mono whitespace-nowrap shadow-md flex items-center gap-1 transition-all duration-200 z-[1000];
	}

	.time-badge.locked {
		@apply bg-blue-800 px-3 font-semibold;
	}

	.lock-indicator {
		@apply text-[10px] opacity-90;
	}

	.click-hint {
		@apply absolute bottom-2.5 left-1/2 -translate-x-1/2 bg-neutral-800/90 text-white px-2 py-0.5 rounded-sm text-[10px] whitespace-nowrap opacity-0;
		animation: fadeIn 0.3s ease 0.5s forwards;
	}

	@keyframes fadeIn {
		to {
			opacity: 1;
		}
	}
</style>
