<script lang="ts">
	import { Badge } from "$lib/components";
	import { toast } from "svelte-sonner";
	import { slide } from "svelte/transition";
	import {
		getTimelineContext,
		TimelineGrid,
		TimelineCursor,
		TimelineLegend,
	} from "./timeline";
	import {
		ContinuousVisualization,
		BinaryVisualization,
		CategoricalVisualization,
		SpatialVisualization,
		TransitionVisualization,
		CountVisualization,
		EventVisualization,
	} from "./visualizations";
	import { parseDate, toZoned } from "@internationalized/date";

	interface TimelineEvent {
		id: string;
		startTime: Date;
		endTime: Date;
		summary?: string;
		confidence: number;
		source?: string;
		type?: "master" | "event" | "continuous" | "felt";
		metadata?: any;
	}

	interface RawSignal {
		timestamp: Date;
		value: number | string;
		label?: string;
		category?: string;
		coordinates?: any;
	}

	export let name: string;
	export let displayName: string;
	export let sourceName: string;
	export let streamName: string;
	export let signalName: string;
	export let company: "apple" | "google";
	export let type: "event" | "continuous";
	export let visualizationType:
		| "continuous"
		| "binary"
		| "categorical"
		| "spatial"
		| "event"
		| undefined;
	export let computedEvents: TimelineEvent[] = [];
	export let rawSignals: RawSignal[] = [];
	export let signalRange:
		| { min: number; max: number; unit: string }
		| undefined;
	export let showCursorOnExpandedHover: boolean = false;
	export let selectedDate: string | undefined = undefined;
	// Removed: onSignalAnalysisComplete - transitions now happen automatically
	export let transitions: any[] = [];
	export let hasTransitions: boolean = false;
	export let userTimezone: string = "America/Chicago";

	const { timeToPixel, state } = getTimelineContext();

	// Local hover state for this card
	let isHoveringContent = false;

	// View state
	let isExpanded = true; // Default to expanded

	// Get event width in pixels
	function getEventWidth(start: Date, end: Date): number {
		return Math.max(timeToPixel(end) - timeToPixel(start), 2);
	}

	// Format time for display
	function formatTime(date: Date): string {
		return date.toLocaleTimeString("en-US", {
			hour: "numeric",
			minute: "2-digit",
			hour12: true,
		});
	}

	// Removed: runSingleSignalTransitionDetection - transitions now happen automatically
</script>

<div
	class="bg-white border-y border-neutral-200 overflow-hidden transition-all duration-200"
