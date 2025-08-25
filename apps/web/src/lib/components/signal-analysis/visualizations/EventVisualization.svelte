<script lang="ts">
	import { getTimelineContext } from "../timeline";

	interface EventSignal {
		timestamp: Date;
		value: number | string;
		label?: string;
		duration?: number; // Duration in milliseconds
		metadata?: any;
	}

	export let signals: EventSignal[] = [];
	export let height: number = 60;

	const { timeToPixel, state } = getTimelineContext();
	
	// Track which event the cursor is over
	let hoveredEventIndex: number | null = null;
	
	// Check if cursor is over an event
	function isCursorOverEvent(eventStart: Date, eventEnd: Date): boolean {
		const cursorX = $state.mouseX;
		const startX = timeToPixel(eventStart);
		const endX = timeToPixel(eventEnd);
		
		// Check if cursor X is within event bounds
		return cursorX >= startX && cursorX <= endX;
	}
	
	// Update hovered event based on cursor position
	$: if ($state.isHovering || $state.isCursorLocked) {
		// Find which event the cursor is over
		const layoutedEvents = layoutEvents(signals);
		let foundIndex: number | null = null;
		
		for (let i = 0; i < layoutedEvents.length; i++) {
			const { signal } = layoutedEvents[i];
			const { start, end } = getEventTimes(signal);
			
			if (isCursorOverEvent(start, end)) {
				foundIndex = i;
				break;
			}
		}
		
		hoveredEventIndex = foundIndex;
	} else {
		hoveredEventIndex = null;
	}

	// Parse event metadata to get proper start/end times
	function getEventTimes(signal: EventSignal): { start: Date; end: Date; title: string } {
		let start = signal.timestamp;
		let end: Date;
		let title = signal.label || "Event";

		// Try to extract proper timing from metadata if available
		if (signal.metadata) {
			// Check for timing information in metadata
			if (signal.metadata.timing) {
				if (signal.metadata.timing.start) {
					start = new Date(signal.metadata.timing.start);
				}
				if (signal.metadata.timing.end) {
					end = new Date(signal.metadata.timing.end);
				} else if (signal.metadata.timing.duration_minutes) {
					end = new Date(start.getTime() + signal.metadata.timing.duration_minutes * 60 * 1000);
				}
			}
			
			// Check if it's an all-day event
			if (signal.metadata.event?.is_all_day) {
				// For all-day events, set end to 23:59:59 of the same day
				end = new Date(start);
				end.setHours(23, 59, 59, 999);
				title = `[All Day] ${title.replace('[All Day] ', '')}`;
			}
		}

		// Fallback: if no end time, use duration or default to 1 hour
		if (!end) {
			const duration = signal.duration || (60 * 60 * 1000); // Default 1 hour
			end = new Date(start.getTime() + duration);
		}

		return { start, end, title };
	}

	// Group overlapping events into rows
	function layoutEvents(signals: EventSignal[]): { signal: EventSignal; row: number }[] {
		const layout: { signal: EventSignal; row: number }[] = [];
		const rows: { end: number }[] = [];

		// Sort signals by start time
		const sortedSignals = [...signals].sort((a, b) => {
			const aStart = getEventTimes(a).start.getTime();
			const bStart = getEventTimes(b).start.getTime();
			return aStart - bStart;
		});

		for (const signal of sortedSignals) {
			const { start, end } = getEventTimes(signal);
			const startPx = timeToPixel(start);
			const endPx = timeToPixel(end);

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

			layout.push({ signal, row });
		}

		return layout;
	}

	// Detect gaps in the timeline
	function detectGaps(signals: EventSignal[]): { start: Date; end: Date }[] {
		// Get the first event to determine the day
		const firstEvent = signals.length > 0 ? getEventTimes(signals[0]) : null;
		
		// Create day boundaries based on the first event's date, or today if no events
		const referenceDate = firstEvent ? firstEvent.start : new Date();
		const dayStart = new Date(referenceDate);
		dayStart.setHours(0, 0, 0, 0);
		const dayEnd = new Date(referenceDate);
		dayEnd.setHours(23, 59, 59, 999);
		
		if (signals.length === 0) {
			// If no events, the whole day is unknown
			return [{ start: dayStart, end: dayEnd }];
		}
		
		const gaps: { start: Date; end: Date }[] = [];
		const sortedEvents = [...signals]
			.map(s => getEventTimes(s))
			.sort((a, b) => a.start.getTime() - b.start.getTime());
		
		// Check for gap at start of day
		const firstEventStart = sortedEvents[0].start.getTime();
		const dayStartTime = dayStart.getTime();
		
		if (firstEventStart > dayStartTime) {
			// Always show gap from start of day to first event
			gaps.push({ start: dayStart, end: sortedEvents[0].start });
		}
		
		// Check for gaps between events
		for (let i = 0; i < sortedEvents.length - 1; i++) {
			const currentEnd = sortedEvents[i].end;
			const nextStart = sortedEvents[i + 1].start;
			
			// Add any gap between events (no minimum time requirement for between events)
			if (nextStart.getTime() > currentEnd.getTime()) {
				gaps.push({ start: currentEnd, end: nextStart });
			}
		}
		
		// Check for gap at end of day
		const lastEventEnd = sortedEvents[sortedEvents.length - 1].end;
		const dayEndTime = dayEnd.getTime();
		
		if (lastEventEnd.getTime() < dayEndTime) {
			// Always show gap from last event to end of day
			gaps.push({ start: lastEventEnd, end: dayEnd });
		}
		
		return gaps;
	}

	$: eventLayout = layoutEvents(signals);
	$: totalRows = Math.max(1, ...eventLayout.map(e => e.row + 1));
	$: rowHeight = Math.min(30, height / totalRows);
	$: gaps = (() => {
		const detectedGaps = detectGaps(signals);
		console.log('Detected gaps:', detectedGaps.map(g => ({
			start: g.start.toLocaleTimeString(),
			end: g.end.toLocaleTimeString()
		})));
		console.log('Signals:', signals.length);
		return detectedGaps;
	})();
