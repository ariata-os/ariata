<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	// Redirect to database settings by default
	onMount(() => {
		goto('/settings/database', { replaceState: true });
	});
</script>

<div class="flex items-center justify-center h-full">
	<p class="text-gray-500">Redirecting to settings...</p>
</div>