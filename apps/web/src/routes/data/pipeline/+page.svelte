<script lang="ts">
    import { Page, Badge } from "$lib/components";
    import { formatDuration, formatBytes, formatRelativeTime, getStatusBadgeVariant } from "$lib/utils/format";
    import type { PageData } from "./$types";
    import "iconify-icon";

    let { data = $bindable() }: { data: PageData } = $props();

    // Get activity type label
    function getActivityTypeLabel(activityType: string): string {
        switch (activityType) {
            case "ingestion":
                return "Data Ingestion";
            case "signal_creation":
                return "Signal Creation";
            case "token_refresh":
                return "Token Refresh";
            case "scheduled_check":
                return "Scheduled Check";
            case "cleanup":
                return "Cleanup";
            case "transition_detection":
                return "Transition Detection";
            default:
                return activityType;
        }
    }

    // Get activity type icon
    function getActivityTypeIcon(activityType: string): string {
        switch (activityType) {
            case "ingestion":
                return "ri:download-cloud-line";
            case "signal_creation":
                return "ri:pulse-line";
            case "token_refresh":
                return "ri:key-line";
            case "scheduled_check":
                return "ri:timer-line";
            case "cleanup":
                return "ri:delete-bin-line";
            case "transition_detection":
                return "ri:git-branch-line";
            default:
                return "ri:settings-line";
        }
    }

    // Get activity type color
    function getActivityTypeColor(activityType: string): string {
        switch (activityType) {
            case "ingestion":
                return "text-blue-600";
            case "signal_creation":
                return "text-purple-600";
            case "token_refresh":
                return "text-amber-600";
            case "scheduled_check":
                return "text-green-600";
            case "cleanup":
                return "text-red-600";
            case "transition_detection":
                return "text-indigo-600";
            default:
                return "text-neutral-600";
        }
    }

    // Get activity description
    function getActivityDescription(activity: any): string {
        switch (activity.activityType) {
            case "ingestion":
                return (
                    activity.streamDisplayName ||
                    activity.streamName ||
                    "Data sync"
                );
            case "signal_creation":
                return activity.signalDisplayName || "Signal processing";
            case "token_refresh":
                const metadata = activity.activityMetadata || {};
                return `${metadata.sources_checked || 0} sources checked`;
            case "scheduled_check":
                const checkMetadata = activity.activityMetadata || {};
                return `${checkMetadata.streams_triggered || 0} syncs triggered`;
            case "cleanup":
                return "Old records cleaned up";
            case "transition_detection":
                return "Transition analysis";
            default:
                return activity.activityName || "System task";
        }
    }

    // Get source icon
    function getSourceIcon(sourceName: string): string {
        const iconMap: Record<string, string> = {
            ios: "ri:apple-line",
            mac: "ri:mac-line",
            google: "ri:google-line",
            plaid: "ri:bank-line",
            apple_ios_core_location: "ri:map-pin-line",
            apple_ios_mic_transcription: "ri:mic-line",
            apple_ios_healthkit: "ri:heart-pulse-line",
            apple_mac_apps: "ri:window-line",
            google_calendar: "ri:calendar-line",
            plaid_transactions: "ri:bank-line",
            system: "ri:server-line",
        };
        return iconMap[sourceName] || "ri:database-line";
    }
</script>

