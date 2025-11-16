<script lang="ts">
	import { fly, fade } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import { models, type ModelOption, DEFAULT_MODEL } from "$lib/config/models";

	interface Props {
		value: ModelOption;
		disabled?: boolean;
	}

	let {
		value = $bindable(),
		disabled = false,
	}: Props = $props();

	// Helper function to get icon for provider
	function getProviderIcon(provider: string): string {
		switch (provider.toLowerCase()) {
			case 'anthropic':
				return 'ri:claude-fill';
			case 'openai':
				return 'simple-icons:openai';
			case 'google':
				return 'simple-icons:google';
			case 'xai':
				return 'ri:twitter-x-fill';
			case 'moonshot ai':
				return 'ri:moon-fill';
			default:
				return 'ri:robot-fill';
		}
	}

	// Helper function to get icon color for provider
	function getProviderIconColor(provider: string): string {
		switch (provider.toLowerCase()) {
			case 'anthropic':
				return 'text-orange-600';
			case 'openai':
				return 'text-green-600';
			case 'google':
				return 'text-blue-600';
			case 'xai':
				return 'text-neutral-900';
			case 'moonshot ai':
				return 'text-purple-600';
			default:
				return 'text-neutral-600';
		}
	}

	let open = $state(false);
	let buttonElement: HTMLButtonElement;
	let dropdownElement: HTMLDivElement;

	function selectModel(model: ModelOption) {
		console.log('[ModelPicker] selectModel called:', {
			oldModel: value.id,
			newModel: model.id,
			timestamp: Date.now()
		});
		value = model;
		console.log('[ModelPicker] value updated to:', value.id);
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
		aria-label="Select model"
	>
		{#if disabled}
			<iconify-icon
				icon={getProviderIcon(value.provider)}
				class="text-neutral-600"
				width="16"
				in:fade={{ duration: 200 }}
			></iconify-icon>
		{:else}
			<span class="text-neutral-700">{value.displayName}</span>
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
			class="absolute z-50 left-0 top-full mt-2 w-64 bg-white border border-neutral-300 shadow-lg rounded-lg overflow-hidden"
			transition:fly={{ y: -10, duration: 200, easing: cubicOut }}
		>
			<div class="max-h-80 overflow-y-auto">
				{#each models as model}
					<button
						type="button"
						class="w-full px-4 py-2.5 text-left transition-colors border-b border-neutral-100 last:border-b-0"
						class:bg-neutral-50={model.id === value.id}
						class:hover:bg-neutral-50={model.id !== value.id}
						onclick={() => selectModel(model)}
					>
						<div class="flex items-center justify-between gap-2">
							<div class="flex items-center gap-2">
								<iconify-icon
									icon={getProviderIcon(model.provider)}
									class={getProviderIconColor(model.provider)}
									width="16"
								></iconify-icon>
								<span class="text-sm text-neutral-900">
									{model.displayName}
									{#if model.id === DEFAULT_MODEL.id}
										<span class="text-xs text-neutral-500 ml-1">(recommended)</span>
									{/if}
								</span>
							</div>
							{#if model.id === value.id}
								<iconify-icon
									icon="ri:check-line"
									class="text-blue-600 flex-shrink-0"
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
