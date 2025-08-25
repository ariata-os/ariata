<script lang="ts">
    interface Tab {
        id: string;
        label: string;
        icon?: string;
    }

    interface Props {
        tabs: Tab[];
        activeTab: string;
        onTabChange?: (tabId: string) => void;
    }

    let { tabs, activeTab = $bindable(), onTabChange }: Props = $props();

    function handleTabClick(tabId: string) {
        activeTab = tabId;
        onTabChange?.(tabId);
    }
</script>

<div class="border-b border-neutral-200">
    <nav class="-mb-px flex space-x-8" aria-label="Tabs">
        {#each tabs as tab}
            <button
                type="button"
                onclick={() => handleTabClick(tab.id)}
                class={`
                    group cursor-pointer inline-flex items-center py-4 px-1 border-b-2 font-mono text-sm font-medium transition-colors
                    ${
                        activeTab === tab.id
                            ? "border-neutral-600 text-neutral-900"
                            : "border-transparent text-neutral-600 hover:text-neutral-900 hover:border-neutral-300"
                    }
                `}
                aria-current={activeTab === tab.id ? "page" : undefined}
            >
                {#if tab.icon}
                    <iconify-icon
                        icon={tab.icon}
                        width="16"
                        height="16"
                        class="mr-2 {activeTab === tab.id
                            ? 'text-blue-600'
                            : 'text-neutral-500 group-hover:text-neutral-700'}"
                    ></iconify-icon>
                {/if}
                <span>{tab.label}</span>
            </button>
        {/each}
    </nav>
</div>

<div class="mt-6">
    <slot />
</div>
