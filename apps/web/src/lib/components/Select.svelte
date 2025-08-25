<script lang="ts">
    import { onMount } from "svelte";
    import "iconify-icon";

    interface Option {
        value: string;
        label: string;
        description?: string;
        icon?: string;
        disabled?: boolean;
    }

    interface Props {
        options: Option[];
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        class?: string;
        error?: boolean;
        searchable?: boolean;
        clearable?: boolean;
        onchange?: (value: string) => void;
    }

    let {
        options = [],
        value = $bindable(),
        placeholder = "Select an option...",
        disabled = false,
        class: className = "",
        error = false,
        searchable = false,
        clearable = false,
        onchange,
    }: Props = $props();

    let dropdownOpen = $state(false);
    let dropdownRef = $state<HTMLElement>();
    let searchTerm = $state("");
    let selectedOption = $derived(options.find((opt) => opt.value === value));

    // Filter options based on search term
    let filteredOptions = $derived(
        searchable && searchTerm
            ? options.filter(
                  (opt) =>
                      opt.label
                          .toLowerCase()
                          .includes(searchTerm.toLowerCase()) ||
                      opt.description
                          ?.toLowerCase()
                          .includes(searchTerm.toLowerCase()),
              )
            : options,
    );

    function toggleDropdown() {
        if (disabled) return;
        dropdownOpen = !dropdownOpen;
        if (dropdownOpen && searchable) {
            searchTerm = "";
        }
    }

    function selectOption(option: Option) {
        if (option.disabled) return;
        value = option.value;
        dropdownOpen = false;
        searchTerm = "";
        onchange?.(option.value);
    }

    function clearSelection() {
        value = "";
        dropdownOpen = false;
        searchTerm = "";
        onchange?.("");
    }

    function handleClickOutside(event: MouseEvent) {
        if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
            dropdownOpen = false;
            searchTerm = "";
        }
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            dropdownOpen = false;
            searchTerm = "";
        }
    }

    onMount(() => {
        document.addEventListener("click", handleClickOutside);
        document.addEventListener("keydown", handleKeydown);

        return () => {
            document.removeEventListener("click", handleClickOutside);
            document.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<div class="relative {className}" bind:this={dropdownRef}>
    <button
        type="button"
        onclick={toggleDropdown}
        {disabled}
        class="w-full px-3 py-1 text-left border-2 bg-white rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 transition-all cursor-pointer flex items-center justify-between {error
            ? 'border-red-500 focus:border-red-500'
            : disabled
              ? 'border-neutral-200 bg-neutral-50 cursor-not-allowed'
              : 'border-neutral-200 hover:border-neutral-300 focus:border-blue-500'}"
    >
        <div class="flex items-center gap-3 flex-1 min-w-0">
            {#if selectedOption}
                {#if selectedOption.icon}
                    <iconify-icon
                        icon={selectedOption.icon}
                        width="20"
                        height="20"
                        class="text-neutral-700 flex-shrink-0"
                    ></iconify-icon>
                {/if}
                <div class="min-w-0 flex-1">
                    <span class="text-neutral-900 truncate block"
                        >{selectedOption.label}</span
                    >
                    {#if selectedOption.description}
                        <span class="text-neutral-500 text-sm truncate block"
                            >{selectedOption.description}</span
                        >
                    {/if}
                </div>
            {:else}
                <span class="text-neutral-500 truncate">{placeholder}</span>
            {/if}
        </div>

        <div class="flex items-center gap-2 flex-shrink-0">
            {#if clearable && selectedOption}
                <!-- svelte-ignore a11y_consider_explicit_label -->
                <div
                    role="button"
                    tabindex="0"
                    onclick={(e) => {
                        e.stopPropagation();
                        clearSelection();
                    }}
                    onkeydown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            e.stopPropagation();
                            clearSelection();
                        }
                    }}
                    class="p-1 hover:bg-neutral-100 rounded text-neutral-400 hover:text-neutral-600 transition-colors cursor-pointer"
                >
                    <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M6 18L18 6M6 6l12 12"
                        ></path>
                    </svg>
                </div>
            {/if}
            <svg
                class="w-5 h-5 text-neutral-400 transition-transform {dropdownOpen
                    ? 'rotate-180'
                    : ''}"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                ></path>
            </svg>
        </div>
    </button>

    {#if dropdownOpen}
        <div
            class="absolute z-10 w-full mt-1 bg-white border border-neutral-200 rounded-lg shadow-lg max-h-60 overflow-hidden"
        >
            {#if searchable}
                <div class="p-2 border-b border-neutral-200">
                    <input
                        type="text"
                        bind:value={searchTerm}
                        placeholder="Search options..."
                        class="w-full px-3 py-1.5 border border-neutral-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    />
                </div>
            {/if}

            <div class="max-h-48 overflow-y-auto">
                {#each filteredOptions as option}
                    <button
                        type="button"
                        onclick={() => selectOption(option)}
                        disabled={option.disabled}
                        class="w-full px-3 py-2 text-left flex items-center gap-3 transition-colors {option.disabled
                            ? 'opacity-50 cursor-not-allowed'
                            : option.value === value
                              ? 'bg-blue-50 text-blue-900'
                              : 'hover:bg-neutral-50'}"
                    >
                        {#if option.icon}
                            <iconify-icon
                                icon={option.icon}
                                width="20"
                                height="20"
                                class="text-neutral-700 flex-shrink-0"
                            ></iconify-icon>
                        {/if}
                        <div class="min-w-0 flex-1">
                            <div class="font-medium text-neutral-900 truncate">
                                {option.label}
                            </div>
                            {#if option.description}
                                <div class="text-sm text-neutral-500 truncate">
                                    {option.description}
                                </div>
                            {/if}
                        </div>
                        {#if option.value === value}
                            <svg
                                class="w-5 h-5 text-blue-600 flex-shrink-0"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M5 13l4 4L19 7"
                                ></path>
                            </svg>
                        {/if}
                    </button>
                {/each}

                {#if filteredOptions.length === 0}
                    <div class="px-3 py-2 text-neutral-500 text-center">
                        {searchable && searchTerm
                            ? "No matching options"
                            : "No options available"}
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>
