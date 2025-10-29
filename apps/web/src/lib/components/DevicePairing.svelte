<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { Button } from "$lib";
	import * as api from "$lib/api/client";
	import type {
		PairingInitResponse,
		DeviceInfo,
	} from "$lib/types/device-pairing";

	interface Props {
		deviceType: string;
		deviceName: string;
		onSuccess: (sourceId: string, deviceInfo: DeviceInfo) => void;
		onCancel?: () => void;
	}

	let { deviceType, deviceName, onSuccess, onCancel }: Props = $props();

	// Pairing state
	let isInitiating = $state(false);
	let pairingData: PairingInitResponse | null = $state(null);
	let deviceInfo: DeviceInfo | null = $state(null);
	let error = $state<string | null>(null);
	let isPolling = $state(false);
	let hasTimedOut = $state(false);

	// Timer state
	let timeRemaining = $state(600); // 10 minutes in seconds
	let timerInterval: ReturnType<typeof setInterval> | null = null;
	let pollInterval: ReturnType<typeof setInterval> | null = null;

	// API endpoint state
	let apiEndpoint = $state("");
	let isLoadingEndpoint = $state(true);

	$effect(() => {
		if (typeof window !== "undefined") {
			// Fetch smart endpoint from server
			fetch('/api/app/server-info')
				.then(r => r.json())
				.then(data => {
					apiEndpoint = data.apiEndpoint;
					isLoadingEndpoint = false;
				})
				.catch(() => {
					// Fallback to origin-based detection
					apiEndpoint = `${window.location.origin}/api`;
					isLoadingEndpoint = false;
				});
		}
	});

	// Format time remaining as MM:SS
	function formatTime(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${mins}:${secs.toString().padStart(2, "0")}`;
	}

	// Get urgency color based on time remaining
	function getUrgencyColor(seconds: number): string {
		if (seconds > 300) return "text-neutral-700"; // > 5 min
		if (seconds > 120) return "text-yellow-700"; // > 2 min
		return "text-red-700"; // < 2 min
	}

	// Copy code to clipboard
	async function copyCode() {
		if (!pairingData) return;

		try {
			await navigator.clipboard.writeText(pairingData.code);
			// Could add a toast notification here
		} catch (err) {
			console.error("Failed to copy code:", err);
		}
	}

	// Copy API endpoint to clipboard
	async function copyEndpoint() {
		try {
			await navigator.clipboard.writeText(apiEndpoint);
			// Could add a toast notification here
		} catch (err) {
			console.error("Failed to copy endpoint:", err);
		}
	}

	// Poll for pairing completion
	async function checkPairingStatus() {
		if (!pairingData || deviceInfo) return;

		try {
			const status = await api.getPairingStatus(pairingData.source_id);

			if (status.status === "active") {
				// Pairing completed!
				deviceInfo = status.device_info;
				stopPolling();
				stopTimer();
				onSuccess(pairingData.source_id, status.device_info);
			} else if (status.status === "revoked") {
				// Pairing was cancelled
				error = "Pairing was cancelled";
				stopPolling();
				stopTimer();
			}
			// If still pending, continue polling
		} catch (err) {
			console.error("Failed to check pairing status:", err);
			// Don't stop polling on network errors, just log
		}
	}

	// Start polling for pairing completion
	function startPolling() {
		if (pollInterval) return;

		isPolling = true;
		// Check immediately
		checkPairingStatus();
		// Then poll every 2 seconds
		pollInterval = setInterval(checkPairingStatus, 2000);
	}

	// Stop polling
	function stopPolling() {
		if (pollInterval) {
			clearInterval(pollInterval);
			pollInterval = null;
		}
		isPolling = false;
	}

	// Start countdown timer
	function startTimer() {
		if (timerInterval || !pairingData) return;

		// Calculate time remaining based on expires_at
		const expiresAt = new Date(pairingData.expires_at);
		const now = new Date();
		const secondsRemaining = Math.floor(
			(expiresAt.getTime() - now.getTime()) / 1000,
		);

		timeRemaining = Math.max(0, secondsRemaining);

		timerInterval = setInterval(() => {
			timeRemaining--;

			if (timeRemaining <= 0) {
				hasTimedOut = true;
				stopTimer();
				stopPolling();
			}
		}, 1000);
	}

	// Stop timer
	function stopTimer() {
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	// Initiate pairing
	async function initiate() {
		isInitiating = true;
		error = null;

		try {
			const response = await api.initiatePairing(deviceType, deviceName);
			pairingData = response;

			// Start polling and timer
			startPolling();
			startTimer();
		} catch (err) {
			error = err instanceof Error ? err.message : "Failed to initiate pairing";
		} finally {
			isInitiating = false;
		}
	}

	// Retry after timeout
	function retry() {
		hasTimedOut = false;
		pairingData = null;
		deviceInfo = null;
		error = null;
		timeRemaining = 600;
		initiate();
	}

	// Handle cancel
	function handleCancel() {
		stopPolling();
		stopTimer();
		onCancel?.();
	}

	// Auto-initiate on mount
	onMount(() => {
		initiate();
	});

	// Cleanup on unmount
	onDestroy(() => {
		stopPolling();
		stopTimer();
	});
</script>

<div class="space-y-8">
	{#if error}
		<div class="p-4 border border-red-300 bg-red-50">
			<p class="text-sm font-serif text-red-900">{error}</p>
		</div>
	{/if}

	{#if isInitiating}
		<div class="text-center py-8">
			<p class="text-neutral-600">Generating pairing code...</p>
		</div>
	{:else if hasTimedOut}
		<div class="text-center py-8 space-y-6">
			<div>
				<p class="text-xl font-serif text-red-700 mb-2">
					Pairing Code Expired
				</p>
				<p class="text-neutral-600">
					The pairing code expired after 10 minutes. No device
					connected.
				</p>
			</div>
			<div class="flex gap-3 justify-center">
				<Button onclick={retry}>Try Again</Button>
				{#if onCancel}
					<Button variant="ghost" onclick={handleCancel}>
						Cancel
					</Button>
				{/if}
			</div>
		</div>
	{:else if deviceInfo}
		<!-- Success state -->
		<div class="text-center py-8 space-y-6">
			<div>
				<div class="text-4xl mb-4">✅</div>
				<p class="text-xl font-serif text-green-700 mb-2">
					Device Paired Successfully!
				</p>
			</div>

			<div
				class="inline-block text-left border border-neutral-200 p-6 space-y-3"
			>
				<h3 class="font-serif text-neutral-900 mb-4">Device Details</h3>
				<div class="space-y-2 text-sm">
					<div class="flex justify-between gap-8">
						<span class="text-neutral-600">Name:</span>
						<span class="text-neutral-900 font-medium">
							{deviceInfo.device_name}
						</span>
					</div>
					<div class="flex justify-between gap-8">
						<span class="text-neutral-600">Model:</span>
						<span class="text-neutral-900 font-medium">
							{deviceInfo.device_model}
						</span>
					</div>
					<div class="flex justify-between gap-8">
						<span class="text-neutral-600">OS:</span>
						<span class="text-neutral-900 font-medium">
							{deviceInfo.os_version}
						</span>
					</div>
					{#if deviceInfo.app_version}
						<div class="flex justify-between gap-8">
							<span class="text-neutral-600">App:</span>
							<span class="text-neutral-900 font-medium">
								Ariata v{deviceInfo.app_version}
							</span>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{:else if pairingData}
		<!-- Waiting for device to connect -->
		<div class="space-y-8">
			<div class="text-center">
				<p class="text-neutral-900 font-serif text-lg mb-2">
					Open your device app and enter these details:
				</p>
				<p class="text-neutral-600 text-sm">
					Waiting for device to connect...
				</p>
			</div>

			<!-- API Endpoint Display -->
			<div class="space-y-4">
				<div>
					<label class="block text-sm font-medium text-neutral-700 mb-2">
						API Endpoint
					</label>
					<div
						class="border-2 border-neutral-300 bg-neutral-50 py-4 px-6 text-center"
					>
						<div
							class="text-lg font-mono text-neutral-900 break-all select-all"
						>
							{#if isLoadingEndpoint}
								<span class="text-neutral-500">Loading...</span>
							{:else}
								{apiEndpoint}
							{/if}
						</div>
						<button
							onclick={copyEndpoint}
							disabled={isLoadingEndpoint}
							class="mt-3 text-sm text-neutral-600 hover:text-neutral-900 underline disabled:opacity-50 disabled:cursor-not-allowed"
						>
							Copy Endpoint
						</button>
					</div>
				</div>

				<!-- Pairing Code Display -->
				<div>
					<label class="block text-sm font-medium text-neutral-700 mb-2">
						Pairing Code
					</label>
					<div
						class="border-2 border-neutral-300 bg-neutral-50 py-8 px-6 text-center"
					>
						<div
							class="text-6xl font-mono font-bold tracking-[0.5em] text-neutral-900 select-all"
						>
							{pairingData.code}
						</div>
						<button
							onclick={copyCode}
							class="mt-4 text-sm text-neutral-600 hover:text-neutral-900 underline"
						>
							Copy Code
						</button>
					</div>

					<!-- Countdown timer -->
					<div class="mt-4 text-center">
						<p class={`text-sm font-medium ${getUrgencyColor(timeRemaining)}`}>
							Code expires in {formatTime(timeRemaining)}
						</p>
					</div>
				</div>
			</div>

			<!-- Instructions -->
			<div class="border-t border-neutral-200 pt-6 space-y-4">
				<h3 class="font-serif text-neutral-900">Instructions:</h3>
				<ol class="space-y-2 text-sm text-neutral-600 leading-relaxed">
					<li>1. Open the Ariata app on your device</li>
					<li>2. Navigate to Settings → Sync</li>
					<li>3. Tap "Connect to Server"</li>
					<li>4. Enter the <strong class="text-neutral-900">API Endpoint</strong> above</li>
					<li>5. Enter the <strong class="text-neutral-900">Pairing Code</strong> above</li>
					<li>6. Wait for confirmation</li>
				</ol>
			</div>

			<!-- Cancel button -->
			{#if onCancel}
				<div class="pt-4 border-t border-neutral-200 text-center">
					<Button variant="ghost" onclick={handleCancel}>
						Cancel Pairing
					</Button>
				</div>
			{/if}

			<!-- Polling indicator -->
			{#if isPolling}
				<div class="flex items-center justify-center gap-2 text-sm text-neutral-500">
					<div class="animate-spin h-4 w-4 border-2 border-neutral-400 border-t-transparent rounded-full"></div>
					<span>Checking for connection...</span>
				</div>
			{/if}
		</div>
	{/if}
</div>
