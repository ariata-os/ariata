<script lang="ts">
	import { slide } from 'svelte/transition';

	let { text }: { text: string } = $props();

	let isExpanded = $state(false);
	let lineCount = $state(0);
	let textContainer: HTMLDivElement | undefined = $state();
	const LINE_THRESHOLD = 8;

	// Calculate line count after mount
	$effect(() => {
		if (textContainer) {
			const lineHeight = parseFloat(getComputedStyle(textContainer).lineHeight);
			const height = textContainer.scrollHeight;
			lineCount = Math.round(height / lineHeight);
		}
	});

	const shouldTruncate = $derived(lineCount > LINE_THRESHOLD);
	const maxHeight = $derived(!isExpanded && shouldTruncate ? `${LINE_THRESHOLD * 1.5}rem` : 'none');
</script>

<div class="user-message-container">
	<div
		bind:this={textContainer}
		class="text-base whitespace-pre-wrap font-serif text-blue"
		style="max-height: {maxHeight}; overflow: hidden;"
	>
		{text}
	</div>

	{#if shouldTruncate}
		<button
			onclick={() => (isExpanded = !isExpanded)}
			class="font-serif text-sm text-blue bg-stone-200 rounded px-2 py-1 mt-2 transition-colors hover:bg-blue/15 cursor-pointer"
			transition:slide={{ duration: 200 }}
		>
			{isExpanded ? '↑ Show less' : '↓ Show more'}
		</button>
	{/if}
</div>
