<script lang="ts">
	/**
	 * Context indicator showing conversation token usage
	 * Can be configured to always show or only when threshold is exceeded
	 */
	interface Props {
		cumulativeTokens: number;
		contextWindow: number | null;
		alwaysVisible?: boolean;
		showThreshold?: number;
	}

	let {
		cumulativeTokens = 0,
		contextWindow = null,
		alwaysVisible = false,
		showThreshold = 70,
	}: Props = $props();

	// Calculate percentage used
	const percentageUsed = $derived(
		contextWindow && contextWindow > 0
			? (cumulativeTokens / contextWindow) * 100
			: 0,
	);

	// Show based on alwaysVisible setting or threshold
	const shouldShow = $derived(
		alwaysVisible || percentageUsed > showThreshold,
	);

	// Calculate percentage remaining
	const percentageRemaining = $derived(100 - percentageUsed);

	// Determine color based on usage
	const color = $derived.by(() => {
		if (percentageUsed >= 90) return "text-red-600 bg-red-50";
		if (percentageUsed >= 80) return "text-orange-600 bg-orange-50";
		if (percentageUsed >= showThreshold)
			return "text-yellow-600 bg-yellow-50";
		// Neutral gray when below threshold but alwaysVisible is true
		return "text-neutral-600 bg-neutral-100";
	});

	// Format numbers for display
	function formatNumber(n: number): string {
		if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
		return Math.round(n).toString();
	}
</script>

{#if shouldShow && contextWindow}
	<div
		class="flex items-center gap-1.5 px-2 py-1 rounded-md {color} text-xs font-medium transition-colors"
		title="{formatNumber(cumulativeTokens)} / {formatNumber(
			contextWindow,
		)} tokens used ({percentageUsed.toFixed(1)}% of context)"
	>
		<iconify-icon icon="ri:file-text-line" width="12"></iconify-icon>
		<span>{Math.round(percentageRemaining)}% left</span>
	</div>
{/if}

<style>
	/* Smooth transitions */
	div {
		transition: all 0.2s ease-in-out;
	}
</style>
