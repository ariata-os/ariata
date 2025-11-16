<script lang="ts">
	import { onMount } from "svelte";

	interface Props {
		tags: string[];
		ontags?: (tags: string[]) => void;
	}

	let { tags = $bindable([]), ontags }: Props = $props();

	let inputValue = $state("");
	let availableTags = $state<string[]>([]);
	let selectedIndex = $state(-1);
	let manuallyHidden = $state(false);

	const API_BASE = "/api";

	// Load available tags from API
	async function loadAvailableTags() {
		try {
			const response = await fetch(`${API_BASE}/axiology/tags`);
			if (response.ok) {
				availableTags = await response.json();
			}
		} catch (error) {
			console.error("Failed to load tags:", error);
		}
	}

	// Filter tags based on input - use $derived to avoid infinite loops
	const filteredTags = $derived(() => {
		if (!inputValue.trim()) return [];
		const input = inputValue.toLowerCase();
		return availableTags
			.filter(
				(tag) =>
					tag.toLowerCase().includes(input) &&
					!tags.includes(tag),
			)
			.slice(0, 10); // Limit to 10 suggestions
	});

	const showDropdown = $derived(
		!manuallyHidden && inputValue.trim().length > 0 && filteredTags.length > 0
	);

	function addTag(tag: string) {
		if (tag.trim() && !tags.includes(tag.trim())) {
			tags = [...tags, tag.trim()];
			if (ontags) ontags(tags);
			inputValue = "";
			selectedIndex = -1;
			manuallyHidden = false;
		}
	}

	function removeTag(index: number) {
		tags = tags.filter((_, i) => i !== index);
		if (ontags) ontags(tags);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") {
			e.preventDefault();
			if (selectedIndex >= 0 && filteredTags[selectedIndex]) {
				addTag(filteredTags[selectedIndex]);
			} else if (inputValue.trim()) {
				addTag(inputValue);
			}
		} else if (e.key === "ArrowDown") {
			e.preventDefault();
			manuallyHidden = false;
			selectedIndex = Math.min(
				selectedIndex + 1,
				filteredTags.length - 1,
			);
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			manuallyHidden = false;
			selectedIndex = Math.max(selectedIndex - 1, -1);
		} else if (e.key === "Escape") {
			manuallyHidden = true;
			selectedIndex = -1;
		} else if (e.key === "Backspace" && !inputValue && tags.length > 0) {
			removeTag(tags.length - 1);
		} else {
			// Reset manuallyHidden when user types
			manuallyHidden = false;
		}
	}

	function selectTag(tag: string) {
		addTag(tag);
	}

	onMount(() => {
		loadAvailableTags();
	});
</script>

<div class="relative">
	<div
		class="w-full px-3 py-2 border border-neutral-300 rounded-md min-h-[40px] flex flex-wrap gap-2 items-center"
	>
		{#each tags as tag, i}
			<span
				class="inline-flex items-center gap-1 px-2 py-1 text-xs bg-neutral-100 text-neutral-700 rounded"
			>
				{tag}
				<button
					type="button"
					onclick={() => removeTag(i)}
					class="hover:text-neutral-900"
					aria-label="Remove tag"
				>
					×
				</button>
			</span>
		{/each}
		<input
			type="text"
			bind:value={inputValue}
			onkeydown={handleKeydown}
			placeholder={tags.length === 0 ? "Type to add tags..." : ""}
			class="flex-1 min-w-[120px] outline-none bg-transparent"
		/>
	</div>

	{#if showDropdown && filteredTags.length > 0}
		<div
			class="absolute z-10 w-full mt-1 bg-white border border-neutral-300 rounded-md shadow-lg max-h-48 overflow-y-auto"
		>
			{#each filteredTags as tag, i}
				<button
					type="button"
					onclick={() => selectTag(tag)}
					class="w-full text-left px-3 py-2 text-sm hover:bg-neutral-100 {selectedIndex ===
					i
						? 'bg-neutral-100'
						: ''}"
				>
					{tag}
				</button>
			{/each}
		</div>
	{/if}
</div>
