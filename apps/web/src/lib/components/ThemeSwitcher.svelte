<script lang="ts">
	import { onMount } from 'svelte';
	import { getTheme, setTheme, getAvailableThemes, getThemeDisplayName, type Theme } from '$lib/utils/theme';

	let currentTheme = $state<Theme>('light');
	let isOpen = $state(false);

	const themes = getAvailableThemes();

	onMount(() => {
		// Initialize current theme
		currentTheme = getTheme();

		// Listen for theme changes from other sources
		const handleThemeChange = (e: Event) => {
			const customEvent = e as CustomEvent<{ theme: Theme }>;
			currentTheme = customEvent.detail.theme;
		};

		window.addEventListener('themechange', handleThemeChange);

		return () => {
			window.removeEventListener('themechange', handleThemeChange);
		};
	});

	function handleThemeSelect(theme: Theme) {
		setTheme(theme);
		currentTheme = theme;
		isOpen = false;
	}

	function toggleDropdown() {
		isOpen = !isOpen;
	}

	// Close dropdown when clicking outside
	function handleClickOutside(e: MouseEvent) {
		if (isOpen && !(e.target as HTMLElement).closest('.theme-switcher')) {
			isOpen = false;
		}
	}
</script>

<svelte:window onclick={handleClickOutside} />

<div class="theme-switcher relative">
	<button
		onclick={toggleDropdown}
		class="px-3 py-2 rounded-lg bg-surface hover:bg-surface-elevated border border-border transition-colors duration-150 flex items-center gap-2 text-sm"
		aria-label="Select theme"
		aria-expanded={isOpen}
	>
		<span class="text-foreground-muted">Theme:</span>
		<span class="text-foreground font-medium">{getThemeDisplayName(currentTheme)}</span>
		<svg
			class="w-4 h-4 text-foreground-subtle transition-transform duration-150"
			class:rotate-180={isOpen}
			fill="none"
			stroke="currentColor"
			viewBox="0 0 24 24"
		>
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>

	{#if isOpen}
		<div
			class="absolute right-0 mt-2 w-48 bg-surface-overlay border border-border rounded-lg shadow-lg overflow-hidden z-50"
		>
			{#each themes as theme}
				<button
					onclick={() => handleThemeSelect(theme)}
					class="w-full px-4 py-2.5 text-left text-sm hover:bg-surface-elevated transition-colors duration-150 flex items-center justify-between"
					class:bg-surface-elevated={theme === currentTheme}
				>
					<span class="text-foreground">{getThemeDisplayName(theme)}</span>
					{#if theme === currentTheme}
						<svg class="w-4 h-4 text-primary" fill="currentColor" viewBox="0 0 20 20">
							<path
								fill-rule="evenodd"
								d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
								clip-rule="evenodd"
							/>
						</svg>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.rotate-180 {
		transform: rotate(180deg);
	}
</style>
