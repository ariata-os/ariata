<script lang="ts">
	import {
		Page,
		Button,
		Input,
		Toggle,
		Badge,
		Textarea,
		Radio,
	} from "$lib/components";
	import { goto, invalidate } from "$app/navigation";
	import { onDestroy } from "svelte";
	import { toast } from "svelte-sonner";
	import "iconify-icon";
	import type { PageData } from "./$types";
	import { slide } from "svelte/transition";
	import { getVideoUrl } from "$lib/utils/videoLoader";

	// Function to get video URL for a source
	function getSourceVideo(sourceName: string): string | null {
		// Map source names to their video files
		const videoFileMap: Record<string, string> = {
			google: "google2.webm",
			ios: "ios.webm",
			mac: "mac2.webm",
			notion: "notion.webm",
			strava: "strava.webm"
		};
		
		const videoFile = videoFileMap[sourceName];
		return videoFile ? getVideoUrl(videoFile) : null;
	}

	// Track hover state for video
	let isVideoHovered = $state(false);
	let videoElement: HTMLVideoElement;

	// Handle video play/pause based on hover
	$effect(() => {
		if (videoElement) {
			if (isVideoHovered) {
				videoElement.play().catch(() => {
					// Ignore autoplay errors
				});
			} else {
				videoElement.pause();
			}
		}
	});

	interface StreamConfig {
		id: string;
		name: string;
		displayName: string;
		description?: string;
		ingestionType: string;
		cronSchedule?: string;
		settings: Record<string, any>;
		enabled: boolean;
		syncSchedule: string;
		syncType?: 'token' | 'date_range' | 'hybrid' | 'none';
		supportsInitialSync?: boolean;
		initialSyncType: "limited" | "full";
		initialSyncDays: number;
		initialSyncDaysFuture?: number;
		supportsFutureSync?: boolean;
		showAdvanced?: boolean;
		cronValid?: boolean;
	}

	let { data }: { data: PageData } = $props();

	// Form state
	let instanceName = $state(`${data.source.displayName} Account`);
	let connectionDescription = $state("");
	
	// Track if the user has meaningfully filled out the basic info
	let basicInfoComplete = $derived(instanceName && instanceName.trim() !== "" && connectionDescription && connectionDescription.trim() !== "");
	
	// Track if all steps are complete for OAuth sources
	let allStepsComplete = $derived(
		basicInfoComplete && 
		data.source.isConnected && 
		streamConfigs.some(s => s.enabled)
	);
	
	let deviceToken = $state("");
	let generatedToken = $state("");
	let autoSync = $state(true);
	let syncInterval = $state("60");
	let isSubmitting = $state(false);
	let errorMessage = $state("");
	let deviceConnected = $state(false);
	let checkingConnection = $state(false);
	let connectionCheckInterval: number | null = null;

	// Stream configurations - initialize each stream with its own sync settings
	let streamConfigs = $state<StreamConfig[]>(
		data.streams?.map((stream: any) => {
			// Check sync type from configuration
			const syncType = stream.settings?.sync?.type;
			
			// Determine if this stream supports initial sync based on sync type
			const supportsInitialSync = syncType === 'date_range' || syncType === 'hybrid';
			
			// Detect if this stream supports future sync
			const supportsFutureSync = 
				(stream.settings?.sync?.initial_sync_days_future !== undefined ||
				stream.settings?.sync?.lookahead_days !== undefined ||
				stream.name?.includes('calendar')) && syncType !== 'token' && syncType !== 'none';
			
			// Get default future days from stream config or use 30
			const defaultFutureDays = 
				stream.settings?.sync?.initial_sync_days_future || 
				(stream.settings?.sync?.lookahead_days ? Math.floor(stream.settings.sync.lookahead_days / 12) : 30);
			
			return {
				...stream,
				enabled: true,
				syncSchedule: stream.settings?.sync?.schedule || stream.cronSchedule || "0 * * * *", // Default to hourly
				syncType,  // Store the sync type
				supportsInitialSync,  // New flag to control UI display
				initialSyncType: syncType === 'token' ? "full" as const : "limited" as const,
				initialSyncDays: stream.settings?.sync?.initial_sync_days || 90,
				initialSyncDaysFuture: supportsFutureSync ? defaultFutureDays : undefined,
				supportsFutureSync,
				showAdvanced: false,
				cronValid: true,
			};
		}) || [],
	);

	// Restore form data from localStorage if returning from OAuth
	// Also restore device token if returning to a pending source
	$effect(() => {
		if (typeof window !== 'undefined') {
			// Restore OAuth form data
			const storageKey = `ariata_oauth_form_${data.source.name}`;
			const saved = localStorage.getItem(storageKey);
			if (saved) {
				try {
					const formData = JSON.parse(saved);
					instanceName = formData.instanceName || instanceName;
					connectionDescription = formData.connectionDescription || "";
					pendingSourceId = formData.pendingSourceId || null;
					
					// Update data.source.existingSource if we have a pending source
					if (pendingSourceId) {
						data.source.existingSource = {
							id: pendingSourceId,
							instanceName: instanceName,
							status: 'authenticated'
						};
					}
					
					// Clean up after restoring
					localStorage.removeItem(storageKey);
				} catch (e) {
					console.error('Failed to restore form data:', e);
					localStorage.removeItem(storageKey);
				}
			}
			
			// If we have an existing source with pending status and a device token, restore it
			if (data.source.existingSource?.status === 'pending' && data.source.existingSource?.deviceToken) {
				generatedToken = data.source.existingSource.deviceToken;
				instanceName = data.source.existingSource.instanceName || instanceName;
			}
		}
	});

	// Validate cron expression (basic validation)
	function validateCron(cron: string): { valid: boolean } {
		const parts = cron.trim().split(/\s+/);
		if (parts.length !== 5) {
			return { valid: false };
		}

		try {
			const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;
			const isValidPart = (part: string, max: number) => {
				if (part === "*") return true;
				if (part.startsWith("*/")) {
					const interval = parseInt(part.substring(2));
					return !isNaN(interval) && interval > 0 && interval <= max;
				}
				const num = parseInt(part);
				return !isNaN(num) && num >= 0 && num <= max;
			};

			if (!isValidPart(minute, 59)) return { valid: false };
			if (!isValidPart(hour, 23)) return { valid: false };
			if (!isValidPart(dayOfMonth, 31)) return { valid: false };
			if (!isValidPart(month, 12)) return { valid: false };
			if (!isValidPart(dayOfWeek, 7)) return { valid: false };

			return { valid: true };
		} catch {
			return { valid: false };
		}
	}

	// Update cron validation when schedule changes
	function updateSyncSchedule(streamIndex: number, cron: string) {
		streamConfigs[streamIndex].syncSchedule = cron;
		streamConfigs[streamIndex].cronValid = validateCron(cron).valid;
	}

	// Handle form submission for device sources
	async function handleDeviceSubmit() {
		if (!generatedToken) {
			errorMessage = "Please generate a device token first";
			return;
		}

		isSubmitting = true;
		errorMessage = "";

		try {
			// Update the existing device source with stream configurations
			// The source was already created when the token was generated
			const response = await fetch("/api/streams", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					sourceName: data.source.name,
					sourceId: data.source.existingSource?.id,
					instanceName: instanceName,
					description: connectionDescription,
					streamConfigs: streamConfigs
						.filter((s) => s.enabled)
						.map((s) => ({
							streamName: s.name,
							enabled: s.enabled,
							syncSchedule: s.syncSchedule,
							initialSyncType: s.initialSyncType,
							initialSyncDays:
								s.initialSyncType === "limited"
									? s.initialSyncDays
									: null,
							initialSyncDaysFuture:
								s.initialSyncType === "limited" && s.supportsFutureSync
									? s.initialSyncDaysFuture
									: null,
						})),
				}),
			});

			if (!response.ok) {
				const error = await response.json();
				throw new Error(
					error.message || "Failed to save configuration",
				);
			}

			const result = await response.json();

			// Clear any saved form data on successful submission
			localStorage.removeItem(`ariata_oauth_form_${data.source.name}`);

			// Show success toast
			toast.success("Configuration saved successfully!", {
				description: "Starting initial sync..."
			});

			// Invalidate sources data to ensure fresh data when navigating back
			await invalidate('app:sources');

			// Redirect to source detail page or sources list
			await goto(`/data/sources/${result.id || ""}`);
		} catch (error) {
			errorMessage =
				error instanceof Error
					? error.message
					: "Failed to save device configuration. Please try again.";
			console.error("Error saving device configuration:", error);
			
			// Show error toast
			toast.error("Failed to save configuration", {
				description: errorMessage
			});
		} finally {
			isSubmitting = false;
		}
	}

	// Handle OAuth connection
	function handleOAuthConnect() {
		if (data.source.oauthUrl) {
			// Save form data to localStorage before OAuth redirect
			const storageKey = `ariata_oauth_form_${data.source.name}`;
			localStorage.setItem(storageKey, JSON.stringify({
				instanceName,
				connectionDescription
			}));
			
			window.location.href = data.source.oauthUrl;
		}
	}
	
	// Handle OAuth source configuration submission
	async function handleOAuthSubmit() {
		// Validate basic information is complete
		if (!basicInfoComplete) {
			errorMessage = "Please complete the basic information (name and description)";
			const basicInfoSection = document.querySelector('[data-basic-info-section]');
			if (basicInfoSection) {
				basicInfoSection.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
			return;
		}

		// Validate that OAuth authentication is complete
		if (!data.source.isConnected && !data.source.existingSource?.oauth_access_token) {
			errorMessage = `Please authenticate with ${data.source.displayName} first by clicking "Connect with ${data.source.company}"`;
			
			// Scroll to the authentication section to show the error
			const authSection = document.querySelector('[data-auth-section]');
			if (authSection) {
				authSection.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
			return;
		}

		// Validate at least one stream is enabled
		const enabledStreams = streamConfigs.filter(s => s.enabled);
		if (enabledStreams.length === 0) {
			errorMessage = "Please enable at least one stream to sync";
			const streamsSection = document.querySelector('[data-streams-section]');
			if (streamsSection) {
				streamsSection.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
			return;
		}

		isSubmitting = true;
		errorMessage = "";

		try {
			// Save stream configurations for the connected OAuth source
			const response = await fetch("/api/streams", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					sourceName: data.source.name,
					sourceId: data.source.existingSource?.id,
					instanceName: instanceName,
					description: connectionDescription,
					streamConfigs: streamConfigs
						.filter((s) => s.enabled)
						.map((s) => ({
							streamName: s.name,
							enabled: true,
							syncSchedule: s.syncSchedule,
							initialSyncType: s.initialSyncType,
							initialSyncDays:
								s.initialSyncType === "limited"
									? s.initialSyncDays
									: null,
							initialSyncDaysFuture:
								s.initialSyncType === "limited" && s.supportsFutureSync
									? s.initialSyncDaysFuture
									: null,
						})),
				}),
			});

			if (!response.ok) {
				const error = await response.json();
				throw new Error(
					error.message || "Failed to save stream configuration",
				);
			}

			const result = await response.json();
			
			// Clear any saved form data on successful submission
			localStorage.removeItem(`ariata_oauth_form_${data.source.name}`);

			// Show success toast
			toast.success("Configuration saved successfully!", {
				description: "Starting initial sync..."
			});

			// Invalidate sources data to ensure fresh data when navigating back
			await invalidate('app:sources');

			// Redirect to the source detail page
			await goto(`/data/sources/${result.sourceId}`);
		} catch (error) {
			errorMessage =
				error instanceof Error
					? error.message
					: "Failed to save stream configuration. Please try again.";
			console.error("Error saving stream configuration:", error);
			
			// Show error toast
			toast.error("Failed to save configuration", {
				description: errorMessage
			});
		} finally {
			isSubmitting = false;
		}
	}

	// Handle cancel
	async function handleCancel() {
		// Clear any saved form data when cancelling
		localStorage.removeItem(`ariata_oauth_form_${data.source.name}`);
		// Invalidate sources data to ensure fresh data when navigating back
		await invalidate('app:sources');
		goto("/data/sources");
	}

	// Generate a new device token and save it immediately
	async function generateDeviceToken() {
		// Validate that instance name is provided
		if (!instanceName.trim()) {
			errorMessage = "Please enter a name for this connection first";
			return;
		}

		try {
			// Create the device source immediately with the token
			const response = await fetch("/api/sources/device-token", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					sourceName: data.source.name,
					instanceName: instanceName.trim(),
					description: connectionDescription
				}),
			});

			const result = await response.json();

			if (!response.ok || !result.success) {
				throw new Error(result.error || "Failed to generate device token");
			}

			// Store the generated token and source ID
			generatedToken = result.deviceToken;
			data.source.existingSource = {
				id: result.source.id,
				instanceName: result.source.instanceName,
				status: result.source.status
			};

			// Reset connection status
			deviceConnected = false;
			if (connectionCheckInterval) {
				clearInterval(connectionCheckInterval);
			}

			// Start checking for device connection every 2 seconds
			startConnectionCheck();

			// Show success toast
			toast.success("Device token generated", {
				description: "Token is ready to use in your device app"
			});
		} catch (error) {
			console.error("Failed to generate device token:", error);
			errorMessage = error instanceof Error ? error.message : "Failed to generate device token";
			
			// Show error toast
			toast.error("Failed to generate token", {
				description: errorMessage
			});
		}
	}

	// Check if device has connected with the token
	async function checkDeviceConnection() {
		if (!generatedToken || deviceConnected) return;

		try {
			const response = await fetch(
				`/api/device/verify?token=${encodeURIComponent(generatedToken)}`,
			);
			const data = await response.json();

			if (data.connected) {
				deviceConnected = true;
				if (connectionCheckInterval) {
					clearInterval(connectionCheckInterval);
					connectionCheckInterval = null;
				}
			}
		} catch (error) {
			console.error("Failed to check device connection:", error);
		}
	}

	// Start polling for device connection
	function startConnectionCheck() {
		checkingConnection = true;
		connectionCheckInterval = setInterval(
			checkDeviceConnection,
			2000,
		) as unknown as number;
		// Also check immediately
		checkDeviceConnection();

		// Stop checking after 5 minutes
		setTimeout(
			() => {
				if (connectionCheckInterval) {
					clearInterval(connectionCheckInterval);
					connectionCheckInterval = null;
					checkingConnection = false;
				}
			},
			5 * 60 * 1000,
		);
	}

	// Cleanup on component destroy
	onDestroy(() => {
		if (connectionCheckInterval) {
			clearInterval(connectionCheckInterval);
		}
	});