<Page>
    <div class="space-y-6">
        <!-- Header -->
        <div>
            <h1 class="text-3xl font-mono text-neutral-900">Data Pipeline</h1>
            <p class="mt-2 text-base text-neutral-600">
                Real-time view of all system activities
            </p>
        </div>

        <!-- Statistics Cards -->
        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-4">
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Active</p>
                <p class="text-2xl text-neutral-900 font-mono mt-1">
                    {data.stats.activeTotal || 0}
                </p>
            </div>
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Completed</p>
                <p class="text-2xl text-green-600 font-mono mt-1">
                    {data.stats.completedToday || 0}
                </p>
            </div>
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Failed</p>
                <p class="text-2xl text-red-600 font-mono mt-1">
                    {data.stats.failedToday || 0}
                </p>
            </div>
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Success Rate</p>
                <p class="text-2xl text-neutral-900 font-mono mt-1">
                    {data.stats.successRate || 0}%
                </p>
            </div>
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Data Volume</p>
                <p class="text-2xl text-neutral-900 font-mono mt-1">
                    {formatBytes(data.stats.dataVolumeToday || 0)}
                </p>
            </div>
            <div class="bg-white rounded-lg border border-neutral-200 p-4">
                <p class="text-sm font-medium text-neutral-600">Ingestions</p>
                <p class="text-2xl text-blue-600 font-mono mt-1">
                    {data.stats.ingestionsToday || 0}
                </p>
            </div>
        </div>

        <!-- Activity Table -->
        <div
            class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
        >
            {#if data.activities.length === 0}
                <div class="p-12 text-center">
                    <iconify-icon
                        icon="ri:time-line"
                        width="48"
                        height="48"
                        class="text-neutral-400 mx-auto mb-4"
                    ></iconify-icon>
                    <p class="text-neutral-600">No recent activities</p>
                    <p class="text-sm text-neutral-500 mt-1">
                        Pipeline activities will appear here
                    </p>
                </div>
            {:else}
                <div class="overflow-x-auto">
                    <table class="w-full">
                        <thead
                            class="bg-neutral-100 border-b border-neutral-200"
                        >
                            <tr>
                                <th
                                    class="px-4 py-3 text-left text-xs font-mono font-medium text-neutral-600 uppercase tracking-wider"
                                    >Type</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-mono font-medium text-neutral-600 uppercase tracking-wider"
                                    >Status</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-mono font-medium text-neutral-600 uppercase tracking-wider"
                                    >Source</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-mono font-medium text-neutral-600 uppercase tracking-wider"
                                    >Description</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-mono font-medium text-neutral-600 uppercase tracking-wider"
                                    >Time</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider"
                                    >Duration</th
                                >
                                <th
                                    class="px-4 py-3 text-left text-xs font-medium text-neutral-600 uppercase tracking-wider"
                                    >Data</th
                                >
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-neutral-200">
                            {#each data.activities as activity}
                                <tr
                                    class="hover:bg-neutral-50 transition-colors"
                                >
                                    <!-- Type -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        <div class="flex items-center gap-2">
                                            <iconify-icon
                                                icon={getActivityTypeIcon(
                                                    activity.activityType,
                                                )}
                                                width="18"
                                                height="18"
                                                class={getActivityTypeColor(
                                                    activity.activityType,
                                                )}
                                            ></iconify-icon>
                                            <span
                                                class="text-sm text-neutral-900"
                                            >
                                                {getActivityTypeLabel(
                                                    activity.activityType,
                                                )}
                                            </span>
                                        </div>
                                    </td>

                                    <!-- Status -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        <Badge
                                            variant={getStatusBadgeVariant(
                                                activity.status,
                                            )}
                                            size="sm"
                                        >
                                            {activity.status}
                                        </Badge>
                                    </td>

                                    <!-- Source -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        {#if activity.sourceName && activity.sourceName !== "system"}
                                            <div
                                                class="flex items-center gap-1"
                                            >
                                                <iconify-icon
                                                    icon={getSourceIcon(
                                                        activity.sourceName,
                                                    )}
                                                    width="16"
                                                    height="16"
                                                    class="text-neutral-500"
                                                ></iconify-icon>
                                                <span
                                                    class="text-sm text-neutral-700"
                                                >
                                                    {activity.sourceDisplayName ||
                                                        activity.sourceName}
                                                </span>
                                            </div>
                                        {:else}
                                            <span
                                                class="text-sm text-neutral-500"
                                                >System</span
                                            >
                                        {/if}
                                    </td>

                                    <!-- Description -->
                                    <td class="px-4 py-3">
                                        <div class="text-sm text-neutral-700">
                                            {getActivityDescription(activity)}
                                        </div>
                                        {#if activity.errorMessage}
                                            <div
                                                class="mt-1 text-xs text-red-600"
                                            >
                                                <iconify-icon
                                                    icon="ri:error-warning-line"
                                                    class="inline mr-1"
                                                ></iconify-icon>
                                                {activity.errorMessage}
                                            </div>
                                        {/if}
                                    </td>

                                    <!-- Time -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        <span class="text-sm text-neutral-600">
                                            {formatRelativeTime(activity.startedAt)}
                                        </span>
                                    </td>

                                    <!-- Duration -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        <span class="text-sm text-neutral-600">
                                            {formatDuration(
                                                activity.startedAt,
                                                activity.completedAt,
                                            )}
                                        </span>
                                    </td>

                                    <!-- Data -->
                                    <td class="px-4 py-3 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {#if activity.recordsProcessed}
                                                <div>
                                                    {activity.recordsProcessed.toLocaleString()}
                                                    records
                                                </div>
                                            {/if}
                                            {#if activity.dataSizeBytes}
                                                <div>
                                                    {formatBytes(
                                                        activity.dataSizeBytes,
                                                    )}
                                                </div>
                                            {/if}
                                            {#if !activity.recordsProcessed && !activity.dataSizeBytes}
                                                <span class="text-neutral-400"
                                                    >—</span
                                                >
                                            {/if}
                                        </div>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        </div>
    </div>
</Page>
