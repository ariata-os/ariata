<script lang="ts">
	import { Page } from "$lib";
	import "iconify-icon";
	import { onMount } from "svelte";

	let loading = true;
	let saving = false;
	let saveSuccess = false;

	let userName = '';

	onMount(async () => {
		try {
			const response = await fetch('/api/preferences');
			if (response.ok) {
				const prefs = await response.json();
				userName = prefs.user_name || '';
			}
		} catch (error) {
			console.error('Failed to load preferences:', error);
		} finally {
			loading = false;
		}
	});

	async function savePreferences() {
		saving = true;
		saveSuccess = false;
		try {
			const response = await fetch('/api/preferences', {
				method: 'PATCH',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					user_name: userName
				})
			});

			if (response.ok) {
				saveSuccess = true;
				setTimeout(() => {
					saveSuccess = false;
				}, 3000);
			} else {
				throw new Error('Failed to save preferences');
			}
		} catch (error) {
			console.error('Failed to save preferences:', error);
			alert('Failed to save preferences. Please try again.');
		} finally {
			saving = false;
		}
	}
</script>

<Page>
	<div class="max-w-3xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">
				Account
			</h1>
			<p class="text-neutral-600">
				Manage your account settings and preferences
			</p>
		</div>

		<!-- Account Settings Content -->
		<div class="space-y-6">
			<!-- Communication Preferences Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					Communication Preferences
				</h2>
				<p class="text-sm text-neutral-600 mb-6">
					Customize how the assistant communicates with you.
				</p>

				{#if loading}
					<div class="text-center py-4 text-neutral-500">
						Loading preferences...
					</div>
				{:else}
					<div class="space-y-6">
						<!-- Name -->
						<div>
							<label
								for="userName"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Preferred Name (Optional)
							</label>
							<input
								type="text"
								id="userName"
								bind:value={userName}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="How should the assistant address you?"
							/>
							<p class="text-xs text-neutral-500 mt-1">
								Used for personalization in conversations
							</p>
						</div>

						<!-- Save Button -->
						<div class="flex items-center gap-3 pt-2">
							<button
								on:click={savePreferences}
								disabled={saving}
								class="px-4 py-2 bg-neutral-900 text-white rounded-md hover:bg-neutral-800 focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								{saving ? 'Saving...' : 'Save Preferences'}
							</button>
							{#if saveSuccess}
								<span class="text-sm text-green-600 flex items-center gap-1">
									<iconify-icon icon="mdi:check-circle" width="16"></iconify-icon>
									Saved successfully
								</span>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<!-- Other Preferences Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					Other Preferences
				</h2>
				<div class="space-y-4">
					<div class="flex items-center justify-between">
						<div>
							<p class="text-sm font-medium text-neutral-900">
								Theme
							</p>
							<p class="text-xs text-neutral-500">
								Customize your interface appearance
							</p>
						</div>
						<select
							class="px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							disabled
						>
							<option>Light</option>
							<option>Dark</option>
							<option>System</option>
						</select>
					</div>
					<p class="text-xs text-neutral-500">
						Theme preferences coming soon
					</p>
				</div>
			</div>

			<!-- About Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					About
				</h2>
				<div class="space-y-2 text-sm text-neutral-600">
					<div class="flex items-center justify-between">
						<span>Version</span>
						<span class="font-medium text-neutral-900">1.0.0</span>
					</div>
				</div>
			</div>
		</div>
	</div>
</Page>
