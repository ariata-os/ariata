<script lang="ts">
	import { Page, Badge, Button } from "$lib/components";
	import "iconify-icon";
	import type { PageData } from "./$types";
	import { getVideoUrl } from "$lib/utils/videoLoader";

	let { data }: { data: PageData } = $props();

	// Track which card is being hovered
	let hoveredSource = $state<string | null>(null);

	// Store video element references
	let videoElements: Record<string, HTMLVideoElement> = {};

	// Video action to handle play/pause
	function handleVideo(node: HTMLVideoElement) {
		const sourceName = node.dataset.sourceName;

		// Preload the video to avoid flash
		node.load();

		$effect(() => {
			if (hoveredSource === sourceName) {
				node.play().catch(() => {
					// Ignore autoplay errors
				});
			} else {
				node.pause();
				// Don't reset currentTime to avoid flash
			}
		});

		return {
			destroy() {
				// Cleanup if needed
			},
		};
	}

	// Get the video source path using the video loader
	function getVideoSrc(video: string | null): string | null {
		return getVideoUrl(video);
	}

	// Get badge text for a source
	function getBadgeText(source: any): string {
		// For device sources, show the device type
		if (source.platform === "device" && source.deviceType) {
			return source.deviceType;
		}
		// For cloud sources, show platform or auth type
		if (source.platform === "cloud") {
			return source.authType || "cloud";
		}
		return source.platform;
	}

	// Get badge variant based on platform
	function getBadgeVariant(source: any): "default" | "info" {
		return source.platform === "cloud" ? "info" : "default";
	}
</script>

<Page>
	<div class="space-y-8">
		<!-- Header -->
		<div>
			<div class="flex items-center justify-between mb-4">
				<div>
					<h1 class="text-3xl text-neutral-900 font-mono">
						Source Catalog
					</h1>
					<p class="text-neutral-600 mt-1">
						Browse and connect available data sources
					</p>
				</div>
				<!-- <Button
					href="/data/sources"
					text="← Back to Sources"
					variant="outline"
				/> -->
			</div>
		</div>

		{#if data.error}
			<div class="bg-red-50 border border-red-200 rounded-lg p-4">
				<p class="text-red-700">{data.error}</p>
			</div>
		{/if}

		<!-- All Sources Grid -->
		{#if data.sources && data.sources.length > 0}
			<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
				{#each data.sources as source}
					{@const videoSrc = source.video
						? getVideoSrc(source.video)
						: null}
					<div
						class="group bg-white border border-neutral-200 rounded-lg overflow-hidden hover:border-neutral-300 hover:bg-neutral-100 transition-colors duration-300 cursor-pointer flex flex-col h-full"
						role="button"
						tabindex="0"
						onmouseenter={() => (hoveredSource = source.name)}
						onmouseleave={() => (hoveredSource = null)}
						onclick={() => {
							window.location.href = `/data/sources/new?source=${source.name}`;
						}}
						onkeydown={(e) => {
							if (e.key === "Enter" || e.key === " ") {
								e.preventDefault();
								window.location.href = `/data/sources/new?source=${source.name}`;
							}
						}}
					>
						{#if videoSrc}
							<div
								class="relative w-full aspect-video bg-neutral-900 overflow-hidden"
							>
								<video
									class="absolute inset-0 w-full h-full object-cover"
									muted
									loop
									playsinline
									bind:this={videoElements[source.name]}
									use:handleVideo
									data-source-name={source.name}
								>
									<source src={videoSrc} type="video/webm" />
								</video>
							</div>
						{/if}
						<div class="p-4 flex flex-col flex-grow">
							<div class="flex items-center gap-2 mb-2">
								{#if source.icon}
									<iconify-icon
										icon={source.icon}
										class="text-xl text-neutral-700"
									></iconify-icon>
								{/if}
								<h3
									class="text-lg font-semibold text-neutral-900 font-mono"
								>
									{source.displayName}
								</h3>
							</div>
							<p class="text-sm text-neutral-600 mb-3 flex-grow">
								{source.description}
							</p>
							<div
								class="flex items-center justify-between mt-auto"
							>
								<Badge
									variant={getBadgeVariant(source)}
									size="sm"
								>
									{getBadgeText(source)}
								</Badge>
								<Button
									text="Connect"
									variant="text"
									type="link"
									href={`/data/sources/new?source=${source.name}`}
								/>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{:else if !data.error}
			<!-- Empty State -->
			<div
				class="bg-neutral-100 border border-neutral-200 rounded-lg p-12 text-center"
			>
				<iconify-icon
					icon="ri:database-2-line"
					class="text-6xl text-neutral-400 mx-auto mb-4"
				></iconify-icon>
				<h3 class="text-xl font-mono text-neutral-900 mb-2">
					No sources available
				</h3>
				<p class="text-neutral-600">
					No data sources are configured in the system
				</p>
			</div>
		{/if}
	</div>
</Page>
