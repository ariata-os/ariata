<script lang="ts">
	import { fade } from "svelte/transition";
	import { getEnabledAgents, type AgentUIMetadata } from "$lib/config/agents";
	import UniversalSelect from "./UniversalSelect.svelte";

	interface Props {
		value: string; // agentId: 'auto' | 'analytics' | 'research' | 'general' | 'action'
		disabled?: boolean;
	}

	let {
		value = $bindable(),
		disabled = false,
	}: Props = $props();

	// Add 'auto' option to the agent list
	const autoAgent = {
		id: 'auto',
		name: 'Auto',
		description: 'Automatically selects the best agent for your query',
		color: '#6b7280', // gray
		icon: 'ri:compass-line',
		enabled: true
	};

	const agents = [autoAgent, ...getEnabledAgents()];

	function getAgentKey(agent: typeof autoAgent) {
		return agent.id;
	}

	function getAgentValue(agent: typeof autoAgent) {
		return agent.id;
	}
</script>

<UniversalSelect
	bind:value
	items={agents}
	{disabled}
	width="w-72"
	getKey={getAgentKey}
	getValue={getAgentValue}
>
	{#snippet trigger(agent: typeof autoAgent, isDisabled: boolean, open: boolean)}
		<div
			class="flex items-center gap-2"
			class:px-3={!isDisabled}
			class:py-1.5={!isDisabled}
			class:w-8={isDisabled}
			class:h-8={isDisabled}
			class:justify-center={isDisabled}
			aria-label="Select agent"
		>
			{#if isDisabled}
				<iconify-icon
					icon={agent.icon}
					style="color: {agent.color}"
					width="16"
					in:fade={{ duration: 200 }}
				></iconify-icon>
			{:else}
				<iconify-icon
					icon={agent.icon}
					style="color: {agent.color}"
					width="14"
				></iconify-icon>
				<span class="text-neutral-700">{agent.name}</span>
				<iconify-icon
					icon="ri:arrow-down-s-line"
					class="text-neutral-400 transition-transform duration-200"
					class:rotate-180={open}
					width="16"
				></iconify-icon>
			{/if}
		</div>
	{/snippet}

	{#snippet item(agent: typeof autoAgent, isSelected: boolean)}
		<div class="px-4 py-3">
			<div class="flex items-start justify-between gap-3">
				<div class="flex items-start gap-3">
					<div class="mt-0.5">
						<iconify-icon
							icon={agent.icon}
							style="color: {agent.color}"
							width="18"
						></iconify-icon>
					</div>
					<div class="flex-1 min-w-0">
						<div class="flex items-center gap-2">
							<span class="text-sm font-medium text-neutral-900">
								{agent.name}
							</span>
							{#if agent.id === 'auto'}
								<span class="text-xs text-neutral-500">(recommended)</span>
							{/if}
						</div>
						<p class="text-xs text-neutral-600 mt-0.5 line-clamp-2">
							{agent.description}
						</p>
					</div>
				</div>
				{#if isSelected}
					<iconify-icon
						icon="ri:check-line"
						class="text-blue-600 flex-shrink-0 mt-0.5"
						width="16"
					></iconify-icon>
				{/if}
			</div>
		</div>
	{/snippet}
</UniversalSelect>

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
