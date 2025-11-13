<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		label: string;
	}

	let { label }: Props = $props();

	// Animated dots that cycle: . → .. → ...
	let dots = $state('');

	onMount(() => {
		const interval = setInterval(() => {
			dots = dots.length >= 3 ? '' : dots + '.';
		}, 800);

		return () => clearInterval(interval);
	});
</script>

<div class="thinking-indicator flex items-center gap-2 py-3">
	<div class="thinking-text">
		<span class="thinking-shimmer font-serif font-light text-base">
			{label}{dots}
		</span>
	</div>
</div>

<style>
	.thinking-indicator {
		opacity: 0;
		animation: fadeIn 0.3s ease-out forwards;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.thinking-shimmer {
		background: linear-gradient(
			90deg,
			var(--color-stone-600) 0%,
			var(--color-stone-400) 20%,
			var(--color-navy) 40%,
			var(--color-stone-400) 60%,
			var(--color-stone-600) 100%
		);
		background-size: 2000px 100%;
		animation: shimmer 4s infinite linear;
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	@keyframes shimmer {
		0% {
			background-position: -1000px 0;
		}
		100% {
			background-position: 1000px 0;
		}
	}

	/* Respect reduced motion preferences */
	@media (prefers-reduced-motion: reduce) {
		.thinking-indicator {
			animation: none;
			opacity: 1;
		}

		.thinking-shimmer {
			animation: none;
			background: var(--color-stone-600);
			-webkit-background-clip: unset;
			-webkit-text-fill-color: unset;
			background-clip: unset;
		}
	}
</style>
