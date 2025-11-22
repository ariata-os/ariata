/**
 * Theme management utilities for Ariata
 *
 * Provides functions to get, set, and toggle themes across the application.
 * Themes are stored in localStorage and applied via data-theme attribute on <html>.
 */

export type Theme = 'light' | 'dark' | 'warm' | 'high-contrast';

const THEME_STORAGE_KEY = 'ariata-theme';
const DEFAULT_THEME: Theme = 'light';

/**
 * Get the current theme from localStorage or system preference
 */
export function getTheme(): Theme {
	if (typeof window === 'undefined') {
		return DEFAULT_THEME;
	}

	// Check localStorage first
	const stored = localStorage.getItem(THEME_STORAGE_KEY) as Theme | null;
	if (stored && isValidTheme(stored)) {
		return stored;
	}

	// Fall back to system preference for dark mode
	if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
		return 'dark';
	}

	return DEFAULT_THEME;
}

/**
 * Set the theme and persist to localStorage
 */
export function setTheme(theme: Theme): void {
	if (typeof window === 'undefined') {
		return;
	}

	// Validate theme
	if (!isValidTheme(theme)) {
		console.warn(`Invalid theme: ${theme}. Using default.`);
		theme = DEFAULT_THEME;
	}

	// Apply to document
	document.documentElement.setAttribute('data-theme', theme);

	// Persist to localStorage
	localStorage.setItem(THEME_STORAGE_KEY, theme);

	// Dispatch custom event for listeners
	window.dispatchEvent(new CustomEvent('themechange', { detail: { theme } }));
}

/**
 * Toggle between light and dark themes
 */
export function toggleTheme(): void {
	const current = getTheme();
	const next = current === 'dark' ? 'light' : 'dark';
	setTheme(next);
}

/**
 * Initialize theme on page load
 * Call this in your root layout or app component
 */
export function initTheme(): void {
	if (typeof window === 'undefined') {
		return;
	}

	const theme = getTheme();
	document.documentElement.setAttribute('data-theme', theme);
}

/**
 * Listen to system theme preference changes
 */
export function watchSystemTheme(callback: (isDark: boolean) => void): () => void {
	if (typeof window === 'undefined') {
		return () => {};
	}

	const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

	const handler = (e: MediaQueryListEvent) => {
		callback(e.matches);
	};

	// Modern browsers
	if (mediaQuery.addEventListener) {
		mediaQuery.addEventListener('change', handler);
		return () => mediaQuery.removeEventListener('change', handler);
	}

	// Older browsers
	mediaQuery.addListener(handler);
	return () => mediaQuery.removeListener(handler);
}

/**
 * Type guard to check if a string is a valid theme
 */
function isValidTheme(theme: string): theme is Theme {
	return ['light', 'dark', 'warm', 'high-contrast'].includes(theme);
}

/**
 * Get all available themes
 */
export function getAvailableThemes(): Theme[] {
	return ['light', 'dark', 'warm', 'high-contrast'];
}

/**
 * Get theme display name
 */
export function getThemeDisplayName(theme: Theme): string {
	const names: Record<Theme, string> = {
		light: 'Light',
		dark: 'Dark',
		warm: 'Warm',
		'high-contrast': 'High Contrast'
	};
	return names[theme];
}
