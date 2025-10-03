<script lang="ts">
    import { Page, Badge, Button } from "$lib/components";
    import { page } from "$app/stores";
    import { formatDate, formatRelativeTime } from "$lib/utils/format";
    import "iconify-icon";
    import type { PageData } from "./$types";

    let { data }: { data: PageData } = $props();

    // Timer for real-time "last seen" updates
    let currentTime = $state(new Date());

    $effect(() => {
        const timer = setInterval(() => {
            currentTime = new Date();
        }, 1000);

        return () => clearInterval(timer);
    });

    // Get status badge variant for a source
    function getStatusBadgeVariant(
        status: string | null,
    ): "success" | "error" | "warning" | "default" | "info" {
        switch (status) {
            case "active":
                return "success";
            case "authenticated":
                return "warning";
            case "paused":
                return "default";
            case "needs_reauth":
            case "error":
                return "error";
            default:
                return "default";
        }
    }

    // Get status display text for a source
    function getStatusText(status: string | null): string {
        switch (status) {
            case "authenticated":
                return "Setup Required";
            case "active":
                return "Active";
            case "paused":
                return "Paused";
            case "needs_reauth":
                return "Reconnect";
            case "error":
                return "Error";
            default:
                return "Inactive";
        }
    }

    // Get connected sources (flattened list of all instances)
    const connectedSources = $derived(
        data.sources
            .filter((s) => s.is_connected)
            .flatMap((source) => {
                if (
                    source.multiple_connections &&
                    source.instances &&
                    source.instances.length > 0
                ) {
                    // Return all instances for sources that support multiple connections
                    return source.instances.map((instance) => ({
                        id: instance.id,
                        sourceName: source.name,
                        sourceDisplayName: source.display_name,
                        instanceName: instance.instanceName,
                        icon: source.icon,
                        platform: source.platform,
                        status: instance.status,
                        lastSyncAt: instance.lastSyncAt,
                        createdAt: instance.createdAt,
                    }));
                } else if (source.is_connected) {
                    // Fallback for sources without instances array (shouldn't happen now)
                    return [
                        {
                            id: source.id,
                            sourceName: source.name,
                            sourceDisplayName: source.display_name,
                            instanceName:
                                source.device_name || source.display_name,
                            icon: source.icon,
                            platform: source.platform,
                            status: source.status,
                            lastSyncAt: source.last_seen,
                            createdAt: null,
                        },
                    ];
                }
                return [];
            }),
    );

    // Summary stats
    const totalSources = $derived(connectedSources.length);
    const activeSources = $derived(
        connectedSources.filter((s) => s.status === "active").length,
    );
</script>

