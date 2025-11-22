<script lang="ts">
	export let label: string;
	export let value: string | number;
	export let sublabel: string = '';
</script>

<div class="border border-neutral-200 rounded-lg p-4 bg-white">
	<div class="text-sm text-neutral-500 mb-1">{label}</div>
	<div class="text-2xl font-semibold text-neutral-900">{value}</div>
	{#if sublabel}
		<div class="text-xs text-neutral-400 mt-1">{sublabel}</div>
	{/if}
</div>
