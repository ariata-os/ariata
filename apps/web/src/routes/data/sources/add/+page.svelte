<script lang="ts">
	import { Button, Page } from "$lib";
	import TypedSelect from "$lib/components/TypedSelect.svelte";
	import DevicePairing from "$lib/components/DevicePairing.svelte";
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import type { PageData } from "./$types";
	import type { DeviceInfo } from "$lib/types/device-pairing";
	import * as api from "$lib/api/client";
	import { toast } from "svelte-sonner";

	let { data }: { data: PageData } = $props();

	interface CatalogSource {
		name: string;
		display_name: string;
		description: string;
		auth_type: string;
		stream_count: number;
		icon?: string;
	}

	interface Stream {
		stream_name: string;
		display_name: string;
		description: string;
		is_enabled: boolean;
		supports_incremental: boolean;
		default_cron_schedule: string | null;
	}

	let selectedSource: CatalogSource | null = $state(
		data.selectedSource || null,
	);
	let sourceName = $state("");
	let selectedStreams = $state<Set<string>>(new Set());
	let availableStreams: Stream[] = $state([]);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let createdSourceId: string | null = $state(null);

	let currentStep = $state<1 | 2 | 3>(1);

	let oauthSources: CatalogSource[] = $state([]);
	let deviceSources: CatalogSource[] = $state([]);
	let allSources: CatalogSource[] = $state([]);

	// Device pairing state
	let devicePairingSourceId: string | null = $state(null);
	let devicePairingInfo: DeviceInfo | null = $state(null);

	// Configure mode state (when coming from OAuth callback)
	let isConfigureMode = $state(!!data.existingSource);
	let configureSource = $state(data.existingSource);
	let configureStreams = $state<Stream[]>(data.availableStreams || []);

	$effect(() => {
		if (data.catalog) {
			allSources = data.catalog;
			oauthSources = data.catalog.filter(
				(s: CatalogSource) => s.auth_type === "oauth2",
			);
			deviceSources = data.catalog.filter(
				(s: CatalogSource) => s.auth_type === "device",
			);
		}
	});

	// Initialize selected streams in configure mode (all checked by default)
	$effect(() => {
		if (isConfigureMode && configureStreams.length > 0) {
			selectedStreams = new Set(
				configureStreams.map((s) => s.stream_name),
			);
		}
	});

	onMount(() => {
		if (selectedSource) {
			sourceName = `${selectedSource.display_name} Account`;
			currentStep = 2;
		}
	});

	function handleSourceSelect(source: CatalogSource | null) {
		selectedSource = source;
		if (!source) {
			currentStep = 1;
			sourceName = "";
			return;
		}
		sourceName = `${source.display_name} Account`;
		currentStep = 2;
	}

	// Watch for selectedSource changes to update step
	$effect(() => {
		if (selectedSource && currentStep === 1) {
			sourceName = `${selectedSource.display_name} Account`;
			currentStep = 2;
		}
	});

	async function handleAuthorize() {
		if (!selectedSource) return;

		isLoading = true;
		error = null;

		try {
			// Build the callback URL for our app
			const callbackUrl = `${window.location.origin}/oauth/callback`;

			// Get the OAuth authorization URL from the backend
			const oauthResponse = await api.initiateOAuth(
				selectedSource.name,
				callbackUrl,
			);

			// Redirect to the OAuth provider's authorization page
			window.location.href = oauthResponse.authorization_url;
		} catch (e) {
			error = e instanceof Error ? e.message : "Authorization failed";
			isLoading = false;
		}
	}

	// Handle device pairing success
	async function handleDevicePairingSuccess(
		sourceId: string,
		deviceInfo: DeviceInfo,
	) {
		devicePairingSourceId = sourceId;
		devicePairingInfo = deviceInfo;

		// Fetch available streams for this device source
		try {
			const streams = await api.listStreams(sourceId);
			availableStreams = streams;
			// Select all streams by default
			selectedStreams = new Set(streams.map((s: Stream) => s.stream_name));
			// Move to step 3
			currentStep = 3;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load streams";
		}
	}

	// Handle device pairing cancel
	function handleDevicePairingCancel() {
		// Reset state and go back to step 1
		selectedSource = null;
		currentStep = 1;
		devicePairingSourceId = null;
		devicePairingInfo = null;
	}

	function toggleStream(streamName: string) {
		const newSet = new Set(selectedStreams);
		if (newSet.has(streamName)) {
			newSet.delete(streamName);
		} else {
			newSet.add(streamName);
		}
		selectedStreams = newSet;
	}

	async function handleSaveStreams() {
		if (!configureSource || selectedStreams.size === 0) return;

		isLoading = true;
		error = null;

		try {
			// Enable each selected stream
			for (const streamName of selectedStreams) {
				await api.enableStream(configureSource.id, streamName);
			}

			// Show success toast with sync info
			const streamCount = selectedStreams.size;
			toast.success(
				`${streamCount} stream${streamCount === 1 ? "" : "s"} enabled and syncing in the background`,
			);

			// Redirect to sources list
			await goto("/data/sources");
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to enable streams";
			isLoading = false;
		}
	}

	async function handleEnableStreams() {
		// For device sources, use devicePairingSourceId; for OAuth, use createdSourceId
		const sourceId = devicePairingSourceId || createdSourceId;
		if (!sourceId || selectedStreams.size === 0) return;

		isLoading = true;
		error = null;

		try {
			// Enable each selected stream
			for (const streamName of selectedStreams) {
				await api.enableStream(sourceId, streamName);
			}

			// Show success toast
			const streamCount = selectedStreams.size;
			toast.success(
				`${streamCount} stream${streamCount === 1 ? "" : "s"} enabled and syncing in the background`,
			);

			// Redirect to sources list
			await goto("/data/sources");
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to enable streams";
			isLoading = false;
		}
	}

	function formatCron(cron: string | null): string {
		if (!cron) return "Manual";
		const map: Record<string, string> = {
			"*/15 * * * *": "Every 15 minutes",
			"*/30 * * * *": "Every 30 minutes",
			"0 */1 * * *": "Every hour",
			"0 */6 * * *": "Every 6 hours",
			"0 0 * * *": "Daily at midnight",
		};
		return map[cron] || cron;
	}
