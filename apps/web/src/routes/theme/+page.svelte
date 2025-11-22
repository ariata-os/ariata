<script lang="ts">
	import ThemeSwitcher from "$lib/components/ThemeSwitcher.svelte";
</script>

<div class="container mx-auto p-6 max-w-6xl">
	<!-- Header -->
	<div class="mb-8">
		<div class="flex items-center justify-between mb-4">
			<div>
				<h1 class="text-3xl font-serif text-foreground mb-2">Theme System</h1>
				<p class="text-foreground-subtle">
					Ariata's semantic color system with multiple theme variants
				</p>
			</div>
			<ThemeSwitcher />
		</div>
	</div>

	<!-- Theme Overview -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Available Themes</h2>
		<div class="grid grid-cols-2 gap-4">
			<div class="p-4 bg-background rounded border border-border-subtle">
				<h3 class="font-medium text-foreground mb-1">Light (Default)</h3>
				<p class="text-sm text-foreground-subtle">Paper background with navy accents</p>
			</div>
			<div class="p-4 bg-background rounded border border-border-subtle">
				<h3 class="font-medium text-foreground mb-1">Dark</h3>
				<p class="text-sm text-foreground-subtle">Navy-based dark theme for reduced eye strain</p>
			</div>
			<div class="p-4 bg-background rounded border border-border-subtle">
				<h3 class="font-medium text-foreground mb-1">Warm</h3>
				<p class="text-sm text-foreground-subtle">Sepia-toned with warm browns and amber</p>
			</div>
			<div class="p-4 bg-background rounded border border-border-subtle">
				<h3 class="font-medium text-foreground mb-1">High Contrast</h3>
				<p class="text-sm text-foreground-subtle">Maximum contrast (WCAG AAA compliant)</p>
			</div>
		</div>
	</div>

	<!-- Color Palette Preview -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Semantic Colors</h2>
		<div class="grid grid-cols-3 gap-6">
			<!-- Surface Colors -->
			<div>
				<h3 class="text-sm font-medium text-foreground mb-3">Surface Colors</h3>
				<div class="space-y-2">
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-background border border-border"></div>
						<div>
							<div class="text-sm font-mono text-foreground">background</div>
							<div class="text-xs text-foreground-subtle">Main background</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface border border-border"></div>
						<div>
							<div class="text-sm font-mono text-foreground">surface</div>
							<div class="text-xs text-foreground-subtle">Cards & panels</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface-elevated border border-border"></div>
						<div>
							<div class="text-sm font-mono text-foreground">surface-elevated</div>
							<div class="text-xs text-foreground-subtle">Hover states</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface-overlay border border-border"></div>
						<div>
							<div class="text-sm font-mono text-foreground">surface-overlay</div>
							<div class="text-xs text-foreground-subtle">Modals & overlays</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Content Colors -->
			<div>
				<h3 class="text-sm font-medium text-foreground mb-3">Content Colors</h3>
				<div class="space-y-2">
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface border border-border flex items-center justify-center">
							<span class="text-foreground font-bold">Aa</span>
						</div>
						<div>
							<div class="text-sm font-mono text-foreground">foreground</div>
							<div class="text-xs text-foreground-subtle">Primary text</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface border border-border flex items-center justify-center">
							<span class="text-foreground-muted font-bold">Aa</span>
						</div>
						<div>
							<div class="text-sm font-mono text-foreground">foreground-muted</div>
							<div class="text-xs text-foreground-subtle">Secondary text</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface border border-border flex items-center justify-center">
							<span class="text-foreground-subtle font-bold">Aa</span>
						</div>
						<div>
							<div class="text-sm font-mono text-foreground">foreground-subtle</div>
							<div class="text-xs text-foreground-subtle">Tertiary text</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-surface border border-border flex items-center justify-center">
							<span class="text-foreground-disabled font-bold">Aa</span>
						</div>
						<div>
							<div class="text-sm font-mono text-foreground">foreground-disabled</div>
							<div class="text-xs text-foreground-subtle">Disabled text</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Interactive Colors -->
			<div>
				<h3 class="text-sm font-medium text-foreground mb-3">Interactive Colors</h3>
				<div class="space-y-2">
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-primary"></div>
						<div>
							<div class="text-sm font-mono text-foreground">primary</div>
							<div class="text-xs text-foreground-subtle">Actions & links</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-primary-hover"></div>
						<div>
							<div class="text-sm font-mono text-foreground">primary-hover</div>
							<div class="text-xs text-foreground-subtle">Hover state</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-secondary"></div>
						<div>
							<div class="text-sm font-mono text-foreground">secondary</div>
							<div class="text-xs text-foreground-subtle">Secondary actions</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-16 h-16 rounded-lg bg-accent"></div>
						<div>
							<div class="text-sm font-mono text-foreground">accent</div>
							<div class="text-xs text-foreground-subtle">Highlights</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>

	<!-- Status Colors -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Status Colors</h2>
		<div class="grid grid-cols-4 gap-4">
			<div class="p-4 bg-success-subtle border border-success rounded-lg">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-4 h-4 rounded-full bg-success"></div>
					<span class="text-sm font-medium text-success">Success</span>
				</div>
				<p class="text-xs text-foreground-subtle">Positive actions and confirmations</p>
			</div>
			<div class="p-4 bg-warning-subtle border border-warning rounded-lg">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-4 h-4 rounded-full bg-warning"></div>
					<span class="text-sm font-medium text-warning">Warning</span>
				</div>
				<p class="text-xs text-foreground-subtle">Caution and important notices</p>
			</div>
			<div class="p-4 bg-error-subtle border border-error rounded-lg">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-4 h-4 rounded-full bg-error"></div>
					<span class="text-sm font-medium text-error">Error</span>
				</div>
				<p class="text-xs text-foreground-subtle">Errors and destructive actions</p>
			</div>
			<div class="p-4 bg-info-subtle border border-info rounded-lg">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-4 h-4 rounded-full bg-info"></div>
					<span class="text-sm font-medium text-info">Info</span>
				</div>
				<p class="text-xs text-foreground-subtle">Informational messages</p>
			</div>
		</div>
	</div>

	<!-- Typography Preview -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Typography</h2>
		<div class="space-y-4">
			<div>
				<h1>Heading 1 - The quick brown fox</h1>
				<h2>Heading 2 - The quick brown fox</h2>
				<h3>Heading 3 - The quick brown fox</h3>
				<h4>Heading 4 - The quick brown fox</h4>
				<h5>Heading 5 - The quick brown fox</h5>
				<h6>Heading 6 - The quick brown fox</h6>
			</div>
			<div class="pt-4 border-t border-border-subtle">
				<p class="text-foreground mb-2">
					This is body text using the <code>foreground</code> color. It's designed for optimal readability across all themes.
				</p>
				<p class="text-foreground-muted mb-2">
					This is muted text using <code>foreground-muted</code> for secondary information.
				</p>
				<p class="text-foreground-subtle mb-2">
					This is subtle text using <code>foreground-subtle</code> for tertiary information.
				</p>
				<p>
					<a href="#" class="text-primary hover:text-primary-hover">This is a link</a> with automatic hover states.
				</p>
			</div>
		</div>
	</div>

	<!-- Button Examples -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Button Variants</h2>
		<div class="space-y-4">
			<div>
				<h3 class="text-sm font-medium text-foreground mb-2">Default State</h3>
				<div class="flex gap-3">
					<button class="btn-primary px-4 py-2 rounded-lg">Primary Button</button>
					<button class="btn-secondary px-4 py-2 rounded-lg">Secondary Button</button>
					<button class="btn-ghost px-4 py-2 rounded-lg">Ghost Button</button>
				</div>
			</div>
			<div>
				<h3 class="text-sm font-medium text-foreground mb-2">Disabled State</h3>
				<div class="flex gap-3">
					<button class="btn-primary px-4 py-2 rounded-lg" disabled>Primary Button</button>
					<button class="btn-secondary px-4 py-2 rounded-lg" disabled>Secondary Button</button>
					<button class="btn-ghost px-4 py-2 rounded-lg" disabled>Ghost Button</button>
				</div>
			</div>
		</div>
	</div>

	<!-- Form Elements -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Form Elements</h2>
		<div class="space-y-4 max-w-md">
			<div>
				<label class="block text-sm font-medium text-foreground mb-1">Text Input</label>
				<input
					type="text"
					placeholder="Enter some text..."
					class="w-full px-3 py-2 rounded-lg"
				/>
			</div>
			<div>
				<label class="block text-sm font-medium text-foreground mb-1">Textarea</label>
				<textarea
					placeholder="Enter a longer message..."
					rows="3"
					class="w-full px-3 py-2 rounded-lg"
				></textarea>
			</div>
			<div>
				<label class="block text-sm font-medium text-foreground mb-1">Select</label>
				<select class="w-full px-3 py-2 rounded-lg">
					<option>Option 1</option>
					<option>Option 2</option>
					<option>Option 3</option>
				</select>
			</div>
			<div>
				<label class="block text-sm font-medium text-foreground mb-1">Disabled Input</label>
				<input
					type="text"
					value="This input is disabled"
					disabled
					class="w-full px-3 py-2 rounded-lg"
				/>
			</div>
		</div>
	</div>

	<!-- Border Examples -->
	<div class="mb-8 p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Border Styles</h2>
		<div class="grid grid-cols-3 gap-4">
			<div class="p-4 border border-border rounded-lg">
				<div class="text-sm font-mono text-foreground mb-1">border</div>
				<div class="text-xs text-foreground-subtle">Default borders</div>
			</div>
			<div class="p-4 border border-border-subtle rounded-lg">
				<div class="text-sm font-mono text-foreground mb-1">border-subtle</div>
				<div class="text-xs text-foreground-subtle">Subtle dividers</div>
			</div>
			<div class="p-4 border-2 border-border-strong rounded-lg">
				<div class="text-sm font-mono text-foreground mb-1">border-strong</div>
				<div class="text-xs text-foreground-subtle">Emphasized borders</div>
			</div>
		</div>
	</div>

	<!-- Code Examples -->
	<div class="p-6 bg-surface rounded-lg border border-border">
		<h2 class="text-xl font-serif text-foreground mb-4">Code Styling</h2>
		<div class="space-y-4">
			<div>
				<h3 class="text-sm font-medium text-foreground mb-2">Inline Code</h3>
				<p class="text-foreground">
					Use <code>semantic colors</code> like <code>bg-surface</code> and <code>text-foreground</code> for theme-aware styling.
				</p>
			</div>
			<div>
				<h3 class="text-sm font-medium text-foreground mb-2">Code Block</h3>
				<pre class="p-4 overflow-x-auto"><code>&lt;div class="bg-surface text-foreground"&gt;
  &lt;h1&gt;Theme-aware component&lt;/h1&gt;
  &lt;p class="text-foreground-muted"&gt;Automatically adapts to theme&lt;/p&gt;
&lt;/div&gt;</code></pre>
			</div>
		</div>
	</div>
</div>
