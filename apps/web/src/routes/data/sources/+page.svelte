<script lang="ts">
	import { Button, Page } from "$lib";
	import { mockSources, getSourceTypeIcon, getSourceTypeColor } from "$lib/mock-data/connections";
	import "iconify-icon";

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
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class="mb-8 flex items-center justify-between">
			<div>
				<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Sources</h1>
				<p class="text-neutral-600">
					Manage your connected data sources and their sync schedules
				</p>
			</div>
			<Button variant="secondary" disabled class="flex items-center gap-2">
				<iconify-icon icon="ri:add-line" class="text-lg"></iconify-icon>
				<span>Add Source</span>
			</Button>
		</div>

		<!-- Sources Grid -->
		{#if mockSources.length === 0}
			<!-- Empty State -->
			<div
				class="border-2 border-dashed border-neutral-200 rounded-lg p-12 text-center bg-neutral-50"
			>
				<iconify-icon icon="ri:plug-line" class="text-6xl text-neutral-300 mb-4"></iconify-icon>
				<h3 class="text-lg font-medium text-neutral-900 mb-2">No sources connected</h3>
				<p class="text-neutral-600 mb-4">
					Connect your first data source to start syncing your personal data
				</p>
				<Button variant="primary">
					Add Your First Source
				</Button>
			</div>
		{:else}
			<!-- Sources Cards -->
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each mockSources as source}
					<a
						href="/data/sources/{source.id}"
						class="block p-6 bg-white border border-neutral-200 rounded-lg hover:border-neutral-300 hover:bg-neutral-50 transition-all duration-200 cursor-pointer"
					>
						<!-- Header with Icon and Status -->
						<div class="flex items-start justify-between mb-4">
							<div class="flex items-center gap-3">
								<iconify-icon
									icon={getSourceTypeIcon(source.type)}
									class="text-3xl {getSourceTypeColor(source.type)}"
								></iconify-icon>
								<div>
									<h3 class="font-medium text-neutral-900">{source.name}</h3>
									<span
										class="inline-block px-2 py-0.5 text-xs font-medium bg-neutral-100 text-neutral-600 rounded-full capitalize mt-1"
									>
										{source.type}
									</span>
								</div>
							</div>
							{#if source.isActive}
								<span
									class="inline-block w-2 h-2 bg-green-500 rounded-full"
									title="Active"
								></span>
							{:else}
								<span
									class="inline-block w-2 h-2 bg-neutral-300 rounded-full"
									title="Inactive"
								></span>
							{/if}
						</div>

						<!-- Stats -->
						<div class="space-y-2 text-sm">
							<div class="flex items-center justify-between">
								<span class="text-neutral-600">Streams</span>
								<span class="font-medium text-neutral-900">
									{source.enabledStreamsCount} of {source.totalStreamsCount} enabled
								</span>
							</div>
							<div class="flex items-center justify-between">
								<span class="text-neutral-600">Last sync</span>
								<span class="font-medium text-neutral-900">
									{formatRelativeTime(source.lastSyncAt)}
								</span>
							</div>
						</div>

						<!-- Footer -->
						<div class="mt-4 pt-4 border-t border-neutral-100">
							<span class="text-xs text-neutral-500">
								View streams and sync history →
							</span>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</Page>
