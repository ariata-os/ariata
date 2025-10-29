<script lang="ts">
	import { Page, Button } from "$lib";
	import type { Job } from "$lib/api/client";
	import { goto, invalidateAll } from "$app/navigation";
	import { cancelJob } from "$lib/api/client";
	import "iconify-icon";
	import type { PageData } from "./$types";

	export let data: PageData;

	$: jobs = data.jobs;

	function formatRelativeTime(timestamp: string): string {
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

	function getStatusBadgeClass(status: Job["status"]): string {
		const classes: Record<Job["status"], string> = {
			succeeded: "bg-green-50 text-green-700 border border-green-200",
			failed: "bg-red-50 text-red-700 border border-red-200",
			cancelled: "bg-amber-50 text-amber-700 border border-amber-200",
			running: "bg-blue-50 text-blue-700 border border-blue-200",
			pending: "bg-neutral-50 text-neutral-600 border border-neutral-200",
		};
		return classes[status] || "bg-neutral-50 text-neutral-600 border border-neutral-200";
	}

	function getJobTypeLabel(jobType: Job["job_type"], streamName?: string): string {
		const baseLabel = jobType === "sync" ? "Sync" : "Transform";
		return streamName ? `${baseLabel} · ${streamName}` : baseLabel;
	}

	function calculateDuration(startedAt: string, completedAt?: string): string {
		const start = new Date(startedAt).getTime();
		const end = completedAt ? new Date(completedAt).getTime() : Date.now();
		const durationMs = end - start;

		const seconds = Math.floor(durationMs / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);

		if (hours > 0) return `${hours}h ${minutes % 60}m`;
		if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
		return `${seconds}s`;
	}

	async function handleCancelJob(jobId: string) {
		if (!confirm("Are you sure you want to cancel this job?")) return;

		try {
			await cancelJob(jobId);
			await invalidateAll();
		} catch (err) {
			alert(err instanceof Error ? err.message : "Failed to cancel job");
		}
	}

	async function handleRefresh() {
		await invalidateAll();
	}

	$: hasActiveJobs = jobs.some((j) => j.status === "pending" || j.status === "running");
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class="mb-8 flex items-center justify-between">
			<div>
				<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Activity</h1>
				<p class="text-neutral-600">
					System activity including syncs, transformations, and configuration changes
				</p>
			</div>
			<Button onclick={handleRefresh} variant="secondary">
				<iconify-icon icon="ri:refresh-line" class="text-lg"></iconify-icon>
				Refresh
			</Button>
		</div>

		<!-- Activity Table -->
		{#if jobs.length === 0}
			<div
				class="border-2 border-dashed border-neutral-200 rounded-lg p-12 text-center bg-neutral-50"
			>
				<iconify-icon icon="ri:history-line" class="text-6xl text-neutral-300 mb-4"
				></iconify-icon>
				<h3 class="text-lg font-medium text-neutral-900 mb-2">No activity yet</h3>
				<p class="text-neutral-600">
					System activity will appear here once your sources start syncing
				</p>
			</div>
		{:else}
			<div class="bg-white border border-neutral-200 rounded-lg overflow-hidden">
				<table class="w-full">
					<thead class="bg-neutral-50 border-b border-neutral-200">
						<tr>
							<th class="px-6 py-4 text-left text-xs font-medium text-neutral-500 uppercase">
								Type
							</th>
							<th class="px-6 py-4 text-left text-xs font-medium text-neutral-500 uppercase">
								Source
							</th>
							<th class="px-6 py-4 text-left text-xs font-medium text-neutral-500 uppercase">
								Status
							</th>
							<th class="px-6 py-4 text-right text-xs font-medium text-neutral-500 uppercase">
								Duration
							</th>
							<th class="px-6 py-4 text-right text-xs font-medium text-neutral-500 uppercase">
								Records
							</th>
							<th class="px-6 py-4 text-right text-xs font-medium text-neutral-500 uppercase">
								Time
							</th>
							<th class="px-6 py-4 text-right text-xs font-medium text-neutral-500 uppercase">
								Actions
							</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-200">
						{#each jobs as job}
							<tr class="hover:bg-neutral-50 transition-colors">
								<!-- Type -->
								<td class="px-6 py-4 whitespace-nowrap">
									<span class="text-sm text-neutral-900">
										{getJobTypeLabel(job.job_type, job.stream_name || undefined)}
									</span>
								</td>

								<!-- Source -->
								<td class="px-6 py-4 whitespace-nowrap">
									{#if job.source_id}
										<a
											href="/data/sources/{job.source_id}"
											class="text-sm text-neutral-700 hover:text-neutral-900 hover:underline"
										>
											{job.source_id}
										</a>
									{:else}
										<span class="text-sm text-neutral-400">—</span>
									{/if}
								</td>

								<!-- Status -->
								<td class="px-6 py-4 whitespace-nowrap">
									<span
										class="inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded-full capitalize {getStatusBadgeClass(
											job.status
										)}"
									>
										{#if job.status === "failed" && job.error_message}
											<iconify-icon icon="ri:error-warning-line" class="text-xs"
											></iconify-icon>
										{/if}
										{#if job.status === "running"}
											<iconify-icon icon="ri:loader-4-line" class="text-xs animate-spin"
											></iconify-icon>
										{/if}
										{job.status.replace("_", " ")}
									</span>
								</td>

								<!-- Duration -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									<span class="text-sm text-neutral-600">
										{calculateDuration(job.started_at, job.completed_at)}
									</span>
								</td>

								<!-- Records -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									{#if job.records_processed > 0}
										<span class="text-sm text-neutral-700">
											{job.records_processed.toLocaleString()}
										</span>
									{:else}
										<span class="text-sm text-neutral-300">—</span>
									{/if}
								</td>

								<!-- Time -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									<span class="text-sm text-neutral-600">
										{formatRelativeTime(job.started_at)}
									</span>
								</td>

								<!-- Actions -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									<div class="flex items-center justify-end gap-2">
										{#if job.status === "pending" || job.status === "running"}
											<button
												on:click={() => handleCancelJob(job.id)}
												class="text-xs text-red-600 hover:text-red-700 hover:underline"
											>
												Cancel
											</button>
										{/if}
										{#if job.status === "failed" && job.error_message}
											<button
												on:click={() => alert(`Error: ${job.error_message}`)}
												class="text-xs text-neutral-600 hover:text-neutral-700 hover:underline"
												title={job.error_message}
											>
												View Error
											</button>
										{/if}
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Summary -->
			<div class="mt-4 flex items-center justify-between">
				<div class="text-sm text-neutral-600">
					Showing {jobs.length} job{jobs.length !== 1 ? "s" : ""}
					{#if hasActiveJobs}
						<span class="ml-2 text-blue-600">
							· {jobs.filter((j) => j.status === "pending" || j.status === "running").length} active
						</span>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</Page>
