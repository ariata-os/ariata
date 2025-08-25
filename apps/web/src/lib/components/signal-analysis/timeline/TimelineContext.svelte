<script context="module" lang="ts">
	import { getContext, setContext } from "svelte";
	import {
		writable,
		derived,
		type Writable,
		type Readable,
	} from "svelte/store";

	interface TimelineState {
		mouseX: number;
		isHovering: boolean;
		containerWidth: number;
		timeRange: number; // hours (default 24)
		pixelsPerMinute: number;
		hoveredTime: Date | null;
		isCursorLocked: boolean;
		lockedTime: Date | null;
	}

	const TIMELINE_KEY = Symbol("timeline");

	export function getTimelineContext(): {
		state: Writable<TimelineState>;
		hoveredTimeFormatted: Readable<string>;
		timeToPixel: (date: Date) => number;
		pixelToTime: (x: number, baseDate: Date) => Date;
	} {
		return getContext(TIMELINE_KEY);
	}
</script>

<script lang="ts">
	import { onMount } from "svelte";
	import { getHoursInTimezone, getMinutesInTimezone, pixelToUTCDate } from "$lib/utils/timezone";

	export let selectedDate: string; // YYYY-MM-DD format
	export let containerWidth = 1200;
	export let padding = 0;
	export let userTimezone: string;

	// Initialize timeline state
	const timeRange = 24; // Always show full day
	const minutesInRange = timeRange * 60;

	// Make pixelsPerMinute reactive
	$: pixelsPerMinute = (containerWidth - padding * 2) / minutesInRange;

	const state = writable<TimelineState>({
		mouseX: 0,
		isHovering: false,
		containerWidth,
		timeRange,
		pixelsPerMinute,
		hoveredTime: null,
		isCursorLocked: false,
		lockedTime: null,
	});

	// Update state when containerWidth or pixelsPerMinute changes
	$: {
		state.update((s) => ({
			...s,
			containerWidth,
			pixelsPerMinute,
		}));
	}

	// Derived store for formatted time
	const hoveredTimeFormatted = derived(state, ($state) => {
		const time = $state.isCursorLocked
			? $state.lockedTime
			: $state.hoveredTime;
		if (!time) return "";

		// Format as HH:MM using timezone
		const hours = getHoursInTimezone(time, userTimezone).toString().padStart(2, "0");
		const minutes = getMinutesInTimezone(time, userTimezone).toString().padStart(2, "0");
		return `${hours}:${minutes}`;
	});

	// Convert time to pixel position
	function timeToPixel(date: Date): number {
		// Use timezone-aware hours and minutes
		const hours = getHoursInTimezone(date, userTimezone);
		const minutes = getMinutesInTimezone(date, userTimezone);
		const totalMinutes = hours * 60 + minutes;
		// Use the reactive pixelsPerMinute value
		return totalMinutes * pixelsPerMinute;
	}

	// Convert pixel position to time
	function pixelToTime(x: number, baseDate: Date): Date {
		// Clamp x to valid range
		const clampedX = Math.max(
			0,
			Math.min(x, containerWidth),
		);
		const adjustedX = clampedX;

		// Calculate minutes from start of day
		// Use the reactive pixelsPerMinute value
		const totalMinutes = Math.round(adjustedX / pixelsPerMinute);

		// Use timezone utility to create proper UTC date
		return pixelToUTCDate(totalMinutes, baseDate, userTimezone);
	}

	// Handle mouse movement
	function handleMouseMove(event: MouseEvent) {
		if ($state.isCursorLocked) return;

		const rect = event.currentTarget.getBoundingClientRect();
		const x = event.clientX - rect.left;
		// Create date in local time, not UTC
		const [year, month, day] = selectedDate.split('-').map(Number);
		const baseDate = new Date(year, month - 1, day);

		state.update((s) => ({
			...s,
			mouseX: x,
			isHovering: true,
			hoveredTime: pixelToTime(x, baseDate),
		}));
	}

	// Handle mouse leave
	function handleMouseLeave() {
		if ($state.isCursorLocked) return;

		state.update((s) => ({
			...s,
			isHovering: false,
			hoveredTime: null,
		}));
	}

	// Handle click to lock/unlock cursor
	function handleClick(event: MouseEvent) {
		const rect = event.currentTarget.getBoundingClientRect();
		const x = event.clientX - rect.left;
		// Create date in local time, not UTC
		const [year, month, day] = selectedDate.split('-').map(Number);
		const baseDate = new Date(year, month - 1, day);
		const clickedTime = pixelToTime(x, baseDate);

		state.update((s) => {
			if (s.isCursorLocked) {
				// Unlock
				return {
					...s,
					isCursorLocked: false,
					lockedTime: null,
					mouseX: x,
					hoveredTime: clickedTime,
				};
			} else {
				// Lock
				return {
					...s,
					isCursorLocked: true,
					lockedTime: clickedTime,
					mouseX: x,
				};
			}
		});
	}

	// Handle keyboard navigation
	function handleKeyDown(event: KeyboardEvent) {
		if (!$state.isHovering && !$state.isCursorLocked) return;

		const currentTime = $state.isCursorLocked
			? $state.lockedTime
			: $state.hoveredTime;
		if (!currentTime) return;

		let newTime: Date | null = null;

		switch (event.key) {
			case "ArrowLeft":
				event.preventDefault();
				newTime = new Date(currentTime);
				newTime.setMinutes(currentTime.getMinutes() - 5);
				break;
			case "ArrowRight":
				event.preventDefault();
				newTime = new Date(currentTime);
				newTime.setMinutes(currentTime.getMinutes() + 5);
				break;
			case "Escape":
				if ($state.isCursorLocked) {
					event.preventDefault();
					state.update((s) => ({
						...s,
						isCursorLocked: false,
						lockedTime: null,
					}));
				}
				break;
		}

		if (newTime) {
			// Clamp to day boundaries
			// Create date in local time, not UTC
			const [year, month, day] = selectedDate.split('-').map(Number);
			const baseDate = new Date(year, month - 1, day);
			const startOfDay = new Date(baseDate);
			startOfDay.setHours(0, 0, 0, 0);
			const endOfDay = new Date(baseDate);
			endOfDay.setHours(23, 59, 59, 999);

			if (newTime >= startOfDay && newTime <= endOfDay) {
				const newX = timeToPixel(newTime);
				state.update((s) => ({
					...s,
					mouseX: newX,
					hoveredTime: $state.isCursorLocked
						? s.hoveredTime
						: newTime,
					lockedTime: $state.isCursorLocked ? newTime : s.lockedTime,
				}));
			}
		}
	}

	// Update container width on resize
	function updateContainerWidth() {
		const newPixelsPerMinute =
			(containerWidth - padding * 2) / minutesInRange;
		state.update((s) => ({
			...s,
			containerWidth,
			pixelsPerMinute: newPixelsPerMinute,
		}));
	}

	$: if (containerWidth) {
		updateContainerWidth();
	}

	// Set context
	setContext(TIMELINE_KEY, {
		state,
		hoveredTimeFormatted,
		timeToPixel,
		pixelToTime,
	});

	onMount(() => {
		// Add global keyboard listener
		window.addEventListener("keydown", handleKeyDown);

		return () => {
			window.removeEventListener("keydown", handleKeyDown);
		};
	});
</script>

<div
	class="timeline-context"
	on:mousemove={handleMouseMove}
	on:mouseleave={handleMouseLeave}
	on:click={handleClick}
	on:keydown={(e) => {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			handleClick(e);
		}
	}}
	role="application"
	aria-label="Timeline with interactive cursor"
	tabindex="0"
>
	<slot />
</div>

<style>
	.timeline-context {
		position: relative;
		width: 100%;
		height: 100%;
		cursor: crosshair;
	}

	:global(.timeline-context.cursor-locked) {
		cursor: pointer;
	}
</style>
