<script lang="ts">
	import { fly, fade } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import { getEnabledAgents, type AgentUIMetadata } from "$lib/config/agents";

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

	// Get current agent metadata
	const currentAgent = $derived(
		agents.find(a => a.id === value) || autoAgent
	);

	let open = $state(false);
	let buttonElement: HTMLButtonElement;
	let dropdownElement: HTMLDivElement;

	function selectAgent(agentId: string) {
		console.log('[AgentPicker] selectAgent called:', {
			oldAgent: value,
			newAgent: agentId,
			timestamp: Date.now()
		});
		value = agentId;
		console.log('[AgentPicker] value updated to:', value);
		open = false;
	}

	function toggleDropdown() {
		if (!disabled) {
			open = !open;
		}
	}

	function handleClickOutside(e: MouseEvent) {
		if (
			!buttonElement?.contains(e.target as Node) &&
			!dropdownElement?.contains(e.target as Node)
		) {
			open = false;
		}
	}

	$effect(() => {
		if (open) {
			document.addEventListener("mousedown", handleClickOutside);
			return () => {
				document.removeEventListener("mousedown", handleClickOutside);
			};
		}
	});
</script>

<div class="relative">
	<button
		bind:this={buttonElement}
		type="button"
		onclick={toggleDropdown}
		{disabled}
		class="flex cursor-pointer items-center gap-2 rounded-full bg-white text-sm transition-all duration-200"
		class:opacity-50={disabled}
		class:cursor-not-allowed={disabled}
		class:bg-neutral-100={disabled}
		class:hover:bg-stone-100={!disabled}
		class:px-3={!disabled}
		class:py-1.5={!disabled}
		class:w-8={disabled}
		class:h-8={disabled}
		class:justify-center={disabled}
		aria-label="Select agent"
	>
		{#if disabled}
			<iconify-icon
				icon={currentAgent.icon}
				style="color: {currentAgent.color}"
				width="16"
				in:fade={{ duration: 200 }}
			></iconify-icon>
		{:else}
			<iconify-icon
				icon={currentAgent.icon}
				style="color: {currentAgent.color}"
				width="14"
			></iconify-icon>
			<span class="text-neutral-700">{currentAgent.name}</span>
			<iconify-icon
				icon="ri:arrow-down-s-line"
				class="text-neutral-400 transition-transform duration-200"
				class:rotate-180={open}
				width="16"
			></iconify-icon>
		{/if}
	</button>

	{#if open && !disabled}
		<div
			bind:this={dropdownElement}
			class="absolute z-50 left-0 top-full mt-2 w-72 bg-white border border-neutral-300 shadow-lg rounded-lg overflow-hidden"
			transition:fly={{ y: -10, duration: 200, easing: cubicOut }}
		>
			<div class="max-h-80 overflow-y-auto">
				{#each agents as agent}
					<button
						type="button"
						class="w-full px-4 py-3 text-left transition-colors border-b border-neutral-100 last:border-b-0"
						class:bg-neutral-50={agent.id === value}
						class:hover:bg-neutral-50={agent.id !== value}
						onclick={() => selectAgent(agent.id)}
					>
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
							{#if agent.id === value}
								<iconify-icon
									icon="ri:check-line"
									class="text-blue-600 flex-shrink-0 mt-0.5"
									width="16"
								></iconify-icon>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
