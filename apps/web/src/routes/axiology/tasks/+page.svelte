<script lang="ts">
	import { onMount } from 'svelte';
	import { Page, TagInput } from '$lib';
	import type { Task, CreateTaskRequest } from '$lib/types/actions';

	let tasks: Task[] = $state([]);
	let loading = $state(true);
	let creating = $state(false);
	let showCreateForm = $state(false);

	let newTask: CreateTaskRequest = $state({
		title: '',
		description: '',
		tags: [] as string[]
	});

	const API_BASE = '/api';

	async function loadTasks() {
		loading = true;
		try {
			const response = await fetch(`${API_BASE}/praxis/tasks`);
			if (response.ok) {
				tasks = await response.json();
			}
		} catch (error) {
			console.error('Failed to load tasks:', error);
		} finally {
			loading = false;
		}
	}

	async function createTask() {
		if (!newTask.title.trim()) return;

		creating = true;
		try {
			const response = await fetch(`${API_BASE}/praxis/tasks`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(newTask)
			});

			if (response.ok) {
				try {
					const created = await response.json();
					tasks = [created, ...tasks];
				} catch (jsonError) {
					console.error('JSON parsing failed, reloading tasks:', jsonError);
					await loadTasks();
				}
				newTask = { title: '', description: '', tags: [] };
				showCreateForm = false;
			} else {
				console.error('Failed to create task:', response.status, response.statusText);
			}
		} catch (error) {
			console.error('Failed to create task:', error);
		} finally {
			creating = false;
		}
	}

	onMount(() => {
		loadTasks();
	});
</script>

<Page>
	<div class="max-w-4xl">
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Tasks</h1>
			<p class="text-neutral-600">Daily and weekly completable items</p>
		</div>

		<!-- Create Button / Form -->
		{#if !showCreateForm}
			<button
				type="button"
				onclick={() => (showCreateForm = true)}
				class="mb-6 border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors"
			>
				New Task
			</button>
		{:else}
			<div class="mb-6 border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-serif font-medium text-neutral-900 mb-4">Create New Task</h2>

				<div class="space-y-4">
					<div>
						<label for="title" class="block text-sm text-neutral-700 mb-1">Title</label>
						<input
							id="title"
							type="text"
							bind:value={newTask.title}
							placeholder="e.g., Read more books"
							class="w-full px-3 py-2 border border-neutral-300 rounded-md"
						/>
					</div>

					<div>
						<label for="description" class="block text-sm text-neutral-700 mb-1">
							Description
						</label>
						<textarea
							id="description"
							bind:value={newTask.description}
							placeholder="Why is this important to you?"
							rows="3"
							class="w-full px-3 py-2 border border-neutral-300 rounded-md"
						></textarea>
					</div>

					<div>
						<label for="tags" class="block text-sm text-neutral-700 mb-1">
							Tags (optional)
						</label>
						<TagInput bind:tags={newTask.tags!} />
					</div>

					<div class="flex gap-3 pt-2">
						<button
							type="button"
							onclick={createTask}
							disabled={creating || !newTask.title.trim()}
							class="border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors disabled:opacity-50"
						>
							{creating ? 'Creating...' : 'Create'}
						</button>
						<button
							type="button"
							onclick={() => {
								showCreateForm = false;
								newTask = { title: '', description: '', tags: [] };
							}}
							class="border border-neutral-300 rounded-lg px-4 py-2 hover:border-neutral-400 transition-colors"
						>
							Cancel
						</button>
					</div>
				</div>
			</div>
		{/if}

		<!-- Tasks List -->
		{#if loading}
			<div class="text-neutral-500">Loading...</div>
		{:else if tasks.length === 0}
			<div class="border border-neutral-200 rounded-lg p-12 text-center">
				<p class="text-neutral-500">No tasks yet</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each tasks as task}
					<div class="border border-neutral-200 rounded-lg p-4">
						<h3 class="text-lg font-serif font-medium text-neutral-900 mb-1">{task.title}</h3>
						{#if task.description}
							<p class="text-neutral-600 text-sm mb-2">{task.description}</p>
						{/if}
						<div class="flex gap-2 text-xs text-neutral-500">
							{#if task.tags && task.tags.length > 0}
								{#each task.tags as tag}
									<span class="capitalize">{tag}</span>
								{/each}
							{/if}
							{#if task.status && task.status !== 'active'}
								<span>· {task.status.replace('_', ' ')}</span>
							{/if}
							{#if task.progress_percent}
								<span>· {task.progress_percent}%</span>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</Page>