>
	<!-- Signal Header -->
	<div
		class="flex items-center justify-between px-4 py-3 bg-neutral-100 border-neutral-200 min-h-[56px]"
	>
		<div class="flex items-center gap-3 flex-1">
			<button
				class="bg-transparent border-none p-2 rounded cursor-pointer text-neutral-500 transition-colors duration-200 hover:bg-neutral-200 hover:text-neutral-700"
				on:click={() => (isExpanded = !isExpanded)}
				aria-label={isExpanded ? "Collapse" : "Expand"}
			>
				<svg width="12" height="12" viewBox="0 0 12 12" fill="none">
					<path
						d={isExpanded ? "M2 4L6 8L10 4" : "M4 2L8 6L4 10"}
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					/>
				</svg>
			</button>

			<h3 class="text-sm font-normal font-mono text-neutral-900 m-0">
				{displayName}
			</h3>
		</div>

		<div class="flex gap-2 items-center">
			<Badge variant="default" size="sm">{sourceName}</Badge>
			<Badge variant="secondary" size="sm">{streamName}</Badge>
			<Badge variant="secondary" size="sm">{signalName}</Badge>
		</div>
	</div>

	<!-- Signal Content -->
	{#if isExpanded}
		<div
			class="relative"
			on:mouseenter={() => (isHoveringContent = true)}
			on:mouseleave={() => (isHoveringContent = false)}
			role="region"
			aria-label="Timeline content"
			transition:slide={{ duration: 200 }}
		>
			<!-- Single Timeline Grid overlay for entire expanded area -->
			<div class="absolute inset-0 pointer-events-none z-0">
				<TimelineGrid {selectedDate} userTimezone={userTimezone} />
			</div>

			<!-- Content wrapper with proper z-index -->
			<div class="relative z-10">
				<!-- Timeline Legend at the top -->
				<div class="h-8 pointer-events-none relative w-full mb-2">
					<TimelineLegend {selectedDate} {userTimezone} />
				</div>

				<!-- Transition Markers Section -->
				<div class="bg-transparent border-b border-neutral-200">
					<div
						class="flex items-center gap-2 px-4 h-8 border-b border-neutral-200"
					>
						<span
							class="text-[10px] font-semibold text-neutral-700 font-mono"
							>Transition Markers</span
						>
						<span class="text-[10px] text-neutral-500"
							>({transitions.length})</span
						>
					</div>

					{#if transitions.length > 0}
						<!-- Show transitions -->
						<div class="relative my-4">
							<TransitionVisualization
								{transitions}
								height={30}
							/>
						</div>
					{:else if computedEvents.length > 0}
						<!-- Show event data as markers -->
						<div class="relative h-12 my-4">
							{#each computedEvents as event}
								<div
									class="computed-event"
									style="
										left: {timeToPixel(event.startTime)}px;
										width: {getEventWidth(event.startTime, event.endTime)}px;
										background: {event.type === 'felt'
										? `rgba(34,197,94,${event.confidence * 0.2})`
										: 'linear-gradient(135deg, #F5F5F7 0%, #E8E8ED 100%)'};
										border-color: {event.type === 'felt'
										? `rgba(34,197,94,${event.confidence * 0.6})`
										: '#86868B'};
									"
									title="{event.summary} ({formatTime(
										event.startTime,
									)} - {formatTime(
										event.endTime,
									)}) - {Math.round(
										event.confidence * 100,
									)}% confidence"
								>
									{#if getEventWidth(event.startTime, event.endTime) > 60}
										<span
											class="text-[10px] font-mono text-neutral-700 whitespace-nowrap text-ellipsis overflow-hidden"
										>
											{event.summary}
										</span>
									{/if}
								</div>
							{/each}
						</div>
					{:else}
						<div
							class="flex flex-col items-center justify-center p-6 text-neutral-400 text-[13px] gap-2"
						>
							<p class="text-neutral-500">No transitions detected yet</p>
							<p class="text-xs text-neutral-400">Transitions are detected automatically after signal creation</p>
						</div>
					{/if}
				</div>

				<!-- Raw Data Section -->
				<div class="bg-transparent">
					<div
						class="flex items-center gap-2 px-4 h-8 border-b border-neutral-200"
					>
						<span
							class="text-[10px] font-semibold text-neutral-700 font-mono"
							>Raw Data</span
						>
						<span class="text-[10px] text-neutral-500"
							>({rawSignals.length})</span
						>
					</div>

					{#if rawSignals.length > 0}
						<div class="relative py-4 min-h-[60px]">
							{#if visualizationType === "continuous" && signalRange}
								<ContinuousVisualization
									signals={rawSignals}
									{signalRange}
								/>
							{:else if visualizationType === "binary"}
								<BinaryVisualization signals={rawSignals} />
							{:else if visualizationType === "categorical"}
								<CategoricalVisualization
									signals={rawSignals}
								/>
							{:else if visualizationType === "spatial"}
								<SpatialVisualization signals={rawSignals} />
							{:else if visualizationType === "count"}
								<CountVisualization signals={rawSignals} {signalRange} />
							{:else if visualizationType === "event"}
								<EventVisualization signals={rawSignals} />
							{:else}
								<!-- Fallback for unknown visualization types -->
								<div
									class="p-4 bg-neutral-100 rounded text-center text-neutral-500 text-xs"
								>
									<p>{rawSignals.length} data points</p>
								</div>
							{/if}
						</div>
					{:else}
						<div
							class="flex flex-col items-center justify-center p-6 text-neutral-400 text-[13px] gap-2"
						>
							<p>No raw data available</p>
						</div>
					{/if}
				</div>
			</div>

			<!-- Show timeline cursor only when hovering over expanded content -->
			{#if showCursorOnExpandedHover && (isHoveringContent || $state.isCursorLocked)}
				<TimelineCursor />
			{/if}
		</div>
	{/if}
</div>

<style>
	@reference "../../../app.css";

	.computed-event {
		@apply absolute top-2 h-8 border-1 rounded-sm px-1 flex items-center cursor-pointer transition-all duration-200 overflow-hidden;
	}
</style>
