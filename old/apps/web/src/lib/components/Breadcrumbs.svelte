<script lang="ts">
	import { page } from "$app/state";

	interface BreadcrumbItem {
		label: string;
		href: string;
	}

	let breadcrumbs = $derived.by(() => {
		const path = page.url.pathname;
		const segments = path.split("/").filter(Boolean);

		const items: BreadcrumbItem[] = [{ label: "Home", href: "/" }];

		// Build breadcrumb path
		let currentPath = "";
		for (const segment of segments) {
			currentPath += `/${segment}`;
			// Capitalize first letter and replace hyphens with spaces
			const label = segment
				.split("-")
				.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
				.join(" ");
			items.push({ label, href: currentPath });
		}

		return items;
	});
</script>

<nav aria-label="Breadcrumb" class="flex items-center space-x-1 text-sm">
	{#each breadcrumbs as crumb, index}
		{#if index > 0}
			<span class="text-neutral-400 px-2">
				{"/"}
			</span>
		{/if}

		{#if index === breadcrumbs.length - 1}
			<span class="text-neutral-600 font-medium">
				{crumb.label}
			</span>
		{:else}
			<a
				href={crumb.href}
				class="text-neutral-500 hover:text-neutral-700 transition-colors"
			>
				{crumb.label}
			</a>
		{/if}
	{/each}
</nav>
