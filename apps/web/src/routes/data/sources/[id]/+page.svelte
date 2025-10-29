<script lang="ts">
	import { goto } from "$app/navigation";
	import { Button, Page } from "$lib";
	import {
		pauseSource,
		resumeSource,
		deleteSource,
		syncStream,
		enableStream,
		getJobStatus,
		type Job,
	} from "$lib/api/client";
	import {
		getSourceTypeIcon,
		getSourceTypeColor,
	} from "$lib/mock-data/connections";
	import "iconify-icon";
	import type { PageData } from "./$types";
	import { onDestroy } from "svelte";

	let { data }: { data: PageData } = $props();

	let isDeleting = $state(false);
	let isPausing = $state(false);
	let syncingStreams = $state(new Map<string, string>()); // stream_name -> job_id
	let enablingStreams = $state(new Set<string>());
	let pollingIntervals = new Map<string, number>(); // job_id -> interval_id

	// Determine if this source is OAuth-based
	const isOAuthSource = $derived(() => {
		if (!data.catalog || !data.source) return false;
		const catalogSource = data.catalog.find((c: any) => c.name === data.source.type);
		return catalogSource?.auth_type === 'oauth2';
	});

	function formatRelativeTime(timestamp: string | null): string {
		if (!timestamp) return "Never";

		const date = new Date(timestamp);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 1000 / 60);
		const diffHours = Math.floor(diffMins / 60);
		const diffDays = Math.floor(diffHours / 24);

		if (diffMins < 1) return "Just now";
		if (diffMins < 60) return `${diffMins}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		if (diffDays < 7) return `${diffDays}d ago`;

		return date.toLocaleDateString();
	}

	async function handleTogglePause() {
		if (isPausing) return;
		isPausing = true;

		try {
			if (data.source.is_active) {
				const updated = await pauseSource(data.source.id);
				data.source = updated;
			} else {
				const updated = await resumeSource(data.source.id);
				data.source = updated;
			}
		} catch (error) {
			console.error("Failed to toggle source:", error);
			alert("Failed to toggle source. Please try again.");
		} finally {
			isPausing = false;
		}
	}

	async function handleDelete() {
		if (isDeleting) return;

		const confirmed = confirm(
			`Are you sure you want to delete "${data.source.name}"? This will permanently delete the source, all its streams, and all synced data. This action cannot be undone.`
		);
		if (!confirmed) return;

		isDeleting = true;

		try {
			await deleteSource(data.source.id);
			goto("/data/sources");
		} catch (error) {
			console.error("Failed to delete source:", error);
			alert("Failed to delete source. Please try again.");
			isDeleting = false;
		}
	}

	async function handleSyncStream(streamName: string) {
		if (syncingStreams.has(streamName)) return;

		try {
			// Trigger the sync job (returns immediately with job_id)
			const response = await syncStream(data.source.id, streamName);

			// Track the job
			syncingStreams.set(streamName, response.job_id);

			// Start polling for job status
			startPollingJob(response.job_id, streamName);
		} catch (error) {
			console.error("Failed to start sync:", error);
			alert(`Failed to start sync: ${error instanceof Error ? error.message : 'Unknown error'}`);
		}
	}

	function startPollingJob(jobId: string, streamName: string) {
		// Poll every 2 seconds
		const intervalId = setInterval(async () => {
			try {
				const job = await getJobStatus(jobId);

				// Check if job is complete
				if (job.status === 'succeeded' || job.status === 'failed' || job.status === 'cancelled') {
					// Stop polling
					stopPollingJob(jobId, streamName);

					// Update stream data
					const streamIndex = data.streams.findIndex((s: any) => s.stream_name === streamName);
					if (streamIndex !== -1 && job.status === 'succeeded') {
						// Refresh the page data to get updated last_sync_at
						data.streams[streamIndex].last_sync_at = job.completed_at || new Date().toISOString();
					}

					// Show result
					if (job.status === 'succeeded') {
						alert(`Sync completed! Processed ${job.records_processed} records.`);
					} else if (job.status === 'failed') {
						alert(`Sync failed: ${job.error_message || 'Unknown error'}`);
					} else if (job.status === 'cancelled') {
						alert('Sync was cancelled.');
					}
				}
			} catch (error) {
				console.error('Failed to poll job status:', error);
				stopPollingJob(jobId, streamName);
				alert(`Error checking sync status: ${error instanceof Error ? error.message : 'Unknown error'}`);
			}
		}, 2000) as any;

		pollingIntervals.set(jobId, intervalId);
	}

	function stopPollingJob(jobId: string, streamName: string) {
		// Clear interval
		const intervalId = pollingIntervals.get(jobId);
		if (intervalId) {
			clearInterval(intervalId);
			pollingIntervals.delete(jobId);
		}

		// Remove from syncing streams
		syncingStreams.delete(streamName);
		syncingStreams = new Map(syncingStreams); // Trigger reactivity
	}

	// Cleanup on component destroy
	onDestroy(() => {
		// Clear all polling intervals
		for (const intervalId of pollingIntervals.values()) {
			clearInterval(intervalId);
		}
		pollingIntervals.clear();
	});

	async function handleEnableStream(streamName: string) {
		if (enablingStreams.has(streamName)) return;

		enablingStreams = new Set([...enablingStreams, streamName]);

		try {
			await enableStream(data.source.id, streamName);

			// Update the stream's is_enabled in the data
			const streamIndex = data.streams.findIndex((s: any) => s.stream_name === streamName);
			if (streamIndex !== -1) {
				data.streams[streamIndex].is_enabled = true;
			}

			alert('Stream enabled successfully!');
		} catch (error) {
			console.error("Failed to enable stream:", error);
			alert(`Failed to enable stream: ${error instanceof Error ? error.message : 'Unknown error'}`);
		} finally {
			const newSet = new Set(enablingStreams);
			newSet.delete(streamName);
			enablingStreams = newSet;
		}
	}
</script>

{#if !data.source}
	<Page>
		<div class="text-center py-12">
			<h1 class="text-2xl font-serif font-medium text-neutral-900 mb-2">Source not found</h1>
			<p class="text-neutral-600 mb-4">The source you're looking for doesn't exist.</p>
			<a
				href="/data/sources"
				class="inline-flex items-center gap-2 text-neutral-900 hover:underline"
			>
				<iconify-icon icon="ri:arrow-left-line"></iconify-icon>
				Back to Sources
			</a>
		</div>
	</Page>
{:else}
	<Page>
		<div class="max-w-7xl">
			<!-- Header -->
			<div class="mb-8">
				<div class="flex items-start justify-between">
					<div class="flex items-center gap-4">
						<iconify-icon
							icon={getSourceTypeIcon(data.source.type)}
							class="text-5xl {getSourceTypeColor(data.source.type)}"
						></iconify-icon>
						<div>
							<div class="flex items-center gap-3 mb-1">
								<h1 class="text-3xl font-serif font-medium text-neutral-900">
									{data.source.name}
								</h1>
								{#if data.source.is_active}
									<span
										class="inline-block px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-full"
									>
										Active
									</span>
								{:else}
									<span
										class="inline-block px-2 py-1 text-xs font-medium bg-neutral-100 text-neutral-500 rounded-full"
									>
										Paused
									</span>
								{/if}
							</div>
							<p class="text-neutral-600 capitalize">{data.source.type} Source</p>
						</div>
					</div>

					<div class="flex items-center gap-2">
						<Button
							variant="ghost"
							class="flex items-center gap-2 border border-neutral-200"
							onclick={handleTogglePause}
							disabled={isPausing}
						>
							<iconify-icon
								icon={data.source.is_active ? "ri:pause-line" : "ri:play-line"}
							></iconify-icon>
							<span>{data.source.is_active ? "Pause" : "Resume"}</span>
						</Button>
						<Button
							variant="danger"
							class="flex items-center gap-2"
							onclick={handleDelete}
							disabled={isDeleting}
						>
							<iconify-icon icon="ri:delete-bin-line"></iconify-icon>
							<span>Delete</span>
						</Button>
					</div>
				</div>
			</div>

			<!-- Key Attributes -->
			<div class="mb-8 text-neutral-600 space-y-1">
				<p>Created {formatRelativeTime(data.source.created_at).toLowerCase()}</p>
				<p>Last synced {formatRelativeTime(data.source.last_sync_at).toLowerCase()}</p>
				<p>{data.source.enabled_streams_count} of {data.source.total_streams_count} streams enabled</p>
			</div>

			<!-- Streams Section -->
			<div class="mb-8">
				<h2 class="text-xl font-serif font-medium text-neutral-900 mb-4">Streams</h2>

				<div class="bg-white border border-neutral-200 rounded-lg overflow-hidden">
					<table class="w-full">
						<thead class="bg-neutral-50 border-b border-neutral-200">
							<tr>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Stream
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Status
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Last Sync
								</th>
								{#if isOAuthSource()}
									<th class="px-6 py-3 text-right text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Actions
									</th>
								{/if}
							</tr>
						</thead>
						<tbody class="divide-y divide-neutral-200">
							{#each data.streams as stream}
								<tr class="hover:bg-neutral-50 transition-colors">
									<td class="px-6 py-4">
										<div>
											<div class="font-medium text-neutral-900">{stream.display_name}</div>
											<div class="text-sm text-neutral-500">{stream.description}</div>
										</div>
									</td>
									<td class="px-6 py-4">
										{#if stream.is_enabled}
											<span class="inline-block px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-full">
												Enabled
											</span>
										{:else}
											<span class="inline-block px-2 py-1 text-xs font-medium bg-neutral-100 text-neutral-500 rounded-full">
												Disabled
											</span>
										{/if}
									</td>
									<td class="px-6 py-4">
										<span class="text-sm text-neutral-900">
											{formatRelativeTime(stream.last_sync_at)}
										</span>
									</td>
									{#if isOAuthSource()}
										<td class="px-6 py-4 text-right">
											{#if stream.is_enabled}
												<button
													onclick={() => handleSyncStream(stream.stream_name)}
													disabled={syncingStreams.has(stream.stream_name)}
													class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-neutral-700 bg-white border border-neutral-300 rounded-md hover:bg-neutral-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-neutral-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
												>
													{#if syncingStreams.has(stream.stream_name)}
														<iconify-icon icon="ri:loader-4-line" class="animate-spin"></iconify-icon>
														<span>Syncing...</span>
													{:else}
														<iconify-icon icon="ri:refresh-line"></iconify-icon>
														<span>Sync Now</span>
													{/if}
												</button>
											{:else}
												<button
													onclick={() => handleEnableStream(stream.stream_name)}
													disabled={enablingStreams.has(stream.stream_name)}
													class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-neutral-700 bg-white border border-neutral-300 rounded-md hover:bg-neutral-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-neutral-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
												>
													{#if enablingStreams.has(stream.stream_name)}
														<iconify-icon icon="ri:loader-4-line" class="animate-spin"></iconify-icon>
														<span>Enabling...</span>
													{:else}
														<iconify-icon icon="ri:play-line"></iconify-icon>
														<span>Enable</span>
													{/if}
												</button>
											{/if}
										</td>
									{/if}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>

		</div>
	</Page>
{/if}
