<script lang="ts">
	import { Button, Page } from "$lib";

	import "iconify-icon";
	import type { PageData } from "./$types";

	export let data: PageData;

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

	function isSourceConnected(catalogSourceName: string): boolean {
		return data.sources.some((s: any) => s.type === catalogSourceName);
	}
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class=" flex items-center justify-between">
			<div>
				<h1
					class="text-3xl font-serif font-medium text-neutral-900 mb-2"
				>
					Sources
				</h1>
				<p class="text-neutral-600">
					Manage your connected data sources and their sync schedules
				</p>
			</div>
			<a href="/data/sources/add">
				<Button variant="secondary" class="flex items-center gap-2">
					<iconify-icon icon="ri:add-line" class="text-lg"
					></iconify-icon>
					<span>Add Source</span>
				</Button>
			</a>
		</div>

		<!-- Connected Sources Section -->
		<div class="mt-8">
			<h2 class="text-xl font-serif font-medium text-neutral-900 mb-4">
				Connected Sources
				{#if data.sources.length > 0}
					<span class="text-neutral-400 text-sm font-normal"
						>( {data.sources.length} )</span
					>
				{/if}
			</h2>

			{#if data.sources.length === 0}
				<!-- Empty State -->
				<div
					class="border border-neutral-200 rounded-lg p-12 text-center bg-neutral-50"
				>
					<iconify-icon
						icon="ri:plug-line"
						class="text-6xl text-neutral-300 mb-4"
					></iconify-icon>
					<h3 class="text-lg font-medium text-neutral-900 mb-2">
						No sources connected
					</h3>
					<p class="text-neutral-600 mb-4">
						Connect your first data source to start syncing your
						personal data
					</p>
					<a href="/data/sources/add">
						<Button variant="primary">Add Your First Source</Button>
					</a>
				</div>
			{:else}
				<!-- Connected Sources Cards -->
				<div
					class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
				>
					{#each data.sources as source}
						<a
							href="/data/sources/{source.id}"
							class="block p-6 bg-white border border-neutral-200 rounded-lg hover:border-neutral-300 hover:bg-neutral-50 transition-all duration-200 cursor-pointer"
						>
							<!-- Header with Icon and Status -->
							<div class="flex items-start justify-between mb-4">
								<div class="flex items-center gap-3">
									<h3 class="font-medium text-neutral-900">
										{source.name}
									</h3>
								</div>
								{#if source.is_active}
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
									<span class="text-neutral-600">Streams</span
									>
									<span class="font-medium text-neutral-900">
										{source.enabled_streams_count} of {source.total_streams_count}
										enabled
									</span>
								</div>
								<div class="flex items-center justify-between">
									<span class="text-neutral-600"
										>Last sync</span
									>
									<span class="font-medium text-neutral-900">
										{formatRelativeTime(
											source.last_sync_at,
										)}
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

		<!-- Available Sources Catalog -->
		<div class="pt-8">
			<h2 class="text-xl font-serif font-medium text-neutral-900 mb-2">
				Available Sources <span class="text-neutral-400 font-normal">
					<span class="text-neutral-400 text-sm font-normal">
						( {data.catalog.length} )
					</span>
				</span>
			</h2>

			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
				{#each data.catalog as catalogSource}
					{@const connected = isSourceConnected(catalogSource.name)}
					<div
						class="p-6 bg-white border border-neutral-200 rounded-lg hover:border-neutral-300 transition-all duration-200"
					>
						<!-- Header with Icon/Title -->
						<div class="flex items-center justify-between mb-3">
							<div class="flex items-center gap-3">
								<h3
									class="font-medium text-neutral-900 capitalize"
								>
									{catalogSource.name}
								</h3>
							</div>

							<!-- Connected Badge -->
							{#if connected}
								<span
									class="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-full"
								>
									<iconify-icon icon="ri:check-line"
									></iconify-icon>
									Connected
								</span>
							{/if}
						</div>

						<p class="text-sm text-neutral-600 mb-4 line-clamp-2">
							{catalogSource.description}
						</p>

						<!-- Metadata -->
						<div class="flex items-center justify-between">
							<div
								class="flex items-center gap-3 text-xs text-neutral-500"
							>
								<span class="flex items-center gap-1">
									<iconify-icon icon="ri:database-2-line"
									></iconify-icon>
									{catalogSource.stream_count}
									{catalogSource.stream_count === 1
										? "stream"
										: "streams"}
								</span>
								<span
									class="inline-block px-2 py-0.5 bg-neutral-100 text-neutral-600 rounded-full capitalize"
								>
									{catalogSource.auth_type === "oauth2"
										? "OAuth"
										: catalogSource.auth_type}
								</span>
							</div>
							<a
								href="/data/sources/add?type={catalogSource.name}"
							>
								<Button variant="primary" size="sm">
									{connected ? "Add Another" : "Add"}
								</Button>
							</a>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</Page>
