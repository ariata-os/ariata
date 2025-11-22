# Ariata Theme System

A comprehensive, semantic color system with multiple theme variants built on Tailwind CSS v4.

## Overview

The theme system provides:
- **4 theme variants**: Light (default), Dark, Warm, High Contrast
- **Semantic color naming**: Purpose-based colors that adapt to themes
- **Global HTML defaults**: Automatic styling for all HTML elements
- **Easy theme switching**: JavaScript utilities and Svelte component

## Available Themes

### Light (Default)
The standard Ariata theme with paper background and navy accents.

### Dark
Navy-based dark theme with light text for reduced eye strain.

### Warm
Sepia-toned theme with warm browns and amber accents for a cozy reading experience.

### High Contrast
Maximum contrast theme for accessibility (WCAG AAA compliant).

## Semantic Color System

### Surface Colors
Used for backgrounds, cards, and containers:
- `background` - Main app background
- `surface` - Cards, panels, elevated surfaces
- `surface-elevated` - Hover states, raised elements
- `surface-overlay` - Modals, dropdowns, tooltips

### Content Colors
Used for text and foreground elements:
- `foreground` - Primary text color
- `foreground-muted` - Secondary text, less emphasis
- `foreground-subtle` - Tertiary text, placeholders
- `foreground-disabled` - Disabled text

### Interactive Colors
Used for buttons, links, and interactive elements:
- `primary` - Primary actions, links, focus states
- `primary-hover` - Primary hover state
- `primary-active` - Primary pressed/active state
- `primary-subtle` - Primary backgrounds, badges

- `secondary` - Secondary actions
- `secondary-hover` - Secondary hover state
- `secondary-active` - Secondary pressed/active state

- `accent` - Accents, highlights
- `accent-hover` - Accent hover state

### Status Colors
Used for alerts, notifications, and status indicators:
- `success` / `success-subtle` - Success states, positive actions
- `warning` / `warning-subtle` - Warning states, caution
- `error` / `error-subtle` - Error states, destructive actions
- `info` / `info-subtle` - Informational states

### Border Colors
- `border` - Default borders
- `border-subtle` - Subtle borders, dividers
- `border-strong` - Emphasized borders
- `border-focus` - Focus rings

## Usage

### In Tailwind Classes

Use semantic color names with standard Tailwind utilities:

```svelte
<div class="bg-surface text-foreground border border-border">
  <h1 class="text-foreground">Title</h1>
  <p class="text-foreground-muted">Description</p>
  <button class="bg-primary hover:bg-primary-hover text-white">
    Click me
  </button>
</div>
```

### In Custom CSS

Access colors as CSS variables:

```css
.custom-component {
  background-color: var(--color-surface);
  color: var(--color-foreground);
  border: 1px solid var(--color-border);
}

.custom-component:hover {
  background-color: var(--color-surface-elevated);
}
```

### Global HTML Elements

All HTML elements have default semantic styling:

```html
<!-- No classes needed - automatically themed! -->
<h1>This heading uses foreground color</h1>
<p>This paragraph uses foreground color</p>
<a href="#">This link uses primary color with hover</a>
<input type="text" placeholder="Form inputs are themed too">
```

## Theme Switching

### Using the ThemeSwitcher Component

The easiest way to add theme switching:

```svelte
<script>
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
</script>

<ThemeSwitcher />
```

### Programmatic Theme Control

```typescript
import { setTheme, getTheme, toggleTheme } from '$lib/utils/theme';

// Set a specific theme
setTheme('dark');

// Get current theme
const current = getTheme(); // 'light' | 'dark' | 'warm' | 'high-contrast'

// Toggle between light and dark
toggleTheme();
```

### Initialize Theme

The theme is automatically initialized in the root layout, but you can call it manually:

```typescript
import { initTheme } from '$lib/utils/theme';

// Call this once on app startup
initTheme();
```

## Button Variants

Three pre-styled button variants are available:

```svelte
<button class="btn-primary px-4 py-2 rounded-lg">Primary</button>
<button class="btn-secondary px-4 py-2 rounded-lg">Secondary</button>
<button class="btn-ghost px-4 py-2 rounded-lg">Ghost</button>
```

## Migration Guide

### From Brand Colors to Semantic Colors

Old brand colors are still available for backward compatibility but will be removed later:

| Old Color | New Semantic Color | Usage |
|-----------|-------------------|--------|
| `navy` | `secondary` or `foreground` | Depends on context |
| `paper` | `background` | Main background |
| `paper-dark` | `surface-elevated` | Hover states |
| `white` | `surface` | Cards, panels |
| `blue` | `primary` | Links, actions |
| `blue-light` | `primary-hover` | Hover states |

### Updating Components

Replace hardcoded colors with semantic equivalents:

```diff
- <div class="bg-navy text-white">
+ <div class="bg-secondary text-surface">

- <div class="bg-paper text-navy">
+ <div class="bg-background text-foreground">

- <a class="text-blue hover:text-navy">
+ <a class="text-primary hover:text-primary-hover">
```

## Testing Themes

Visit `/theme` to see a comprehensive theme showcase with:
- Color palette preview
- Typography examples
- Button variants
- Status colors
- Live theme switching

## Best Practices

1. **Always use semantic colors** - Don't use brand colors directly
2. **Test all themes** - Make sure your components work in all 4 themes
3. **Use global defaults when possible** - Let HTML elements style themselves
4. **Respect the hierarchy**:
   - `foreground` for primary content
   - `foreground-muted` for secondary content
   - `foreground-subtle` for tertiary content
5. **Use appropriate status colors** - Don't use `error` for non-error states

## Accessibility

- All themes meet WCAG AA contrast standards
- High Contrast theme meets WCAG AAA standards
- Focus states are clearly visible in all themes
- Color is never the only means of conveying information

## File Structure

```
apps/web/src/
├── app.css                          # Main styles & component classes
├── themes.css                       # Theme definitions (4 variants)
├── lib/
│   ├── utils/theme.ts              # Theme utilities
│   └── components/ThemeSwitcher.svelte  # Theme switcher component
└── routes/
    ├── +layout.svelte              # Theme initialization
    └── theme/+page.svelte          # Theme showcase
```

## Architecture Notes

- Themes use Tailwind v4's `@theme` directive with CSS variables
- Theme switching uses `data-theme` attribute on `<html>` element
- Theme preference is persisted to localStorage
- System preference detection with `prefers-color-scheme`
- All themes defined in single CSS file for maintainability
