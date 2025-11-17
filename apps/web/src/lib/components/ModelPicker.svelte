<script lang="ts">
	import { fade } from "svelte/transition";
	import type { ModelOption } from "$lib/config/models";
	import { getModels, getDefaultModel, isLoading, getError } from "$lib/stores/models.svelte";
	import UniversalSelect from "./UniversalSelect.svelte";

	interface Props {
		value?: ModelOption;
		disabled?: boolean;
		onSelect?: (model: ModelOption) => void;
	}

	let {
		value,
		disabled = false,
		onSelect,
	}: Props = $props();

	// Get models from store (reactive)
	const models = $derived(getModels());
	const loading = $derived(isLoading());
	const error = $derived(getError());
	const DEFAULT_MODEL = $derived(getDefaultModel());

	// Local state for UniversalSelect binding
	let localValue = $state<ModelOption | undefined>(value || DEFAULT_MODEL);

	// Sync local value with prop value
	$effect(() => {
		if (value && value !== localValue) {
			localValue = value;
		}
	});

	// Notify parent when local value changes
	$effect(() => {
		if (localValue && localValue !== value && onSelect) {
			onSelect(localValue);
		}
	});

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

	function getModelKey(model: ModelOption) {
		return model.id;
	}
</script>

{#if loading}
	<!-- Loading state -->
	<div class="flex items-center gap-2 px-3 py-1.5 text-neutral-500">
		<iconify-icon icon="ri:loader-4-line" class="animate-spin" width="16"></iconify-icon>
		<span class="text-sm">Loading models...</span>
	</div>
{:else if error}
	<!-- Error state -->
	<div class="flex items-center gap-2 px-3 py-1.5 text-red-600">
		<iconify-icon icon="ri:error-warning-line" width="16"></iconify-icon>
		<span class="text-sm">Error loading models</span>
	</div>
{:else if localValue}
	<!-- Model picker when loaded -->
	<UniversalSelect
		bind:value={localValue}
		items={models}
		disabled={disabled || loading}
		width="w-64"
		getKey={getModelKey}
	>
		{#snippet trigger(model: ModelOption, isDisabled: boolean, open: boolean)}
			<div
				class="flex items-center gap-2"
				class:px-3={!isDisabled}
				class:py-1.5={!isDisabled}
				class:w-8={isDisabled}
				class:h-8={isDisabled}
				class:justify-center={isDisabled}
				aria-label="Select model"
			>
				{#if isDisabled}
					<iconify-icon
						icon={getProviderIcon(model.provider)}
						class="text-neutral-600"
						width="16"
						in:fade={{ duration: 200 }}
					></iconify-icon>
				{:else}
					<span class="text-neutral-700">{model.displayName}</span>
					<iconify-icon
						icon="ri:arrow-down-s-line"
						class="text-neutral-400 transition-transform duration-200"
						class:rotate-180={open}
						width="16"
					></iconify-icon>
				{/if}
			</div>
		{/snippet}

	{#snippet item(model: ModelOption, isSelected: boolean)}
		<div class="px-4 py-2.5">
			<div class="flex items-center justify-between gap-2">
				<div class="flex items-center gap-2">
					<iconify-icon
						icon={getProviderIcon(model.provider)}
						class={getProviderIconColor(model.provider)}
						width="16"
					></iconify-icon>
					<span class="text-sm text-neutral-900">
						{model.displayName}
						{#if DEFAULT_MODEL && model.id === DEFAULT_MODEL.id}
							<span class="text-xs text-neutral-500 ml-1">(recommended)</span>
						{/if}
					</span>
				</div>
				{#if isSelected}
					<iconify-icon
						icon="ri:check-line"
						class="text-blue-600 flex-shrink-0"
						width="16"
					></iconify-icon>
				{/if}
			</div>
		</div>
	{/snippet}
	</UniversalSelect>
{/if}
