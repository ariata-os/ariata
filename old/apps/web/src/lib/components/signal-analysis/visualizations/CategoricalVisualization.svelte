<script lang="ts">
	import { getTimelineContext } from '../timeline';
	
	export let signals: Array<{
		timestamp: Date;
		value: string;
		label?: string;
		category?: string;
	}> = [];
	export let height = 64;
	export let gapThreshold = 2 * 60 * 1000; // 2 minutes in ms
	export let maxVisibleCategories = 6; // Maximum categories before scrolling
	
	const { state, timeToPixel } = getTimelineContext();
	
	// Refs for scroll synchronization
	let labelsContainer: HTMLDivElement;
	let timelineContainer: HTMLDivElement;
	
	// Synchronize scroll between labels and timeline
	function syncScroll(source: 'labels' | 'timeline') {
		if (source === 'labels' && labelsContainer && timelineContainer) {
			timelineContainer.scrollTop = labelsContainer.scrollTop;
		} else if (source === 'timeline' && labelsContainer && timelineContainer) {
			labelsContainer.scrollTop = timelineContainer.scrollTop;
		}
	}
	
	// Get unique categories
	function getUniqueCategories(signals: typeof signals): string[] {
		const categories = new Set<string>();
		signals.forEach(signal => {
			categories.add(signal.category || signal.value || 'Unknown');
		});
		return Array.from(categories).sort();
	}
	
	// Generate consistent color from string
	function stringToColor(str: string): string {
		if (!str) return 'hsl(0, 0%, 50%)';
		const hash = str.split('').reduce((acc, char) => 
			char.charCodeAt(0) + ((acc << 5) - acc), 0
		);
		return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`;
	}
	
	// Group categorical signals into continuous events
	function categorizeContinuousEvents(signals: typeof signals) {
		if (!signals || signals.length === 0) return [];
		
		const sorted = [...signals].sort((a, b) => 
			a.timestamp.getTime() - b.timestamp.getTime()
		);
		
		const events = [];
		let currentEvent = {
			startTime: sorted[0].timestamp,
			endTime: sorted[0].timestamp,
			category: sorted[0].category || sorted[0].value || 'Unknown',
			count: 1
		};
		
		for (let i = 1; i < sorted.length; i++) {
			const signal = sorted[i];
			const category = signal.category || signal.value || 'Unknown';
			const gap = signal.timestamp.getTime() - currentEvent.endTime.getTime();
			
			if (gap <= gapThreshold && category === currentEvent.category) {
				// Extend current event
				currentEvent.endTime = signal.timestamp;
				currentEvent.count++;
			} else {
				// Start new event
				if (currentEvent.endTime.getTime() - currentEvent.startTime.getTime() === 0) {
					// Single point, extend by 1 minute for visibility
					currentEvent.endTime = new Date(currentEvent.endTime.getTime() + 60 * 1000);
				}
				events.push(currentEvent);
				currentEvent = {
					startTime: signal.timestamp,
					endTime: signal.timestamp,
					category: category,
					count: 1
				};
			}
		}
		
		// Add the last event
		if (currentEvent.endTime.getTime() - currentEvent.startTime.getTime() === 0) {
			currentEvent.endTime = new Date(currentEvent.endTime.getTime() + 60 * 1000);
		}
		events.push(currentEvent);
		
		return events;
	}
	
	$: categories = getUniqueCategories(signals);
	$: needsScroll = categories.length > maxVisibleCategories;
	$: visibleHeight = needsScroll ? height : height;
	$: laneHeight = needsScroll 
		? Math.max(12, (height - 8) / maxVisibleCategories)
		: Math.max(12, (height - 8) / Math.max(categories.length, 1));
	$: totalHeight = needsScroll ? laneHeight * categories.length + 8 : height;
	$: categoryToLane = Object.fromEntries(categories.map((cat, i) => [cat, i]));
	$: categorizedEvents = categorizeContinuousEvents(signals);
	
	// Find event at cursor position
	$: hoveredEvent = (() => {
		if (!$state.isHovering && !$state.isCursorLocked) return null;
		
		const targetTime = $state.isCursorLocked ? $state.lockedTime : $state.hoveredTime;
		if (!targetTime) return null;
		
		return categorizedEvents.find(event => 
			targetTime >= event.startTime && targetTime <= event.endTime
		);
	})();
</script>

<div class="categorical-viz" style="height: {height}px">
	<!-- Category labels -->
	<div 
		class="category-labels" 
		style="width: 100px; height: {visibleHeight}px; {needsScroll ? 'overflow-y: auto;' : ''}"
		bind:this={labelsContainer}
		on:scroll={() => syncScroll('labels')}
	>
		<div class="labels-inner" style="height: {totalHeight}px">
			{#each categories as category, i}
				<div 
					class="category-label"
					style="height: {laneHeight}px; background-color: {i % 2 === 0 ? 'transparent' : 'rgba(0,0,0,0.02)'}"
				>
					<div 
						class="category-dot"
						style="background-color: {stringToColor(category)}"
					></div>
					<span class="category-text" title={category}>
						{category.length > 12 ? category.substring(0, 12) + '...' : category}
					</span>
				</div>
			{/each}
		</div>
	</div>
	
	<!-- Timeline visualization -->
	<div 
		class="timeline-area" 
		style="left: 100px; height: {visibleHeight}px; {needsScroll ? 'overflow-y: auto;' : ''}"
		bind:this={timelineContainer}
		on:scroll={() => syncScroll('timeline')}
	>
		<svg class="viz-svg" style="height: {totalHeight}px">
			<!-- Lane backgrounds -->
			{#each categories as category, i}
				<rect
					x="0"
					y={i * laneHeight}
					width="100%"
					height={laneHeight}
					fill={i % 2 === 0 ? 'transparent' : 'rgba(0,0,0,0.02)'}
				/>
			{/each}
			
			<!-- Category events -->
			{#each categorizedEvents as event}
				{@const laneIndex = categoryToLane[event.category] || 0}
				{@const x = timeToPixel(event.startTime)}
				{@const width = Math.max(
					timeToPixel(event.endTime) - x,
					2
				)}
				{@const isHovered = hoveredEvent === event}
				<rect
					x={x}
					y={laneIndex * laneHeight + 2}
					{width}
					height={laneHeight - 4}
					fill={stringToColor(event.category)}
					opacity={isHovered ? 1 : 0.7}
					rx="2"
					class="category-event"
					class:hovered={isHovered}
				/>
			{/each}
		</svg>
	</div>
	
	<!-- Hover tooltip -->
	{#if hoveredEvent}
		{@const x = timeToPixel(hoveredEvent.startTime) + 
			(timeToPixel(hoveredEvent.endTime) - timeToPixel(hoveredEvent.startTime)) / 2}
		{@const y = (categoryToLane[hoveredEvent.category] || 0) * laneHeight + laneHeight / 2}
		<div 
			class="hover-tooltip"
			style="left: {x + 100}px; top: {y}px"
		>
			<div class="tooltip-category">{hoveredEvent.category}</div>
			<div class="tooltip-time">
				{hoveredEvent.startTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})} - {hoveredEvent.endTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})}
			</div>
			<div class="tooltip-count">{hoveredEvent.count} events</div>
		</div>
	{/if}
</div>

<style>
	.categorical-viz {
		position: relative;
		width: 100%;
		display: flex;
	}
	
	.category-labels {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		background: white;
		border-right: 1px solid #e5e7eb;
		z-index: 10;
		scrollbar-width: thin;
		scrollbar-color: #cbd5e1 #f3f4f6;
	}
	
	.category-labels::-webkit-scrollbar {
		width: 6px;
	}
	
	.category-labels::-webkit-scrollbar-track {
		background: #f3f4f6;
	}
	
	.category-labels::-webkit-scrollbar-thumb {
		background: #cbd5e1;
		border-radius: 3px;
	}
	
	.category-labels::-webkit-scrollbar-thumb:hover {
		background: #94a3b8;
	}
	
	.labels-inner {
		position: relative;
	}
	
	.category-label {
		display: flex;
		align-items: center;
		padding: 0 4px;
		font-size: 10px;
		color: #6b7280;
	}
	
	.category-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		margin-right: 4px;
		flex-shrink: 0;
	}
	
	.category-text {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.timeline-area {
		position: absolute;
		right: 0;
		top: 0;
		bottom: 0;
		scrollbar-width: thin;
		scrollbar-color: #cbd5e1 #f3f4f6;
	}
	
	.timeline-area::-webkit-scrollbar {
		width: 6px;
		height: 6px;
	}
	
	.timeline-area::-webkit-scrollbar-track {
		background: #f3f4f6;
	}
	
	.timeline-area::-webkit-scrollbar-thumb {
		background: #cbd5e1;
		border-radius: 3px;
	}
	
	.timeline-area::-webkit-scrollbar-thumb:hover {
		background: #94a3b8;
	}
	
	.viz-svg {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		overflow: visible;
	}
	
	.category-event {
		cursor: pointer;
		transition: all 0.2s ease;
	}
	
	.category-event.hovered {
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
	}
	
	.hover-tooltip {
		position: absolute;
		transform: translate(-50%, -50%);
		background: rgba(30, 41, 59, 0.95);
		color: white;
		padding: 6px 10px;
		border-radius: 4px;
		font-size: 11px;
		white-space: nowrap;
		pointer-events: none;
		z-index: 200;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
	}
	
	.tooltip-category {
		font-weight: 600;
		margin-bottom: 2px;
	}
	
	.tooltip-time {
		font-size: 10px;
		opacity: 0.9;
		font-family: monospace;
	}
	
	.tooltip-count {
		font-size: 10px;
		opacity: 0.7;
		margin-top: 2px;
	}
</style>