</script>

<Page>
	<div class="max-w-2xl">
		<div class="mb-12">
			<h1 class="text-3xl font-serif font-normal text-neutral-900 mb-3">
				Add Source
			</h1>
			<p class="text-neutral-600 leading-relaxed">
				Connect a new data source to start syncing your personal data.
			</p>
		</div>

		{#if error}
			<div class="mb-8 p-4 border border-neutral-300 bg-neutral-50">
				<p class="text-sm font-serif text-neutral-900">{error}</p>
			</div>
		{/if}

		<div class="space-y-12">
			<!-- Step 1: Select Provider -->
			<div>
				<h2
					class="text-xl font-serif font-normal text-neutral-900 mb-6"
				>
					{#if isConfigureMode}
						<span class="text-green-600">✓</span>
					{/if}
					1. Select Source
				</h2>

				{#if !isConfigureMode}
					<div class="space-y-4 w-2/3">
						<TypedSelect
							items={allSources}
							bind:value={selectedSource}
							onValueChange={handleSourceSelect}
							label="Data Source"
							placeholder="Type to search..."
							disabled={isConfigureMode}
							displayKey="display_name"
							searchKey="display_name"
						>
							{#snippet item(source)}
								<div>
									<div class="flex items-center gap-2 mb-1">
										<span class="text-neutral-900">
											{source.display_name}
										</span>
										{#if source.auth_type === "device"}
											<span
												class="text-xs px-2 py-0.5 bg-blue-100 text-blue-700 rounded"
											>
												Device
											</span>
										{/if}
									</div>
									<div class="text-sm text-neutral-600">
										{source.description}
									</div>
								</div>
							{/snippet}
						</TypedSelect>

						{#if selectedSource}
							<div class="pt-4 border-t border-neutral-200">
								<p class="text-sm text-neutral-600 leading-relaxed">
									{selectedSource.description}
								</p>
								<p class="text-sm text-neutral-500 mt-2">
									{selectedSource.stream_count}
									{selectedSource.stream_count === 1
										? "stream"
										: "streams"} available
								</p>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Step 2: Authorize (OAuth) or Pair (Device) -->
			<div>
				<h2
					class="text-xl font-serif font-normal text-neutral-900 mb-6"
				>
					{#if isConfigureMode || devicePairingInfo}
						<span class="text-green-600">✓</span>
					{/if}
					2. {selectedSource?.auth_type === "device"
						? "Pair Device"
						: "Authorize"}
				</h2>

				{#if !isConfigureMode}
					{#if currentStep >= 2 && selectedSource}
						{#if selectedSource.auth_type === "device"}
							<!-- Device Pairing Flow -->
							<div class="space-y-6">
								<div>
									<label class="block text-sm text-neutral-700 mb-2">
										Device Name
									</label>
									<input
										type="text"
										bind:value={sourceName}
										placeholder="e.g., My {selectedSource.display_name}"
										class="w-full px-4 py-2 rounded border border-neutral-300 bg-white text-neutral-900 focus:outline-none focus:border-neutral-900"
										disabled={!!devicePairingInfo}
									/>
									<p class="text-sm text-neutral-500 mt-2">
										A memorable name for this device
									</p>
								</div>

								{#if !devicePairingInfo && sourceName.trim()}
									<div class="pt-6 border-t border-neutral-200">
										<DevicePairing
											deviceType={selectedSource.name}
											deviceName={sourceName}
											onSuccess={handleDevicePairingSuccess}
											onCancel={handleDevicePairingCancel}
										/>
									</div>
								{:else if devicePairingInfo}
									<div class="pt-6 border-t border-neutral-200">
										<p class="text-sm text-neutral-600">
											✓ Device paired: {devicePairingInfo.device_name}
										</p>
									</div>
								{/if}
							</div>
						{:else}
							<!-- OAuth Flow -->
							<div class="space-y-6">
								<div>
									<label class="block text-sm text-neutral-700 mb-2">
										Source Name
									</label>
									<input
										type="text"
										bind:value={sourceName}
										placeholder="e.g., My {selectedSource.display_name} Account"
										class="w-full px-4 py-2 rounded border border-neutral-300 bg-white text-neutral-900 focus:outline-none focus:border-neutral-900"
									/>
									<p class="text-sm text-neutral-500 mt-2">
										A memorable name for this connection
									</p>
								</div>

								{#if currentStep === 2}
									<div class="pt-6 border-t border-neutral-200">
										<p
											class="text-sm text-neutral-600 mb-4 leading-relaxed"
										>
											You'll be redirected to {selectedSource.display_name}
											to authorize access. We request read-only
											permissions.
										</p>
										<Button
											onclick={handleAuthorize}
											disabled={isLoading || !sourceName.trim()}
										>
											{#if isLoading}
												Authorizing...
											{:else}
												Authorize
											{/if}
										</Button>
									</div>
								{:else if currentStep > 2}
									<div class="pt-6 border-t border-neutral-200">
										<p class="text-sm text-neutral-600">
											✓ Connected as "{sourceName}"
										</p>
									</div>
								{/if}
							</div>
						{/if}
					{:else}
						<p class="text-sm text-neutral-500">
							Complete step 1 to continue
						</p>
					{/if}
				{/if}
			</div>

			<!-- Step 3: Enable Streams -->
			<div>
				<h2
					class="text-xl font-serif font-normal text-neutral-900 mb-6"
				>
					3. Enable Streams
				</h2>

				{#if isConfigureMode}
					<p class="text-sm text-neutral-600 mb-6 leading-relaxed">
						Choose which data streams to enable. All streams are
						selected by default.
					</p>

					<div class="space-y-3 mb-8">
						{#each configureStreams as stream}
							<label
								class="flex items-start gap-4 p-4 border border-neutral-200 cursor-pointer hover:border-neutral-300 transition-colors"
							>
								<input
									type="checkbox"
									checked={selectedStreams.has(
										stream.stream_name,
									)}
									onchange={() =>
										toggleStream(stream.stream_name)}
									class="mt-1 w-4 h-4 border-neutral-300"
								/>
								<div class="flex-1">
									<h3
										class="font-serif text-neutral-900 mb-1"
									>
										{stream.display_name}
									</h3>
									<p
										class="text-sm text-neutral-600 mb-2 leading-relaxed"
									>
										{stream.description}
									</p>
									{#if stream.default_cron_schedule}
										<p class="text-xs text-neutral-500">
											Default schedule: {formatCron(
												stream.default_cron_schedule,
											)}
										</p>
									{/if}
								</div>
							</label>
						{/each}
					</div>

					<div class="pt-6 border-t border-neutral-200">
						<Button
							onclick={handleSaveStreams}
							disabled={isLoading || selectedStreams.size === 0}
						>
							{#if isLoading}
								Saving...
							{:else}
								Enable {selectedStreams.size}
								{selectedStreams.size === 1
									? "Stream"
									: "Streams"}
							{/if}
						</Button>
					</div>
				{:else if currentStep >= 3}
					<p class="text-sm text-neutral-600 mb-6 leading-relaxed">
						Choose which data streams to enable. All streams are
						selected by default.
					</p>

					<div class="space-y-3 mb-8">
						{#each availableStreams as stream}
							<label
								class="flex items-start gap-4 p-4 border border-neutral-200 cursor-pointer hover:border-neutral-300 transition-colors"
							>
								<input
									type="checkbox"
									checked={selectedStreams.has(
										stream.stream_name,
									)}
									onchange={() =>
										toggleStream(stream.stream_name)}
									class="mt-1 w-4 h-4 border-neutral-300"
								/>
								<div class="flex-1">
									<div class="flex items-center gap-3 mb-1">
										<h3 class="font-serif text-neutral-900">
											{stream.display_name}
										</h3>
										{#if stream.supports_incremental}
											<span
												class="text-xs text-neutral-500"
											>
												Incremental
											</span>
										{/if}
									</div>
									<p
										class="text-sm text-neutral-600 mb-2 leading-relaxed"
									>
										{stream.description}
									</p>
									<p class="text-xs text-neutral-500">
										{formatCron(
											stream.default_cron_schedule,
										)}
									</p>
								</div>
							</label>
						{/each}
					</div>

					<div class="pt-6 border-t border-neutral-200">
						<Button
							onclick={handleEnableStreams}
							disabled={isLoading || selectedStreams.size === 0}
						>
							{#if isLoading}
								Enabling...
							{:else}
								Enable {selectedStreams.size}
								{selectedStreams.size === 1
									? "Stream"
									: "Streams"}
							{/if}
						</Button>
					</div>
				{:else}
					<p class="text-sm text-neutral-500 italic">
						Complete steps 1 & 2 to continue
					</p>
				{/if}
			</div>
		</div>
	</div>
</Page>