</script>

<div class="relative" style="height: {totalRows * rowHeight + 10}px;">
	<!-- Unknown/gap periods (render first so they're behind events) -->
	{#each gaps as gap}
		{@const left = timeToPixel(gap.start)}
		{@const rawWidth = timeToPixel(gap.end) - left}
		{@const gapSpacing = 2} <!-- 2px gap on each side -->
		{@const width = Math.max(2, rawWidth - gapSpacing * 2)}
		
		<div
			class="unknown-period"
			style="
				left: {left + gapSpacing}px;
				width: {width}px;
				top: 5px;
				height: {rowHeight - 4}px;
			"
			title="Unscheduled: {gap.start.toLocaleTimeString('en-US', {
				hour: 'numeric',
				minute: '2-digit',
				hour12: true
			})} - {gap.end.toLocaleTimeString('en-US', {
				hour: 'numeric',
				minute: '2-digit',
				hour12: true
			})}"
		>
			<!-- No content, just the dotted border -->
		</div>
	{/each}
	
	<!-- Events (render on top of gaps) -->
	{#each eventLayout as { signal, row }, index}
		{@const { start, end, title } = getEventTimes(signal)}
		{@const left = timeToPixel(start)}
		{@const width = Math.max(2, timeToPixel(end) - left)}
		{@const isAllDay = signal.metadata?.event?.is_all_day || title.includes('[All Day]')}
		{@const showTooltip = hoveredEventIndex === index}
		
		<div
			class="event-box"
			class:all-day={isAllDay}
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
					{title.replace('[All Day] ', '')}
				</span>
			{/if}
			
			<!-- Tooltip - shown based on cursor position -->
			<div class="event-tooltip" class:visible={showTooltip}>
				<div class="tooltip-title">{title.replace('[All Day] ', '')}</div>
				<div class="tooltip-time">
					{start.toLocaleTimeString('en-US', {
						hour: 'numeric',
						minute: '2-digit',
						hour12: true
					})} - {end.toLocaleTimeString('en-US', {
						hour: 'numeric',
						minute: '2-digit',
						hour12: true
					})}
				</div>
				{#if signal.metadata?.event?.location}
					<div class="tooltip-location">📍 {signal.metadata.event.location}</div>
				{/if}
				{#if $state.isCursorLocked}
					<div class="tooltip-locked">🔒 Locked</div>
				{/if}
			</div>
		</div>
	{/each}
</div>

<style>
	.unknown-period {
		position: absolute;
		background: rgba(229, 229, 229, 0.2); /* neutral-200 with transparency */
		border: 2px dotted #A1A1AA; /* neutral-400 - more visible */
		border-radius: 3px;
		pointer-events: none;
		z-index: 0;
	}

	.event-box {
		position: absolute;
		background: #D4D4D8; /* neutral-300 */
		border: 1px solid #A1A1AA; /* neutral-400 */
		border-radius: 3px;
		display: flex;
		align-items: center;
		padding: 0 4px;
		overflow: visible;
		cursor: pointer;
		transition: all 0.2s ease;
		z-index: 1; /* Above unknown periods */
	}

	.event-box.cursor-over {
		transform: translateY(-1px);
		border-color: #71717A; /* neutral-500 on hover */
		z-index: 100;
	}

	.event-box.all-day {
		background: #D4D4D8; /* neutral-300 - same as regular */
		border-color: #86EFAC; /* green-300 for distinction */
	}

	.event-title {
		font-size: 10px;
		font-weight: 500;
		color: #27272A; /* neutral-800 */
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		line-height: 1;
	}

	/* Tooltip styles */
	.event-tooltip {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
		background: rgba(31, 41, 55, 0.95);
		backdrop-filter: blur(4px);
		color: white;
		padding: 8px 12px;
		border-radius: 6px;
		white-space: nowrap;
		pointer-events: none;
		opacity: 0;
		visibility: hidden;
		transition: opacity 0.2s ease, visibility 0.2s ease;
		z-index: 1000;
		min-width: 150px;
		max-width: 300px;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
	}

	/* Arrow pointing down */
	.event-tooltip::after {
		content: '';
		position: absolute;
		top: 100%;
		left: 50%;
		transform: translateX(-50%);
		border: 6px solid transparent;
		border-top-color: rgba(31, 41, 55, 0.95);
	}

	.event-tooltip.visible {
		opacity: 1;
		visibility: visible;
	}

	.tooltip-title {
		font-size: 12px;
		font-weight: 600;
		margin-bottom: 4px;
		white-space: normal;
		word-wrap: break-word;
	}

	.tooltip-time {
		font-size: 11px;
		color: #D1D5DB;
		margin-bottom: 4px;
	}

	.tooltip-location {
		font-size: 11px;
		color: #9CA3AF;
		margin-top: 4px;
		white-space: normal;
		word-wrap: break-word;
	}

	.tooltip-locked {
		font-size: 10px;
		color: #60A5FA;
		margin-top: 4px;
		display: flex;
		align-items: center;
		gap: 4px;
	}

	/* Adjust tooltip position if it would go off-screen */
	@media (max-width: 640px) {
		.event-tooltip {
			left: 0;
			transform: translateX(0);
		}
	}
</style>