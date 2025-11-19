<script lang="ts">
    import { page } from "$app/state";
    import { chatSessions as chatSessionsStore } from "$lib/stores/chatSessions.svelte";

    interface Props {
        chatSessions: typeof chatSessionsStore;
    }

    let { chatSessions }: Props = $props();

    interface BreadcrumbItem {
        label: string;
        href: string;
    }

    let breadcrumbs = $derived.by(() => {
        const path = page.url.pathname;
        const segments = path.split("/").filter(Boolean);

        const items: BreadcrumbItem[] = [{ label: "Chat", href: "/" }];

        // If on root path with conversationId, add conversation title
        if (path === "/" && page.url.searchParams.has("conversationId")) {
            const conversationId = page.url.searchParams.get("conversationId");
            const conversation = chatSessions.sessions.find(
                (s: any) => s.conversation_id === conversationId
            );
            if (conversation && conversation.title) {
                items.push({
                    label: conversation.title,
                    href: `/?conversationId=${conversationId}`
                });
            }
        }

        // Build breadcrumb path for other routes
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

<nav aria-label="Breadcrumb" class="flex items-center space-x-1 text-base">
    {#each breadcrumbs as crumb, index}
        {#if index > 0}
            <span class="text-neutral-400 px-2 text-sm">
                {"/"}
            </span>
        {/if}

        {#if index === breadcrumbs.length - 1}
            <span class="text-neutral-700 font-sans text-sm">
                {crumb.label}
            </span>
        {:else}
            <a
                href={crumb.href}
                class="text-neutral-600 font-sans text-sm hover:text-neutral-700 transition-colors"
            >
                {crumb.label}
            </a>
        {/if}
    {/each}
</nav>
