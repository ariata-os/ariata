<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let deviceId = $derived($page.url.searchParams.get('device_id') || '');
	let deviceName = $state('');
	let pairing = $state(false);
	let paired = $state(false);
	let error = $state('');

	// Get computer name on mount
	onMount(async () => {
		if (!deviceId) {
			error = 'Missing device_id parameter';
			return;
		}

		// Check if device already exists
		const checkRes = await fetch(`/api/pairing/check?device_id=${deviceId}`);
		if (checkRes.ok) {
			const data = await checkRes.json();
			if (data.exists) {
				deviceName = data.deviceName || 'Your Mac';
			}
		}
	});

	async function confirmPairing() {
		pairing = true;
		error = '';

		try {
			const res = await fetch('/api/pairing/confirm', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					device_id: deviceId,
					device_name: deviceName || 'My Mac',
					device_type: 'mac'
				})
			});

			if (!res.ok) {
				const data = await res.json();
				throw new Error(data.error || 'Pairing failed');
			}

			paired = true;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Pairing failed';
		} finally {
			pairing = false;
		}
	}
</script>

<div class="min-h-screen flex items-center justify-center bg-gray-50 p-4">
	<div class="max-w-md w-full bg-white rounded-lg shadow-lg p-8">
		{#if !deviceId}
			<div class="text-center">
				<div class="text-red-500 text-4xl mb-4">⚠️</div>
				<h1 class="text-2xl font-bold text-gray-900 mb-2">Invalid Request</h1>
				<p class="text-gray-600">Missing device identifier. Please try again from your Mac app.</p>
			</div>
		{:else if paired}
			<div class="text-center">
				<div class="text-green-500 text-6xl mb-4">✅</div>
				<h1 class="text-2xl font-bold text-gray-900 mb-2">Pairing Successful!</h1>
				<p class="text-gray-600 mb-6">
					Your Mac is now connected to Ariata. You can close this window and return to your Mac.
				</p>
				<div class="bg-green-50 border border-green-200 rounded-lg p-4">
					<p class="text-sm text-green-800">
						<strong>{deviceName || 'Your Mac'}</strong> is now syncing data.
					</p>
				</div>
			</div>
		{:else}
			<div class="text-center mb-6">
				<div class="text-blue-500 text-4xl mb-4">💻</div>
				<h1 class="text-2xl font-bold text-gray-900 mb-2">Pair Your Mac</h1>
				<p class="text-gray-600 mb-4">Confirm pairing for this device:</p>

				<div class="bg-gray-50 border border-gray-200 rounded-lg p-4 mb-6 text-left">
					<div class="text-sm text-gray-500 mb-2">Device ID</div>
					<div class="font-mono text-xs text-gray-700 break-all">{deviceId}</div>
				</div>

				<div class="mb-6">
					<label for="device-name" class="block text-sm font-medium text-gray-700 mb-2 text-left">
						Device Name (optional)
					</label>
					<input
						id="device-name"
						type="text"
						bind:value={deviceName}
						placeholder="My MacBook Pro"
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
					/>
				</div>

				{#if error}
					<div class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3">
						<p class="text-sm text-red-800">{error}</p>
					</div>
				{/if}

				<button
					onclick={confirmPairing}
					disabled={pairing}
					class="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-semibold py-3 px-6 rounded-lg transition-colors"
				>
					{pairing ? 'Pairing...' : 'Confirm Pairing'}
				</button>

				<p class="text-xs text-gray-500 mt-4">
					By pairing this device, you authorize it to sync your personal data to Ariata.
				</p>
			</div>
		{/if}
	</div>
</div>
