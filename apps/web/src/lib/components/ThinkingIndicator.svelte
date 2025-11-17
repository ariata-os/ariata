<script lang="ts">
	import { onMount } from "svelte";

	interface Props {
		label: string;
	}

	let { label }: Props = $props();

	// Animated dots that cycle: . → .. → ...
	let dots = $state("");

	onMount(() => {
		const interval = setInterval(() => {
			dots = dots.length >= 3 ? "" : dots + ".";
		}, 800);

		return () => clearInterval(interval);
	});
</script>

<div class="thinking-indicator py-2 pl-3.5">
	<div class="therefore-spinner">
		<div class="dot dot-1"></div>
		<div class="dot dot-2"></div>
		<div class="dot dot-3"></div>
	</div>
	<div class="thinking-text">
		<span
			class="thinking-text-content font-serif font-light text-base text-blue"
		>
			{label}{dots}
		</span>
	</div>
</div>

<style>
	.thinking-indicator {
		position: relative;
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

	.therefore-spinner {
		position: absolute;
		left: -0.5rem;
		top: 50%;
		transform: translateY(-50%);
		width: 12px;
		height: 12px;
		animation: rotate 2s ease-in-out infinite;
	}

	.dot {
		position: absolute;
		width: 4px;
		height: 4px;
		background-color: var(--color-blue);
		border-radius: 50%;
	}

	/* Top dot */
	.dot-1 {
		top: 0;
		left: 50%;
		transform: translateX(-50%);
	}

	/* Bottom left dot */
	.dot-2 {
		bottom: 0;
		left: 0;
	}

	/* Bottom right dot */
	.dot-3 {
		bottom: 0;
		right: 0;
	}

	@keyframes rotate {
		0% {
			transform: translateY(-50%) rotate(0deg) scale(1);
		}
		25% {
			transform: translateY(-50%) rotate(90deg) scale(0.75);
		}
		50% {
			transform: translateY(-50%) rotate(180deg) scale(1);
		}
		75% {
			transform: translateY(-50%) rotate(270deg) scale(0.75);
		}
		100% {
			transform: translateY(-50%) rotate(360deg) scale(1);
		}
	}

	/* Respect reduced motion preferences */
	@media (prefers-reduced-motion: reduce) {
		.thinking-indicator {
			animation: none;
			opacity: 1;
		}

		.therefore-spinner {
			animation: none;
		}

		.dot {
			animation: none;
		}
	}
</style>
