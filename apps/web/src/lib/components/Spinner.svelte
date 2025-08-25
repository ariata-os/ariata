<script lang="ts">
	interface Props {
		size?: 'sm' | 'md' | 'lg';
		class?: string;
	}

	let { size = 'md', class: className = '' }: Props = $props();

	const sizeClasses = {
		sm: 'w-4 h-4',
		md: 'w-8 h-8',
		lg: 'w-12 h-12'
	};
</script>

<div
	class="animate-spin rounded-full border-2 border-neutral-200 border-t-neutral-600 {sizeClasses[
		size
	]} {className}"
	role="status"
	aria-label="Loading"
>
	<span class="sr-only">Loading...</span>
</div>

<style>
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.animate-spin {
		animation: spin 1s linear infinite;
	}
</style>