<script lang="ts">
	import { getTimelineContext } from '../timeline';

	interface Event {
		id: string;
		clusterId: number;
		startTime: Date;
		endTime: Date;
		coreDensity: number;
		clusterSize: number;
		signalContributions: Record<string, number>;
		eventType?: string;
		metadata: {
			duration_minutes: number;
			avg_confidence: number;
			type?: string;
			reason?: string;
		};
	}

	interface Props {
		events: Event[];
		transitions: Array<{
			id: string;
			transitionTime: Date;
			confidence: number;
			signalName: string;
		}>;
		selectedDate: string;
		userTimezone: string;
	}

	let { events = [], transitions = [], selectedDate, userTimezone }: Props = $props();

	const { timeToPixel, state } = getTimelineContext();

	// Format time for tooltip
	function formatTime(date: Date): string {
		return date.toLocaleTimeString('en-US', {
			hour: 'numeric',
			minute: '2-digit',
			hour12: true,
			timeZone: userTimezone,
		});
	}

	// Get dominant signal for an event
	function getDominantSignal(contributions: Record<string, number>): string {
		const entries = Object.entries(contributions);
		if (entries.length === 0) return 'Unknown';
		
		entries.sort((a, b) => b[1] - a[1]);
		return entries[0][0].replace(/_/g, ' ');
	}
	
	// Check if cursor is over an event
	function isCursorOverEvent(eventStart: Date, eventEnd: Date): boolean {
		const cursorX = state.mouseX;
		const startX = timeToPixel(eventStart);
		const endX = timeToPixel(eventEnd);
		
		// Check if cursor X is within event bounds
		return cursorX >= startX && cursorX <= endX;
	}
	
	// Compute which event is hovered based on cursor position
	let hoveredEventIndex = $derived((() => {
		if (state.isHovering || state.isCursorLocked) {
			// Find which event the cursor is over
			for (let i = 0; i < eventLayout.length; i++) {
				const { event } = eventLayout[i];
				if (isCursorOverEvent(event.startTime, event.endTime)) {
					return i;
				}
			}
		}
		return null;
	})());
	
	// Group overlapping events into rows
	function layoutEvents(events: Event[]): { event: Event; row: number }[] {
		const layout: { event: Event; row: number }[] = [];
		const rows: { end: number }[] = [];

		// Sort events by start time
		const sortedEvents = [...events].sort((a, b) => 
			a.startTime.getTime() - b.startTime.getTime()
		);

		// Get the maximum width (24 hours)
		const maxWidth = state.containerWidth || 1200;

		for (const event of sortedEvents) {
			const startPx = Math.max(0, timeToPixel(event.startTime));
			const endPx = Math.min(maxWidth, timeToPixel(event.endTime));

			// Skip events that are completely outside the visible range
			if (endPx <= 0 || startPx >= maxWidth) {
				continue;
			}

			// Find the first available row
			let row = 0;
			for (let i = 0; i < rows.length; i++) {
				if (rows[i].end <= startPx) {
					row = i;
					break;
				}
				row = i + 1;
			}

			// Update or add row
			if (row < rows.length) {
				rows[row].end = endPx;
			} else {
				rows.push({ end: endPx });
			}

			layout.push({ event, row });
		}

		return layout;
	}


	let eventLayout = $derived(layoutEvents(events));
	let totalRows = $derived(Math.max(1, ...eventLayout.map(e => e.row + 1)));
	let rowHeight = $derived(Math.min(30, 60 / totalRows));
</script>

<div class="relative" style="height: {totalRows * rowHeight + 10}px;">
	<!-- Detected Events -->
	{#each eventLayout as { event, row }, index}
		{@const maxWidth = state.containerWidth || 1200}
		{@const left = Math.max(0, timeToPixel(event.startTime))}
		{@const right = Math.min(maxWidth, timeToPixel(event.endTime))}
		{@const width = Math.max(2, right - left)}
		{@const showTooltip = hoveredEventIndex === index}
		{@const eventNumber = events.indexOf(event) + 1}
		
		<div
			class="event-box"
			class:cursor-over={showTooltip}
			style="
				left: {left}px;
				width: {width}px;
				top: {row * rowHeight + 5}px;
				height: {rowHeight - 4}px;
			"
		>
			{#if width > 30}
				<span class="event-title">
					Event {eventNumber}
				</span>
			{:else}
				<span class="event-title">
					{eventNumber}
				</span>
			{/if}
			
			<!-- Tooltip - shown based on cursor position -->
			<div class="event-tooltip" class:visible={showTooltip}>
				<div class="tooltip-title">Activity Event {eventNumber}</div>
				<div class="tooltip-time">
					{formatTime(event.startTime)} - {formatTime(event.endTime)}
				</div>
				<div class="tooltip-details">
					<div>Duration: {event.metadata?.duration_minutes ? event.metadata.duration_minutes.toFixed(0) : 'N/A'} min</div>
					<div>Transitions: {event.clusterSize}</div>
					<div>Confidence: {event.metadata?.avg_confidence ? (event.metadata.avg_confidence * 100).toFixed(0) : 'N/A'}%</div>
					<div>Density: {(event.coreDensity * 100).toFixed(0)}%</div>
				</div>
				<div class="tooltip-signal">
					Primary: {getDominantSignal(event.signalContributions)}
				</div>
				{#if state.isCursorLocked}
					<div class="tooltip-locked">🔒 Locked</div>
				{/if}
			</div>
		</div>
	{/each}
</div>

<style>
	.event-box {
		position: absolute;
		background: #9CA3AF; /* gray-400 */
		border: 1px solid #6B7280; /* gray-500 */
		border-radius: 3px;
		display: flex;
		align-items: center;
		padding: 0 4px;
		overflow: visible;
		cursor: pointer;
		transition: all 0.2s ease;
		z-index: 1;
	}
	
	.event-box:hover {
		background: #6B7280; /* gray-500 */
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		transform: translateY(-1px);
	}
	
	.event-box.cursor-over {
		background: #6B7280;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.event-title {
		color: white;
		font-size: 0.75rem;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.event-tooltip {
		position: absolute;
		bottom: 100%;
		left: 0;
		margin-bottom: 8px;
		background: #1F2937; /* gray-800 */
		color: white;
		padding: 8px 12px;
		border-radius: 6px;
		font-size: 0.75rem;
		white-space: nowrap;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.2s ease;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		z-index: 10;
		min-width: 200px;
	}

	.event-tooltip.visible {
		opacity: 1;
	}

	.tooltip-title {
		font-weight: 600;
		margin-bottom: 4px;
		color: #F3F4F6; /* gray-100 */
	}

	.tooltip-time {
		color: #D1D5DB; /* gray-300 */
		margin-bottom: 6px;
	}
	
	.tooltip-details {
		padding-top: 6px;
		border-top: 1px solid #374151; /* gray-700 */
		margin-bottom: 6px;
		color: #D1D5DB; /* gray-300 */
		line-height: 1.4;
	}
	
	.tooltip-signal {
		padding-top: 6px;
		border-top: 1px solid #374151; /* gray-700 */
		color: #D1D5DB; /* gray-300 */
	}

	.tooltip-locked {
		margin-top: 6px;
		padding-top: 6px;
		border-top: 1px solid #374151;
		color: #FCD34D; /* amber-300 */
		font-size: 0.7rem;
	}
</style>