<script lang="ts">
    import { page } from "$app/state";
    import "iconify-icon";

    interface NavItem {
        href?: string;
        icon?: string;
        text: string;
        pagespace?: string;
        type?: "item" | "title" | "action";
        onclick?: () => void;
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

        // For chat routes with conversationId query param
        if (pagespace) {
            const currentConversationId =
                page.url.searchParams.get("conversationId");
            if (currentConversationId === pagespace) {
                return true;
            }
        }

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

<div class="flex w-56 flex-col border-stone-200 bg-paper-dark {className}">
    <!-- Module Header -->
    <div class="bg-paper-dark p-4 h-16 flex items-center">
        <h2
            class="font-serif text-base font-medium whitespace-nowrap tracking-wider text-navy"
        >
            <!-- {moduleTitle} -->
        </h2>
    </div>

    <!-- Navigation Items -->
    <nav class="flex-1 px-3">
        {#each items as item}
            {#if item.type === "title"}
                <div class="mx-3 mt-4 mb-2">
                    <div
                        class="text-sm font-medium text-navy font-serif uppercase tracking-wider"
                    >
                        {item.text}
                    </div>
                </div>
            {:else if item.onclick || item.type === "action"}
                <button
                    onclick={item.onclick}
                    class="cursor-pointer w-full rounded-lg mb-1 flex h-9 items-center px-3 text-sm transition-colors hover:bg-paper-dark text-left"
                >
                    <div class="flex items-center min-w-0 flex-1">
                        <iconify-icon
                            icon={item.icon}
                            class="text-base text-stone-600 font-medium shrink-0"
                        ></iconify-icon>
                        <div class="ml-3 text-stone-600 truncate">
                            {item.text}
                        </div>
                    </div>
                </button>
            {:else}
                <a
                    href={item.href}
                    class="w-full rounded-lg mb-1 flex h-9 items-center px-3 text-sm transition-colors hover:bg-paper-dark"
                    class:active={isActive(item.href, item.pagespace)}
                >
                    <div class="flex items-center min-w-0 flex-1">
                        <iconify-icon
                            icon={item.icon}
                            class="text-base text-stone-600 font-medium flex-shrink-0"
                        ></iconify-icon>
                        <div class="ml-3 text-stone-600 truncate">
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
        @apply bg-navy/10 text-navy font-medium;
    }

    .active div {
        @apply text-navy;
    }

    .active iconify-icon {
        @apply text-navy;
    }
</style>
