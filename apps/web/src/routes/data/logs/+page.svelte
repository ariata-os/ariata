<script lang="ts">
	import { Page, Button } from "$lib";
	import {
		getAllActivities,
		getActivityTypeLabel,
		formatDuration,
		type Activity,
	} from "$lib/mock-data/connections";
	import "iconify-icon";

	let activities = getAllActivities();

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

	function getStatusBadgeClass(status: string): string {
		const classes: Record<string, string> = {
			success: "bg-green-50 text-green-700 border border-green-200",
			failed: "bg-red-50 text-red-700 border border-red-200",
			partial: "bg-amber-50 text-amber-700 border border-amber-200",
			in_progress: "bg-blue-50 text-blue-700 border border-blue-200",
		};
		return classes[status] || "bg-neutral-50 text-neutral-600 border border-neutral-200";
	}
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Activity</h1>
			<p class="text-neutral-600">
				System activity including syncs, transformations, and configuration changes
			</p>
		</div>

		<!-- Activity Table -->
		{#if activities.length === 0}
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
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-200">
						{#each activities as activity}
							<tr class="hover:bg-neutral-50 transition-colors">
								<!-- Type -->
								<td class="px-6 py-4 whitespace-nowrap">
									<span class="text-sm text-neutral-900">
										{getActivityTypeLabel(activity.type)}{#if activity.streamDisplayName}
											· {activity.streamDisplayName}
										{/if}
									</span>
								</td>

								<!-- Source -->
								<td class="px-6 py-4 whitespace-nowrap">
									{#if activity.sourceName && activity.sourceId}
										<a
											href="/data/sources/{activity.sourceId}"
											class="text-sm text-neutral-700 hover:text-neutral-900 hover:underline"
										>
											{activity.sourceName}
										</a>
									{:else if activity.sourceName}
										<span class="text-sm text-neutral-700">
											{activity.sourceName}
										</span>
									{:else}
										<span class="text-sm text-neutral-400">—</span>
									{/if}
								</td>

								<!-- Status -->
								<td class="px-6 py-4 whitespace-nowrap">
									<span
										class="inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded-full capitalize {getStatusBadgeClass(
											activity.status
										)}"
									>
										{#if activity.status === "failed" || (activity.status === "partial" && activity.errorMessage)}
											<iconify-icon icon="ri:error-warning-line" class="text-xs"
											></iconify-icon>
										{/if}
										{activity.status.replace("_", " ")}
									</span>
								</td>

								<!-- Duration -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									<span class="text-sm text-neutral-600">
										{formatDuration(activity.durationMs)}
									</span>
								</td>

								<!-- Records -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									{#if activity.recordsWritten !== null && activity.recordsWritten !== undefined}
										<span class="text-sm text-neutral-700">
											{activity.recordsWritten.toLocaleString()}{#if activity.recordsFailed && activity.recordsFailed > 0}<span
													class="text-red-600"
													>*</span
												>{/if}
										</span>
									{:else}
										<span class="text-sm text-neutral-300">—</span>
									{/if}
								</td>

								<!-- Time -->
								<td class="px-6 py-4 whitespace-nowrap text-right">
									<span class="text-sm text-neutral-600">
										{formatRelativeTime(activity.startedAt)}
									</span>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Pagination (non-functional) -->
			<div class="mt-4 flex items-center justify-between">
				<div class="text-sm text-neutral-600">
					Showing {activities.length} of {activities.length} entries
				</div>
				<div class="flex gap-2">
					<Button variant="secondary" size="sm" disabled>
						Previous
					</Button>
					<Button variant="secondary" size="sm" disabled>
						Next
					</Button>
				</div>
			</div>
		{/if}
	</div>
</Page>
