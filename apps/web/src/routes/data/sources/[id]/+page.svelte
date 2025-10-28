<script lang="ts">
	import { page } from "$app/state";
	import { Button, Page } from "$lib";
	import {
		getSourceById,
		getStreamsBySourceId,
		getSyncLogsBySourceId,
		getSourceTypeIcon,
		getSourceTypeColor,
		formatCronSchedule,
	} from "$lib/mock-data/connections";
	import "iconify-icon";

	let sourceId = $derived(page.params.id);
	let source = $derived(getSourceById(sourceId));
	let streams = $derived(getStreamsBySourceId(sourceId));
	let recentLogs = $derived(getSyncLogsBySourceId(sourceId, 5));

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

	function getStatusBadgeClass(status: string | null): string {
		const classes: Record<string, string> = {
			success: "bg-green-100 text-green-700",
			failed: "bg-red-100 text-red-700",
			partial: "bg-yellow-100 text-yellow-700",
			never: "bg-neutral-100 text-neutral-500",
		};
		return classes[status || "never"] || "bg-neutral-100 text-neutral-500";
	}

	function toggleStream(streamId: string) {
		// Mock toggle - in real app this would call an API
		console.log("Toggle stream:", streamId);
	}

	function syncStream(streamId: string) {
		// Mock sync - in real app this would call an API
		console.log("Sync stream:", streamId);
	}
</script>

{#if !source}
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
				<a
					href="/data/sources"
					class="inline-flex items-center gap-2 text-neutral-600 hover:text-neutral-900 mb-4 text-sm"
				>
					<iconify-icon icon="ri:arrow-left-line"></iconify-icon>
					Back to Sources
				</a>

				<div class="flex items-start justify-between">
					<div class="flex items-center gap-4">
						<iconify-icon
							icon={getSourceTypeIcon(source.type)}
							class="text-5xl {getSourceTypeColor(source.type)}"
						></iconify-icon>
						<div>
							<div class="flex items-center gap-3 mb-1">
								<h1 class="text-3xl font-serif font-medium text-neutral-900">
									{source.name}
								</h1>
								{#if source.isActive}
									<span
										class="inline-block px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-full"
									>
										Active
									</span>
								{:else}
									<span
										class="inline-block px-2 py-1 text-xs font-medium bg-neutral-100 text-neutral-500 rounded-full"
									>
										Inactive
									</span>
								{/if}
							</div>
							<p class="text-neutral-600 capitalize">{source.type} Source</p>
						</div>
					</div>

					<Button variant="danger" class="flex items-center gap-2">
						<iconify-icon icon="ri:delete-bin-line"></iconify-icon>
						<span>Delete</span>
					</Button>
				</div>
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
									Enabled
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Schedule
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Last Sync
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Status
								</th>
								<th class="px-6 py-3 text-right text-xs font-medium text-neutral-600 uppercase tracking-wider">
									Actions
								</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-neutral-200">
							{#each streams as stream}
								<tr class="hover:bg-neutral-50 transition-colors">
									<td class="px-6 py-4">
										<div>
											<div class="font-medium text-neutral-900">{stream.displayName}</div>
											<div class="text-sm text-neutral-500">{stream.description}</div>
										</div>
									</td>
									<td class="px-6 py-4">
										<button
											class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {stream.isEnabled
												? 'bg-green-600'
												: 'bg-neutral-300'}"
											onclick={() => toggleStream(stream.id)}
										>
											<span
												class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {stream.isEnabled
													? 'translate-x-6'
													: 'translate-x-1'}"
											></span>
										</button>
									</td>
									<td class="px-6 py-4">
										<div class="flex items-center gap-2 text-sm text-neutral-900">
											<iconify-icon
												icon={stream.cronSchedule ? "ri:calendar-check-line" : "ri:hand-line"}
												class="text-neutral-400"
											></iconify-icon>
											{formatCronSchedule(stream.cronSchedule)}
										</div>
									</td>
									<td class="px-6 py-4">
										<span class="text-sm text-neutral-900">
											{formatRelativeTime(stream.lastSyncAt)}
										</span>
									</td>
									<td class="px-6 py-4">
										<span
											class="inline-block px-2 py-1 text-xs font-medium rounded-full capitalize {getStatusBadgeClass(
												stream.lastSyncStatus
											)}"
										>
											{stream.lastSyncStatus || "never"}
										</span>
									</td>
									<td class="px-6 py-4 text-right">
										<Button
											variant="ghost"
											size="sm"
											onclick={() => syncStream(stream.id)}
											disabled={!stream.isEnabled}
											class="border border-neutral-200"
										>
											Sync Now
										</Button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>

			<!-- Recent Sync History Section -->
			<div class="mb-8">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-xl font-serif font-medium text-neutral-900">Recent Sync History</h2>
					<a
						href="/data/logs"
						class="text-sm text-neutral-600 hover:text-neutral-900 flex items-center gap-1"
					>
						View all logs
						<iconify-icon icon="ri:arrow-right-line"></iconify-icon>
					</a>
				</div>

				{#if recentLogs.length === 0}
					<div
						class="bg-white border border-neutral-200 rounded-lg p-8 text-center text-neutral-500"
					>
						No sync history yet
					</div>
				{:else}
					<div class="bg-white border border-neutral-200 rounded-lg overflow-hidden">
						<table class="w-full">
							<thead class="bg-neutral-50 border-b border-neutral-200">
								<tr>
									<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Time
									</th>
									<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Stream
									</th>
									<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Status
									</th>
									<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Records
									</th>
									<th class="px-6 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider">
										Duration
									</th>
								</tr>
							</thead>
							<tbody class="divide-y divide-neutral-200">
								{#each recentLogs as log}
									<tr class="hover:bg-neutral-50 transition-colors">
										<td class="px-6 py-4 text-sm text-neutral-900">
											{formatRelativeTime(log.startedAt)}
										</td>
										<td class="px-6 py-4 text-sm text-neutral-900">
											{log.streamDisplayName}
										</td>
										<td class="px-6 py-4">
											<span
												class="inline-block px-2 py-1 text-xs font-medium rounded-full capitalize {getStatusBadgeClass(
													log.status
												)}"
											>
												{log.status}
											</span>
										</td>
										<td class="px-6 py-4 text-sm text-neutral-900">
											{log.recordsWritten ?? 0}
										</td>
										<td class="px-6 py-4 text-sm text-neutral-900">
											{log.durationMs ? `${log.durationMs}ms` : "-"}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		</div>
	</Page>
{/if}
