<script lang="ts">
	import { Page } from '$lib';
	import { goto } from '$app/navigation';
	import 'iconify-icon';
	import type { PageData } from './$types';
	import type { TimelineBlock } from './+page';

	export let data: PageData;

	// Rome timezone
	const ROME_TIMEZONE = 'Europe/Rome';

	// Reactive state for date/time picker
	let selectedDate = data.selectedDate;
	let selectedHour = data.selectedHour;

	// Compute the 24-hour window start based on selected date + hour
	$: windowStart = new Date(`${selectedDate}T${selectedHour.toString().padStart(2, '0')}:00:00+01:00`);
	$: windowEnd = new Date(windowStart.getTime() + 24 * 60 * 60 * 1000);

	// Filter blocks that overlap with the 24-hour window
	$: filteredBlocks = data.timelineBlocks.filter((block) => {
		const blockStart = new Date(block.start_time);
		const blockEnd = block.end_time
			? new Date(block.end_time)
			: new Date(blockStart.getTime() + 30 * 60 * 1000);
		return blockStart < windowEnd && blockEnd > windowStart;
	});

	// Navigate when date/time changes
	function updateUrl() {
		goto(`/timeline?date=${selectedDate}&hour=${selectedHour}`, { replaceState: true });
	}

	// Get color for ontology
	function getOntologyColor(ontology: string): string {
		const colors: Record<string, string> = {
			location_visit: 'bg-blue-500',
			health_sleep: 'bg-purple-500',
			praxis_calendar: 'bg-green-500',
			praxis_message: 'bg-orange-500',
			praxis_email: 'bg-red-500',
			activity_app_usage: 'bg-amber-500'
		};
		return colors[ontology] || 'bg-neutral-500';
	}

	// Get lighter color variant for calendar blocks
	function getOntologyBlockColor(ontology: string): string {
		const colors: Record<string, string> = {
			location_visit: 'bg-blue-100 border-blue-300',
			health_sleep: 'bg-purple-100 border-purple-300',
			praxis_calendar: 'bg-green-100 border-green-300',
			praxis_message: 'bg-orange-100 border-orange-300',
			praxis_email: 'bg-red-100 border-red-300',
			activity_app_usage: 'bg-amber-100 border-amber-300'
		};
		return colors[ontology] || 'bg-neutral-100 border-neutral-300';
	}

	// Get text color for ontology
	function getOntologyTextColor(ontology: string): string {
		const colors: Record<string, string> = {
			location_visit: 'text-blue-800',
			health_sleep: 'text-purple-800',
			praxis_calendar: 'text-green-800',
			praxis_message: 'text-orange-800',
			praxis_email: 'text-red-800',
			activity_app_usage: 'text-amber-800'
		};
		return colors[ontology] || 'text-neutral-800';
	}

	// Format time only (HH:MM) in Rome timezone
	function formatTimeOnly(date: Date): string {
		return date.toLocaleString('en-US', {
			timeZone: ROME_TIMEZONE,
			hour: '2-digit',
			minute: '2-digit',
			hour12: false
		});
	}

	// Generate hour markers for the 24-hour window
	function getHourMarkers(): Array<{ position: number; label: string }> {
		const markers = [];
		for (let i = 0; i <= 24; i += 3) {
			const markerTime = new Date(windowStart.getTime() + i * 60 * 60 * 1000);
			markers.push({
				position: (i / 24) * 100,
				label: formatTimeOnly(markerTime)
			});
		}
		return markers;
	}

	// Calculate block position and width
	function getBlockStyle(block: TimelineBlock): { left: string; width: string } | null {
		const blockStart = new Date(block.start_time);
		const blockEnd = block.end_time
			? new Date(block.end_time)
			: new Date(blockStart.getTime() + 30 * 60 * 1000);

		// Clamp to window boundaries
		const clampedStart = new Date(Math.max(blockStart.getTime(), windowStart.getTime()));
		const clampedEnd = new Date(Math.min(blockEnd.getTime(), windowEnd.getTime()));

		if (clampedStart >= clampedEnd) return null;

		const startPercent = ((clampedStart.getTime() - windowStart.getTime()) / (24 * 60 * 60 * 1000)) * 100;
		const endPercent = ((clampedEnd.getTime() - windowStart.getTime()) / (24 * 60 * 60 * 1000)) * 100;
		const widthPercent = endPercent - startPercent;

		return {
			left: `${startPercent}%`,
			width: `${Math.max(widthPercent, 0.5)}%`
		};
	}

	// Format duration
	function formatDuration(block: TimelineBlock): string {
		const start = new Date(block.start_time);
		const end = block.end_time
			? new Date(block.end_time)
			: new Date(start.getTime() + 30 * 60 * 1000);
		const mins = Math.round((end.getTime() - start.getTime()) / (1000 * 60));
		if (mins < 60) return `${mins}m`;
		const hours = Math.floor(mins / 60);
		const remainingMins = mins % 60;
		return remainingMins > 0 ? `${hours}h ${remainingMins}m` : `${hours}h`;
	}

	// Pretty print ontology name
	function formatOntology(ontology: string): string {
		return ontology.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	// Format the window description
	$: windowDescription = windowStart.toLocaleDateString('en-US', {
		timeZone: ROME_TIMEZONE,
		weekday: 'long',
		month: 'long',
		day: 'numeric',
		year: 'numeric'
	}) + ' starting at ' + formatTimeOnly(windowStart);

	$: hourMarkers = getHourMarkers();
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Timeline</h1>
			<p class="text-neutral-600 mb-4">
				Narrative timeline pipeline visualization
			</p>

			<!-- Date/Time Picker -->
			<div class="flex items-center gap-4 mb-4">
				<div class="flex items-center gap-2">
					<label for="date" class="text-sm text-neutral-600">Date:</label>
					<input
						id="date"
						type="date"
						bind:value={selectedDate}
						on:change={updateUrl}
						class="px-3 py-1.5 border border-neutral-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
					/>
				</div>
				<div class="flex items-center gap-2">
					<label for="hour" class="text-sm text-neutral-600">Start hour:</label>
					<select
						id="hour"
						bind:value={selectedHour}
						on:change={updateUrl}
						class="px-3 py-1.5 border border-neutral-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
					>
						{#each Array(24) as _, i}
							<option value={i}>{i.toString().padStart(2, '0')}:00</option>
						{/each}
					</select>
				</div>
			</div>
		</div>

		{#if data.error}
			<!-- Error State -->
			<div class="border-2 border-red-200 rounded-lg p-8 text-center bg-red-50">
				<iconify-icon icon="ri:error-warning-line" class="text-4xl text-red-300 mb-2"></iconify-icon>
				<h3 class="text-lg font-medium text-neutral-900 mb-1">Error loading timeline data</h3>
				<p class="text-neutral-600">{data.error}</p>
			</div>
		{:else if filteredBlocks.length === 0}
			<!-- No Data State -->
			<div class="border-2 border-dashed border-neutral-200 rounded-lg p-8 text-center">
				<iconify-icon icon="ri:database-line" class="text-4xl text-neutral-300 mb-2"></iconify-icon>
				<h3 class="text-lg font-medium text-neutral-900 mb-1">No timeline data for this period</h3>
				<p class="text-neutral-600 mb-3">Try selecting a different date or run the seed script</p>
				<code class="text-sm text-neutral-600 px-3 py-2 rounded" style="background-color: #F3F2E9;">
					make db-reset && make seed
				</code>
			</div>
		{:else}
			<!-- Timeline View -->
			<div class="mb-12">
				<h2 class="text-xl font-serif font-medium text-neutral-800 mb-4">
					Changepoints
					<span class="ml-2 text-xs font-mono text-neutral-500 px-2 py-0.5 rounded" style="background-color: #F3F2E9;">event_boundaries</span>
				</h2>
				<p class="text-sm text-neutral-500 mb-4">
					{filteredBlocks.length} block{filteredBlocks.length !== 1 ? 's' : ''} in this window
				</p>

				<div class="relative bg-white border border-neutral-200 rounded-lg p-6">
					<!-- Hour grid -->
					<div class="relative h-20">
						<!-- Hour lines -->
						{#each hourMarkers as marker}
							<div
								class="absolute top-0 bottom-0 w-px bg-neutral-100"
								style="left: {marker.position}%;"
							></div>
						{/each}

						<!-- Timeline blocks -->
						{#each filteredBlocks as block}
							{@const style = getBlockStyle(block)}
							{#if style}
								<div
									class="absolute top-1 bottom-1 {getOntologyBlockColor(block.source_ontology)} border rounded cursor-pointer group hover:z-10 hover:shadow-md transition-shadow overflow-hidden"
									style="left: {style.left}; width: {style.width};"
								>
									<!-- Block content -->
									<div class="px-1 py-0.5 h-full flex flex-col justify-center min-w-0">
										<span class="text-[10px] font-medium {getOntologyTextColor(block.source_ontology)} truncate">
											{formatOntology(block.source_ontology)}
										</span>
									</div>

									<!-- Tooltip -->
									<div
										class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 bg-neutral-900 text-white text-xs rounded shadow-lg whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-20"
									>
										<div class="font-medium">{formatOntology(block.source_ontology)}</div>
										<div class="text-neutral-300">
											{formatTimeOnly(new Date(block.start_time))} - {block.end_time
												? formatTimeOnly(new Date(block.end_time))
												: 'ongoing'}
										</div>
										<div class="text-neutral-400">{formatDuration(block)}</div>
										{#if block.metadata && Object.keys(block.metadata).length > 0}
											<div class="text-neutral-400 mt-1 border-t border-neutral-700 pt-1">
												{#each Object.entries(block.metadata).slice(0, 2) as [key, value]}
													<div class="truncate max-w-48">{key}: {value}</div>
												{/each}
											</div>
										{/if}
										<div class="absolute top-full left-1/2 -translate-x-1/2 border-4 border-transparent border-t-neutral-900"></div>
									</div>
								</div>
							{/if}
						{/each}
					</div>

					<!-- Hour labels -->
					<div class="relative mt-2 h-4">
						{#each hourMarkers as marker}
							<div
								class="absolute text-[10px] text-neutral-400 -translate-x-1/2"
								style="left: {marker.position}%;"
							>
								{marker.label}
							</div>
						{/each}
					</div>
				</div>

				<!-- Legend -->
				<div class="flex flex-wrap gap-4 text-xs mt-6">
					<div class="flex items-center gap-2">
						<div class="w-4 h-4 bg-blue-100 border border-blue-300 rounded"></div>
						<span class="text-neutral-600">Location Visit</span>
					</div>
					<div class="flex items-center gap-2">
						<div class="w-4 h-4 bg-purple-100 border border-purple-300 rounded"></div>
						<span class="text-neutral-600">Sleep</span>
					</div>
					<div class="flex items-center gap-2">
						<div class="w-4 h-4 bg-green-100 border border-green-300 rounded"></div>
						<span class="text-neutral-600">Calendar</span>
					</div>
					<div class="flex items-center gap-2">
						<div class="w-4 h-4 bg-amber-100 border border-amber-300 rounded"></div>
						<span class="text-neutral-600">App Usage</span>
					</div>
					<div class="flex items-center gap-2">
						<div class="w-4 h-4 bg-orange-100 border border-orange-300 rounded"></div>
						<span class="text-neutral-600">Messages</span>
					</div>
				</div>
			</div>
		{/if}
	</div>
</Page>
