<script lang="ts">
	import { onMount } from 'svelte';
	import { Page } from '$lib';
	import type { Temperament, CreateSimpleRequest } from '$lib/types/axiology';

	let items: Temperament[] = $state([]);
	let loading = $state(true);
	let creating = $state(false);
	let showCreateForm = $state(false);

	let newItem: CreateSimpleRequest = $state({
		title: '',
		description: ''
	});

	const API_BASE = '/api';

	async function loadItems() {
		loading = true;
		try {
			const response = await fetch(`${API_BASE}/axiology/temperaments`);
			if (response.ok) {
				items = await response.json();
			}
		} catch (error) {
			console.error('Failed to load temperaments:', error);
		} finally {
			loading = false;
		}
	}

	async function createItem() {
		if (!newItem.title.trim()) return;

		creating = true;
		try {
			const response = await fetch(`${API_BASE}/axiology/temperaments`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(newItem)
			});

			if (response.ok) {
				const created = await response.json();
				items = [created, ...items];
				newItem = { title: '', description: '' };
				showCreateForm = false;
			}
		} catch (error) {
			console.error('Failed to create temperament:', error);
		} finally {
			creating = false;
		}
	}

	onMount(() => {
		loadItems();
	});
</script>

<Page>
	<div class="max-w-4xl">
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Temperaments</h1>
			<p class="text-neutral-600">Personality patterns and natural dispositions</p>
		</div>

		<!-- Create Button / Form -->
		{#if !showCreateForm}
			<button
				type="button"
				onclick={() => (showCreateForm = true)}
				class="mb-6 border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors"
			>
				New Temperament
			</button>
		{:else}
			<div class="mb-6 border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-serif font-medium text-neutral-900 mb-4">Create New Temperament</h2>

				<div class="space-y-4">
					<div>
						<label for="title" class="block text-sm text-neutral-700 mb-1">Title</label>
						<input
							id="title"
							type="text"
							bind:value={newItem.title}
							placeholder="e.g., Analytical, Enthusiastic, Thoughtful"
							class="w-full px-3 py-2 border border-neutral-300 rounded-md"
						/>
					</div>

					<div>
						<label for="description" class="block text-sm text-neutral-700 mb-1">
							Description
						</label>
						<textarea
							id="description"
							bind:value={newItem.description}
							placeholder="Describe this temperament pattern..."
							rows="3"
							class="w-full px-3 py-2 border border-neutral-300 rounded-md"
						></textarea>
					</div>

					<div class="flex gap-3 pt-2">
						<button
							type="button"
							onclick={createItem}
							disabled={creating || !newItem.title.trim()}
							class="border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors disabled:opacity-50"
						>
							{creating ? 'Creating...' : 'Create'}
						</button>
						<button
							type="button"
							onclick={() => {
								showCreateForm = false;
								newItem = { title: '', description: '' };
							}}
							class="border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors"
						>
							Cancel
						</button>
					</div>
				</div>
			</div>
		{/if}

		<!-- Items List -->
		{#if loading}
			<div class="text-neutral-500">Loading...</div>
		{:else if items.length === 0}
			<div class="border border-neutral-200 rounded-lg p-12 text-center">
				<p class="text-neutral-500">No temperaments yet</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each items as item}
					<div class="border border-neutral-200 rounded-lg p-4">
						<h3 class="text-lg font-serif font-medium text-neutral-900 mb-1">{item.title}</h3>
						{#if item.description}
							<p class="text-neutral-600 text-sm">{item.description}</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
</Page>
