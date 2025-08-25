<script lang="ts">
	import { getTimelineContext } from "../timeline";

	export let transitions: Transition[] = [];
	export let height: number = 30;

	interface Transition {
		id: string;
		transitionTime: Date;
		transitionType: string; // 'changepoint' or 'data_gap'
		changeMagnitude?: number;
		changeDirection?: string; // 'increase' or 'decrease'
		beforeMean?: number;
		afterMean?: number;
		confidence: number;
		metadata?: any;
	}

	const { timeToPixel, pixelToTime, state } = getTimelineContext();

	// Track which transition is being hovered
	let hoveredTransitionId: string | null = null;

	// Format time for tooltip
	function formatTime(date: Date): string {
		return date.toLocaleTimeString("en-US", {
			hour: "numeric",
			minute: "2-digit",
			hour12: true,
		});
	}

	// Get opacity based on confidence
	function getOpacity(confidence: number): number {
		return 0.3 + confidence * 0.7; // Range from 0.3 to 1.0
	}

	// Check if cursor X position matches transition
	function isTransitionHighlighted(transition: Transition): boolean {
		if (!$state.mouseX) return false;
		const transitionX = timeToPixel(new Date(transition.transitionTime));
		const cursorX = $state.mouseX;
		// Highlight if cursor is within 2px of the transition
		return Math.abs(cursorX - transitionX) < 2;
	}
</script>

<div class="relative w-full" style="height: {height}px">
	<!-- Transition markers as simple vertical lines -->
	{#each transitions as transition}
		{@const x = timeToPixel(new Date(transition.transitionTime))}
		{@const opacity = getOpacity(transition.confidence)}
		{@const isHighlighted = isTransitionHighlighted(transition)}
		{@const isHovered = hoveredTransitionId === transition.id}

		<!-- Vertical line (solid for changepoints, dashed for data gaps) -->
		<div
			class="absolute top-0 bottom-0 w-0.5 transition-all duration-200"
			class:bg-blue-500={isHighlighted || isHovered}
			class:bg-neutral-600={!isHighlighted && !isHovered}
			class:w-1={isHighlighted || isHovered}
			class:border-dashed={transition.transitionType === 'data_gap'}
			class:border-l-2={transition.transitionType === 'data_gap'}
			style="
				left: {x}px;
				opacity: {opacity};
				transform: translateX(-50%);
				{transition.transitionType === 'data_gap' ? 'background: none;' : ''}
			"
			role="presentation"
			onmouseenter={() => (hoveredTransitionId = transition.id)}
			onmouseleave={() => (hoveredTransitionId = null)}
		></div>

		<!-- Tooltip (positioned separately for better visibility) -->
		{#if isHighlighted || isHovered}
			<div
				class="absolute px-2 py-1 bg-neutral-900 text-white text-xs rounded whitespace-nowrap z-10 pointer-events-none"
				style="
					left: {x}px;
					bottom: {height + 8}px;
					transform: translateX(-50%);
				"
			>
				<div class="font-medium">
					{formatTime(new Date(transition.transitionTime))}
				</div>
				{#if transition.transitionType === 'changepoint'}
					{#if transition.changeDirection && transition.changeMagnitude !== undefined}
						<div class="text-neutral-300">
							{transition.changeDirection === 'increase' ? '↑' : '↓'} 
							{transition.changeMagnitude.toFixed(1)} change
						</div>
					{/if}
				{:else if transition.transitionType === 'data_gap'}
					<div class="text-neutral-300">Data gap</div>
				{/if}
				<div class="text-neutral-400">
					{Math.round(transition.confidence * 100)}% confidence
				</div>
			</div>
		{/if}
	{/each}
</div>