<Page>
    <div class="space-y-6">
        <!-- Header -->
        <div>
            <div class="flex items-center justify-between mb-4">
                <div>
                    <h1 class="text-3xl text-neutral-900 font-serif">
                        Data Sources
                    </h1>
                    <p class="text-neutral-600 mt-1">
                        Manage your connected data sources and devices
                    </p>
                </div>
                <Button
                    type="link"
                    href="/data/sources/catalog"
                    text="+ Add Source"
                    variant="filled"
                />
            </div>
        </div>

        {#if data.error}
            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                <p class="text-red-700">{data.error}</p>
            </div>
        {/if}

        <!-- Summary Stats -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="bg-white border border-neutral-200 rounded-lg p-4">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm text-neutral-500 font-medium">
                            Total Sources
                        </p>
                        <p
                            class="text-2xl font-bold text-neutral-900 font-sans mt-1"
                        >
                            {totalSources}
                        </p>
                    </div>
                    <iconify-icon
                        icon="ri:database-2-line"
                        class="text-3xl text-neutral-400"
                    ></iconify-icon>
                </div>
            </div>
            <div class="bg-white border border-neutral-200 rounded-lg p-4">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm text-neutral-500 font-medium">
                            Active
                        </p>
                        <p
                            class="text-2xl font-bold text-green-600 font-sans mt-1"
                        >
                            {activeSources}
                        </p>
                    </div>
                    <iconify-icon
                        icon="ri:pulse-line"
                        class="text-3xl text-green-400"
                    ></iconify-icon>
                </div>
            </div>
        </div>

        <!-- Connected Sources Table -->
        {#if connectedSources.length > 0}
            <div
                class="bg-white border border-neutral-200 rounded-lg overflow-hidden"
            >
                <div
                    class="px-6 py-4 bg-neutral-50 border-b border-neutral-200"
                >
                    <h2 class="text-lg font-serif text-neutral-900">
                        Connected Sources
                    </h2>
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full">
                        <thead
                            class="bg-neutral-50 border-b border-neutral-200"
                        >
                            <tr>
                                <th
                                    class="px-6 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Source
                                </th>
                                <th
                                    class="px-6 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Instance
                                </th>
                                <th
                                    class="px-6 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Platform
                                </th>
                                <th
                                    class="px-6 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Status
                                </th>
                                <th
                                    class="px-6 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Last Sync
                                </th>
                                <th
                                    class="px-6 py-3 text-right text-xs font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Actions
                                </th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-neutral-200">
                            {#each connectedSources as source}
                                <tr
                                    class="hover:bg-neutral-50 transition-colors"
                                >
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="flex items-center gap-2">
                                            {#if source.icon}
                                                <iconify-icon
                                                    icon={source.icon}
                                                    class="text-xl text-neutral-700"
                                                ></iconify-icon>
                                            {/if}
                                            <span
                                                class="text-sm font-medium text-neutral-900"
                                            >
                                                {source.sourceDisplayName}
                                            </span>
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <span class="text-sm text-neutral-700">
                                            {source.instanceName}
                                        </span>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge variant="default" size="sm">
                                            {source.platform}
                                        </Badge>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge
                                            variant={getStatusBadgeVariant(
                                                source.status,
                                            )}
                                            size="sm"
                                        >
                                            {getStatusText(source.status)}
                                        </Badge>
                                    </td>
                                    <td
                                        class="px-6 py-4 whitespace-nowrap text-sm text-neutral-600"
                                    >
                                        {#if source.lastSyncAt}
                                            {formatRelativeTime(
                                                source.lastSyncAt,
                                                currentTime,
                                            )}
                                        {:else}
                                            Never
                                        {/if}
                                    </td>
                                    <td
                                        class="px-6 py-4 whitespace-nowrap text-right"
                                    >
                                        <div class="flex justify-end gap-2">
                                            <Button
                                                href={`/data/sources/${source.id}`}
                                                text="View"
                                                variant="secondary"
                                                size="sm"
                                            />
                                            {#if source.status === "active"}
                                                <Button
                                                    text="Pause"
                                                    variant="secondary"
                                                    size="sm"
                                                    disabled
                                                />
                                            {:else if source.status === "paused"}
                                                <Button
                                                    text="Resume"
                                                    variant="primary"
                                                    size="sm"
                                                    disabled
                                                />
                                            {/if}
                                        </div>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>
        {:else}
            <!-- Empty State -->
            <div
                class="bg-neutral-100 border border-neutral-200 rounded-lg p-12 text-center"
            >
                <iconify-icon
                    icon="ri:database-2-line"
                    class="text-6xl text-neutral-400 mx-auto mb-4"
                ></iconify-icon>
                <h3 class="text-xl font-serif text-neutral-900 mb-2">
                    No sources connected
                </h3>
                <p class="text-neutral-600 mb-6">
                    Connect your first data source to start your data
                    sovereignty journey
                </p>
            </div>
        {/if}

        <!-- Footer -->
        <div class="pt-4">
            <p class="text-sm text-neutral-500">
                {connectedSources.length} source{connectedSources.length !== 1
                    ? "s"
                    : ""} connected
            </p>
        </div>
    </div>
</Page>