</script>

<Page>
	<!-- Header -->
	<div class="mb-8">
		<!-- Title with icon -->
		<div class="flex items-center gap-6">
			{#if data.source.icon}
				<iconify-icon
					icon={data.source.icon}
					class="text-4xl text-neutral-700 bg-neutral-100 rounded-lg p-4 border border-neutral-200"
				></iconify-icon>
			{/if}
			<div>
				<h1 class="text-3xl font-mono text-neutral-900">
					Connect {data.source.displayName}
				</h1>
				<p class="text-neutral-600 mt-1">
					{data.source.description}
				</p>
			</div>
		</div>
	</div>
	<div class="flex space-x-8">
		<div class="w-3/5">
			{#if errorMessage}
				<div
					class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6"
				>
					<p class="text-red-700">{errorMessage}</p>
				</div>
			{/if}

			<!-- Basic Information Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6 mb-6" data-basic-info-section>
				<h2 class="text-lg font-mono text-neutral-900 mb-4">
					1. Basic information
				</h2>

				<div class="space-y-4">
					<div>
						<label
							for="instance-name"
							class="block text-sm font-medium text-neutral-700 mb-1"
						>
							Connection Name
						</label>
						<Input
							id="instance-name"
							type="text"
							bind:value={instanceName}
							placeholder="A friendly name to identify this connection"
							required
						/>
					</div>

					<div>
						<label
							for="connection-description"
							class="block text-sm font-medium text-neutral-700 mb-1"
						>
							Description
						</label>
						<Textarea
							id="connection-description"
							bind:value={connectionDescription}
							rows={3}
							placeholder="Add notes about this connection (e.g., which account, purpose, etc.)"
							required
						/>
					</div>
				</div>
			</div>

			<!-- Authentication Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6 mb-6" data-auth-section>
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-mono text-neutral-900">
						2. Authentication
					</h2>
					<Badge class="">OAuth Connection</Badge>
				</div>

				{#if data.source.authType === "oauth2"}
					<div class="space-y-4">
						{#if data.source.connectionSuccessful}
							<div class="bg-green-50 border border-green-200 rounded-lg p-4">
								<div class="flex items-start gap-3">
									<svg class="w-5 h-5 text-green-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
									</svg>
									<div>
										<h3 class="font-medium text-green-900">Successfully connected!</h3>
										<p class="text-sm text-green-700 mt-1">
											{data.source.displayName} has been connected to your account.
										</p>
									</div>
								</div>
							</div>
						{/if}
						
						<div>
							{#if data.source.isConnected && !data.source.connectionSuccessful}
								<div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4">
									<p class="text-sm text-blue-700">
										<strong>Note:</strong> {data.source.connectionCount || 1} {data.source.displayName} {data.source.connectionCount === 1 ? 'account is' : 'accounts are'} already connected
									</p>
									<p class="text-xs text-blue-600 mt-1">
										You can add another account or reconnect an existing one to update permissions.
									</p>
								</div>
							{:else if !data.source.isConnected}
								<p class="text-sm text-neutral-600 mb-4">
									You must authenticate with {data.source.company} before you can save and sync.
									We use secure auth proxies that only handle login credentials - your actual data
									never passes through our servers.
								</p>
							{/if}

							{#if data.source.requiredScopes && data.source.requiredScopes.length > 0}
								<div class="bg-neutral-100 rounded-lg p-4 mb-4">
									<p
										class="text-sm font-medium text-neutral-700 mb-2"
									>
										Permissions we'll request:
									</p>
									<ul class="space-y-1">
										{#each data.source.requiredScopes as scope}
											<li
												class="text-sm text-neutral-600 flex items-start"
											>
												<svg
													class="w-4 h-4 text-green-500 mr-2 mt-0.5 flex-shrink-0"
													fill="none"
													stroke="currentColor"
													viewBox="0 0 24 24"
												>
													<path
														stroke-linecap="round"
														stroke-linejoin="round"
														stroke-width="2"
														d="M5 13l4 4L19 7"
													></path>
												</svg>
												<span
													>{scope
														.replace(
															"https://www.googleapis.com/auth/",
															"",
														)
														.replace(
															".readonly",
															" (read-only)",
														)}</span
												>
											</li>
										{/each}
									</ul>
								</div>
							{/if}

							<Button
								text={data.source.isConnected ? "Add Another " + data.source.displayName + " Account" : "Connect with " + data.source.company}
								variant={data.source.isConnected ? "outline" : "filled"}
								onclick={handleOAuthConnect}
								disabled={isSubmitting}
							/>
						</div>
					</div>
				{:else if data.source.authType === "device_token"}
					<div class="space-y-4">
						<p class="text-sm text-neutral-600 mb-4">
							Generate a secure token for your {data.source
								.displayName} device to connect to Ariata.
						</p>

						{#if !generatedToken}
							<Button
								text="Generate Device Token"
								variant="filled"
								onclick={generateDeviceToken}
							/>
						{:else}
							<div class="space-y-4">
								<div class="bg-neutral-100 rounded-lg p-4">
									<label
										class="block text-sm font-medium text-neutral-700 mb-2"
									>
										Your Device Token
									</label>
									<code
										class="block font-mono text-xs bg-white px-3 py-2 rounded border border-neutral-200 select-all"
									>
										{generatedToken}
									</code>

									{#if !deviceConnected}
										<div
											class="mt-3 flex items-center gap-2 text-sm text-neutral-600"
										>
											<div
												class="w-2 h-2 bg-orange-400 rounded-full animate-pulse"
											></div>
											<span
												>Waiting for device
												connection...</span
											>
										</div>
									{:else}
										<div
											class="mt-3 flex items-center gap-2 text-sm text-green-600"
										>
											<svg
												class="w-4 h-4"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M5 13l4 4L19 7"
												></path>
											</svg>
											<span
												>Device connected successfully!</span
											>
										</div>
									{/if}
								</div>

								<div
									class="bg-white border border-neutral-200 rounded-lg p-4"
								>
									<div class="flex items-center gap-2 mb-3">
										<svg
											class="w-5 h-5 text-neutral-500"
											fill="none"
											stroke="currentColor"
											viewBox="0 0 24 24"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
											></path>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
											></path>
										</svg>
										<p
											class="text-sm font-medium text-neutral-700"
										>
											{data.source.deviceSetup?.title || `Setup your ${data.source.displayName}`}
										</p>
									</div>
									
									{#if data.source.deviceSetup}
										{#if data.source.name === 'mac'}
											<!-- Mac specific setup with enhanced installer -->
											<div class="space-y-4">
												<!-- Quick Install Option (Primary) -->
												<div class="bg-blue-50 border-2 border-blue-200 rounded-lg p-4">
													<h4 class="font-semibold text-blue-900 mb-2 flex items-center gap-2">
														<span class="text-lg">⚡</span> Quick Install
													</h4>
													<p class="text-sm text-blue-700 mb-3">
														Run this command in Terminal:
													</p>
													<div class="relative">
														<pre class="bg-gray-900 text-green-400 p-3 rounded font-mono text-xs overflow-x-auto whitespace-pre">curl -sSL https://github.com/ariata-os/ariata/releases/download/mac-latest/installer.sh | \
bash -s -- --token {generatedToken} --endpoint {typeof window !== "undefined" ? window.location.origin : ""}</pre>
														<button
															onclick={() => {
																const command = `curl -sSL https://github.com/ariata-os/ariata/releases/download/mac-latest/installer.sh | bash -s -- --token ${generatedToken} --endpoint ${window.location.origin}`;
																navigator.clipboard.writeText(command);
																toast.success('Command copied! Open Terminal and paste to run');
															}}
															class="absolute top-2 right-2 text-xs px-3 py-1.5 bg-white/90 text-gray-900 border border-gray-300 rounded hover:bg-white transition-colors font-medium"
														>
															Copy Command
														</button>
													</div>
													<div class="flex items-start gap-2 mt-3">
														<span class="text-blue-600 text-xs mt-0.5">💡</span>
														<div class="text-xs text-blue-600 space-y-1">
															<p>You'll be prompted for your admin password to install to /usr/local/bin</p>
															<p>The installer will guide you through setup with native macOS dialogs</p>
														</div>
													</div>
												</div>

												<!-- Manual Installation (collapsed by default) -->
												<details class="text-sm">
													<summary class="cursor-pointer text-neutral-600 hover:text-neutral-900 font-medium">
														Manual step-by-step installation
													</summary>
													<div class="mt-3 space-y-3 pl-4">
														{#each data.source.deviceSetup.setup_steps as step, i}
															<div class="space-y-1">
																<p class="text-xs font-medium text-neutral-600">Step {i + 1}: {step.label}</p>
																<div class="relative">
																	<code class="block font-mono text-xs bg-neutral-50 px-3 py-2 pr-20 rounded border border-neutral-200 overflow-x-auto whitespace-nowrap">
																		{step.command.replace('{TOKEN}', generatedToken)}
																	</code>
																	<button
																		onclick={() => {
																			navigator.clipboard.writeText(step.command.replace('{TOKEN}', generatedToken));
																			toast.success('Copied to clipboard');
																		}}
																		class="absolute right-2 top-1/2 -translate-y-1/2 text-xs px-2 py-1 bg-white border border-neutral-200 rounded hover:bg-neutral-50 transition-colors"
																	>
																		Copy
																	</button>
																</div>
															</div>
														{/each}
														<div class="space-y-1 mt-3">
															<p class="text-xs font-medium text-neutral-600">Step 4: Set API endpoint</p>
															<div class="relative">
																<code class="block font-mono text-xs bg-neutral-50 px-3 py-2 pr-20 rounded border border-neutral-200 overflow-x-auto whitespace-nowrap">
																	export ARIATA_API_URL="{typeof window !== "undefined" ? window.location.origin : ""}"
																</code>
																<button
																	onclick={() => {
																		navigator.clipboard.writeText(`export ARIATA_API_URL="${window.location.origin}"`);
																		toast.success('Copied to clipboard');
																	}}
																	class="absolute right-2 top-1/2 -translate-y-1/2 text-xs px-2 py-1 bg-white border border-neutral-200 rounded hover:bg-neutral-50 transition-colors"
																>
																	Copy
																</button>
															</div>
														</div>
													</div>
												</details>

												<!-- Permission Notice -->
												<div class="bg-amber-50 border border-amber-200 rounded-lg p-3 mt-4">
													<div class="flex gap-2">
														<span class="text-amber-600 mt-0.5">⚠️</span>
														<div>
															<p class="text-sm font-medium text-amber-800">Accessibility Permission Required</p>
															<p class="text-xs text-amber-700 mt-1">
																macOS will prompt for Accessibility permissions to monitor app usage. 
																This is required for the monitor to function.
															</p>
														</div>
													</div>
												</div>
											</div>
										{:else}
											<!-- iOS and other device setup -->
											<ol class="space-y-3 text-sm text-neutral-600 mt-2">
												{#each data.source.deviceSetup.setup_steps as step, i}
													<li class="flex gap-2">
														<span class="font-mono text-neutral-400">{i + 1}.</span>
														<div>
															<div class="font-medium text-neutral-700">
																{step.label.replace('{TOKEN}', generatedToken)}
															</div>
															{#if step.description}
																<div class="text-xs text-neutral-500 mt-0.5">
																	{step.description.replace('{TOKEN}', generatedToken)}
																</div>
															{/if}
														</div>
													</li>
												{/each}
												
												{#if data.source.deviceSetup.show_api_endpoint}
													<li class="flex gap-2">
														<span class="font-mono text-neutral-400">{data.source.deviceSetup.setup_steps.length + 1}.</span>
														<div>
															<div class="font-medium text-neutral-700">API Endpoint:</div>
															<code class="block mt-1 font-mono text-xs bg-neutral-50 px-2 py-1 rounded border border-neutral-200">
																{typeof window !== "undefined" ? window.location.origin : ""}
															</code>
														</div>
													</li>
												{/if}
											</ol>
										{/if}
									{:else}
										<!-- Fallback to hardcoded instructions -->
										<ol class="space-y-2 text-sm text-neutral-600 mt-2">
											<li>1. Copy the device token above</li>
											<li>2. Open the {data.source.displayName} app on your device</li>
											<li>3. Go to Settings → Connection</li>
											<li>4. Paste the token when prompted</li>
											<li>
												5. Enter this API endpoint:
												<code class="block mt-1 font-mono text-xs bg-neutral-50 px-2 py-1 rounded border border-neutral-200">
													{typeof window !== "undefined" ? window.location.origin : ""}
												</code>
											</li>
										</ol>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				{:else}
					<p class="text-sm text-neutral-600">
						No authentication required for this source.
					</p>
				{/if}
			</div>

			<!-- Stream Configuration Section -->
			{#if streamConfigs.length > 0}
				<div
					class="bg-white border border-neutral-200 rounded-lg p-6 mb-6"
					data-streams-section
				>
					<h2 class="text-lg font-mono text-neutral-900 mb-4">
						3. Stream configurations
					</h2>

					<div class="space-y-6">
						{#each streamConfigs as stream, index}
							<div
								class="border border-neutral-200 rounded-lg p-4 bg-neutral-100"
							>
								<!-- Stream header with enable toggle -->
								<div
									class="flex items-center justify-between mb-4"
								>
									<div>
										<h3 class="font-mono text-neutral-900">
											{stream.displayName}
										</h3>
										{#if stream.description}
											<p
												class="text-sm text-neutral-500 mt-1"
											>
												{stream.description}
											</p>
										{/if}
									</div>
									<Toggle bind:checked={stream.enabled} />
								</div>

								{#if stream.enabled}
									<div class="space-y-4">
										<!-- Advanced settings toggle -->
										<div
											class="border-t border-neutral-300 pt-3"
										>
											<button
												type="button"
												class="flex cursor-pointer hover:underline items-center text-sm text-neutral-600 hover:text-neutral-900 transition-colors"
												onclick={() =>
													(streamConfigs[
														index
													].showAdvanced =
														!streamConfigs[index]
															.showAdvanced)}
											>
												<svg
													class="w-4 h-4 mr-1.5 transition-transform {stream.showAdvanced
														? 'rotate-90'
														: ''}"
													fill="none"
													stroke="currentColor"
													viewBox="0 0 24 24"
												>
													<path
														stroke-linecap="round"
														stroke-linejoin="round"
														stroke-width="2"
														d="M9 5l7 7-7 7"
													></path>
												</svg>
												Advanced settings
											</button>

											{#if stream.showAdvanced}
												<div
													transition:slide
													class="mt-4 space-y-6"
												>
													<!-- Initial sync configuration -->
													{#if stream.supportsInitialSync}
														<div
															class="border-t border-neutral-200 pt-4"
														>
															<h4
																class="text-sm font-medium text-neutral-700 mb-3"
															>
																Initial Sync
															</h4>
															<div class="space-y-3">
															<Radio
																name="initial-sync-{index}"
																value="limited"
																checked={stream.initialSyncType ===
																	"limited"}
																onchange={() =>
																	(stream.initialSyncType =
																		"limited")}
																label="Limited sync"
																description="Sync a specific number of days of historical data"
															/>

															{#if stream.initialSyncType === "limited"}
																<div
																	class="ml-7 space-y-3"
																>
																	<div>
																		<label
																			for="stream-{index}-lookback"
																			class="block text-sm font-medium text-neutral-700 mb-1"
																		>
																			Lookback
																			period
																			(days)
																		</label>
																		<Input
																			id="stream-{index}-lookback"
																			type="number"
																			bind:value={
																				stream.initialSyncDays
																			}
																			min={1}
																			max={365}
																			placeholder="90"
																		/>
																		<p class="text-xs text-neutral-500 mt-1">
																			How many days of past data to sync
																		</p>
																	</div>
																	
																	{#if stream.supportsFutureSync}
																		<div>
																			<label
																				for="stream-{index}-lookahead"
																				class="block text-sm font-medium text-neutral-700 mb-1"
																			>
																				Lookahead
																				period
																				(days)
																			</label>
																			<Input
																				id="stream-{index}-lookahead"
																				type="number"
																				bind:value={
																					stream.initialSyncDaysFuture
																				}
																				min={1}
																				max={365}
																				placeholder="30"
																			/>
																			<p class="text-xs text-neutral-500 mt-1">
																				How many days of future data to sync (e.g., upcoming calendar events)
																			</p>
																		</div>
																	{/if}
																</div>
															{/if}

															<Radio
																name="initial-sync-{index}"
																value="full"
																checked={stream.initialSyncType ===
																	"full"}
																onchange={() =>
																	(stream.initialSyncType =
																		"full")}
																label="Full sync"
																description="Sync all available historical data"
															/>
															</div>
														</div>
													{:else if stream.syncType === 'token'}
														<!-- Token-based sync info -->
														<div class="border-t border-neutral-200 pt-4">
															<div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
																<p class="text-sm text-blue-700">
																	This source uses sync tokens for efficient incremental updates. 
																	The initial sync will fetch all available data.
																</p>
															</div>
														</div>
													{:else if stream.syncType === 'none'}
														<!-- Real-time only sync info -->
														<div class="border-t border-neutral-200 pt-4">
															<p class="text-sm text-neutral-600">
																This stream only supports real-time data collection. 
																Historical data cannot be synced.
															</p>
														</div>
													{/if}

													<!-- Incremental sync configuration -->
													{#if stream.syncType !== 'none'}
														<div
															class="border-t border-neutral-200 pt-4"
														>
															<h4
																class="text-sm font-medium text-neutral-700 mb-3"
															>
																Incremental Sync
															</h4>
															<div>
																<label
																	for="stream-{index}-cron"
																	class="block text-sm font-medium text-neutral-700 mb-1"
																>
																	Sync Schedule
																	(Cron
																	Expression)
																</label>
																<div
																	class="relative"
																>
																	<Input
																		id="stream-{index}-cron"
																		type="text"
																		bind:value={
																			stream.syncSchedule
																		}
																		oninput={(
																			e,
																		) => {
																			const target =
																				e.currentTarget as HTMLInputElement;
																			updateSyncSchedule(
																				index,
																				target.value,
																			);
																		}}
																		placeholder="0 * * * *"
																		class={!stream.cronValid
																			? "border-red-500"
																			: ""}
																	/>
																	{#if stream.cronValid === false}
																		<p
																			class="text-sm text-red-500 mt-1"
																		>
																			Invalid
																			cron
																			expression
																		</p>
																	{:else}
																		<p
																			class="text-sm text-neutral-500 mt-1"
																		>
																			{stream.ingestionType ===
																			"push"
																				? "How often device should upload data"
																				: "How often to fetch new data"}
																		</p>
																	{/if}
																</div>
															</div>
														</div>
													{/if}
												</div>
											{/if}
										</div>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Actions -->
			<div class="flex justify-end gap-3">
				<Button
					text="Cancel"
					variant="danger"
					onclick={handleCancel}
					disabled={isSubmitting}
				/>

				{#if data.source.authType === "device_token"}
					<Button
						text={deviceConnected ? "Save & Start Syncing" : "Configure Streams"}
						variant="filled"
						onclick={handleDeviceSubmit}
						disabled={isSubmitting || !generatedToken}
						title={!generatedToken 
							? "Generate a device token first" 
							: !deviceConnected
								? "Configure which streams to sync"
								: "Save stream configuration and start syncing"}
					/>
				{:else if data.source.authType === "oauth2"}
					<Button
						text={data.source.isConnected ? "Save & Sync" : "Authenticate First"}
						variant={data.source.isConnected && allStepsComplete ? "filled" : "outline"}
						onclick={data.source.isConnected ? handleOAuthSubmit : () => {
							errorMessage = `Please authenticate with ${data.source.displayName} first`;
							const authSection = document.querySelector('[data-auth-section]');
							if (authSection) {
								authSection.scrollIntoView({ behavior: 'smooth', block: 'center' });
							}
						}}
						disabled={isSubmitting || !allStepsComplete}
						title={!data.source.isConnected 
							? "You must authenticate with " + data.source.company + " before saving" 
							: !basicInfoComplete 
								? "Please complete the basic information (name and description)"
								: !streamConfigs.some(s => s.enabled)
									? "Please enable at least one stream to sync"
									: "Save configuration and start syncing"}
					/>
				{:else}
					<Button
						text="Connect"
						variant="filled"
						onclick={() => console.log("Connect")}
						disabled={isSubmitting}
					/>
				{/if}
			</div>
		</div>
		<!-- Friendly image and description of the selected connection -->
		<div class="w-2/5">
			<div class="sticky top-6 space-y-6">
				<!-- Video/Image placeholder -->
				{#snippet videoSection()}
					{@const sourceVideo = getSourceVideo(data.source.name)}
					<div
						class="bg-gradient-to-br from-neutral-100 to-neutral-200 rounded-xl overflow-hidden border border-neutral-300"
						onmouseenter={() => (isVideoHovered = true)}
						onmouseleave={() => (isVideoHovered = false)}
					>
						{#if sourceVideo}
							<video
								class="w-full h-48 object-cover"
								loop
								muted
								playsinline
								bind:this={videoElement}
							>
								<source
									src={sourceVideo}
									type="video/webm"
								/>
							</video>
						{:else}
							<div class="h-48 flex items-center justify-center">
								<iconify-icon
									icon={data.source.icon}
								class="text-6xl text-neutral-400"
							></iconify-icon>
						</div>
					{/if}
				</div>
				{/snippet}
				{@render videoSection()}

				<!-- Overview -->
				<div
					class="bg-neutral-100 border border-neutral-200 rounded-xl p-6"
				>
					<h3 class="text-lg font-mono text-neutral-900 mb-4">
						Setup Steps
					</h3>

					<div class="space-y-4">
						<!-- Step 1: Basic Info -->
						<div class="flex gap-3">
							<div class={basicInfoComplete ? "text-green-600" : "text-neutral-400"}>
								{#if basicInfoComplete}
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
									</svg>
								{:else}
									<span class="block w-5 h-5 rounded-full border-2 border-current"></span>
								{/if}
							</div>
							<div>
								<p class="text-sm font-medium text-neutral-900">
									1. Basic Information
								</p>
								<p class="text-xs text-neutral-500 mt-0.5">
									Name and describe your connection
								</p>
							</div>
						</div>

						<!-- Step 2: Authentication -->
						<div class="flex gap-3">
							<div class={data.source.isConnected || data.source.connectionSuccessful || generatedToken ? "text-green-600" : "text-neutral-400"}>
								{#if data.source.isConnected || data.source.connectionSuccessful || generatedToken}
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
									</svg>
								{:else}
									<span class="block w-5 h-5 rounded-full border-2 border-current"></span>
								{/if}
							</div>
							<div>
								<p class="text-sm font-medium text-neutral-900">
									2. Authentication
								</p>
								<p class="text-xs text-neutral-500 mt-0.5">
									{#if data.source.authType === "oauth2"}
										Connect with {data.source.company}
									{:else if data.source.authType === "device_token"}
										Generate device token
									{:else}
										Complete authentication
									{/if}
								</p>
							</div>
						</div>

						<!-- Step 3: Configure Streams -->
						<div class="flex gap-3">
							<div class={streamConfigs.some(s => s.enabled) ? "text-green-600" : "text-neutral-400"}>
								{#if streamConfigs.some(s => s.enabled)}
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
									</svg>
								{:else}
									<span class="block w-5 h-5 rounded-full border-2 border-current"></span>
								{/if}
							</div>
							<div>
								<p class="text-sm font-medium text-neutral-900">
									3. Configure Streams
								</p>
								<p class="text-xs text-neutral-500 mt-0.5">
									Select data streams to sync
								</p>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- 
		DEVELOPMENT NOTES - Configuration-driven approach:
		
		All the form fields, validations, and UI elements in this page should ideally be driven by:
		
		1. source_configs table - Contains:
		   - Authentication type and OAuth configuration
		   - Platform, company, and device type metadata
		   - Default sync settings
		   - UI display information (icon, descriptions)
		
		2. stream_configs table - Contains:
		   - Available streams for each source
		   - Default sync schedules (cron expressions)
		   - Ingestion types (push/pull)
		   - Stream-specific settings and metadata
		
		3. signal_configs table - Contains:
		   - Signal types that can be extracted from streams
		   - Processing parameters
		   - Signal-specific configuration
		
		4. semantic_configs table - Contains:
		   - Semantic understanding and categorization
		   - Cross-stream signal correlation
		   - Higher-level activity detection
		
		Implementation approach:
		- The page.server.ts should fetch all necessary configs
		- The UI should dynamically generate form sections based on config
		- No hardcoded form fields - everything data-driven
		- Validation rules come from config schemas
		- Stream toggles and settings auto-generated from stream_configs
		- Initial sync options derived from stream ingestion types
		
		Benefits:
		- Adding new sources requires no frontend changes
		- Stream configurations automatically appear in UI
		- Consistent experience across all source types
		- Easy to maintain and extend
		-->
	</div>
</Page>
