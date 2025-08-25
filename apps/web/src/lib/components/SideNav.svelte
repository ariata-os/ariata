<script lang="ts">
	import { page } from "$app/state";
	import "iconify-icon";

	interface NavItem {
		href?: string;
		icon?: string;
		text: string;
		pagespace?: string;
		type?: "item" | "title";
	}

	let {
		items,
		moduleTitle,
		class: className = "",
	} = $props<{
		items: NavItem[];
		moduleTitle: string;
		class?: string;
	}>();

	function isActive(href?: string, pagespace?: string): boolean {
		if (!href) return false;

		if (page.url.pathname === href) {
			return true;
		}

		if (pagespace === "") {
			return page.url.pathname === "/";
		}

		if (pagespace) {
			return page.url.pathname.startsWith(`/${pagespace}`);
		}

		return false;
	}
</script>

<div class="flex w-56 flex-col border-neutral-200 bg-neutral-100 {className}">
	<!-- Module Header -->
	<div class=" p-4 h-16 flex items-center">
		<h2
			class="font-mono text-base font-medium whitespace-nowrap tracking-wider text-neutral-900"
		>
			{moduleTitle}
		</h2>
	</div>

	<!-- Navigation Items -->
	<nav class="flex-1">
		{#each items as item}
			{#if item.type === "title"}
				<div class="mx-3 mt-4 mb-2 pl-0">
					<div
						class="text-xs font-medium text-neutral-900 font-mono uppercase tracking-wider"
					>
						{item.text}
					</div>
				</div>
			{:else}
				<a
					href={item.href}
					class="mx-3 rounded-lg mb-1 flex h-9 items-center px-3 text-sm transition-colors hover:bg-neutral-200"
					class:active={isActive(item.href, item.pagespace)}
				>
					<div class="flex w-full items-center">
						<div
							class="flex-shrink-0 flex items-center justify-between"
						>
							<iconify-icon
								icon={item.icon}
								class="text-base text-neutral-600 font-medium"
							></iconify-icon>
						</div>
						<div class="ml-3 whitespace-nowrap text-neutral-600">
							{item.text}
						</div>
					</div>
				</a>
			{/if}
		{/each}
	</nav>
</div>

<style>
	@reference "../../app.css";

	.active {
		@apply bg-neutral-300/50 text-neutral-900 font-medium;
	}

	.active div {
		@apply text-neutral-900;
	}
</style>
