<script lang="ts">
    import { Page, Badge } from "$lib/components";
    import "iconify-icon";
    import type { PageData } from "./$types";
    import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import { getNextCronExecution, formatRelativeTime, formatCountdown, getLockedFrequencyDescription } from '$lib/utils/cron';
    import { onMount } from 'svelte';

    let { data }: { data: PageData } = $props();
    
    // Current time for countdowns - updates every second
    let currentTime = $state(new Date());
    
    // Update timer every second
    onMount(() => {
        const interval = setInterval(() => {
            currentTime = new Date();
        }, 1000);
        
        return () => clearInterval(interval);
    });

    // Process connected streams with timing data
    const connectedStreamsWithTiming = $derived.by(() => {
        return (data.diagramData?.connectedStreams || []).map(stream => {
            const schedule = stream.syncSchedule || stream.cronSchedule;
            let nextSyncAt: Date | null = null;
            let scheduleDescription = 'Manual sync';
            
            if (schedule && stream.lastSyncAt) {
                try {
                    nextSyncAt = getNextCronExecution(schedule, new Date(stream.lastSyncAt));
                    scheduleDescription = getLockedFrequencyDescription(schedule);
                } catch (e) {
                    // Keep defaults if parsing fails
                }
            }
            
            return {
                ...stream,
                nextSyncAt,
                scheduleDescription,
                lastSyncFormatted: stream.lastSyncAt ? formatRelativeTime(new Date(stream.lastSyncAt), currentTime) : 'Never',
                nextSyncCountdown: nextSyncAt ? formatCountdown(nextSyncAt, currentTime) : '-'
            };
        }).sort((a, b) => {
            // Sort by next sync time (soonest first)
            if (!a.nextSyncAt && !b.nextSyncAt) return 0;
            if (!a.nextSyncAt) return 1;
            if (!b.nextSyncAt) return -1;
            return a.nextSyncAt.getTime() - b.nextSyncAt.getTime();
        });
    });

    // Convert our data to xyflow nodes and edges
    const nodes = $derived.by(() => {
        const nodeList: Node[] = [];
        let nodeId = 0;

        // Add source nodes
        data.diagramData?.sources?.forEach((source, index) => {
            nodeList.push({
                id: `source_${source.name}`,
                position: { x: 0, y: index * 120 },
                data: { 
                    label: source.displayName || source.name
                },
                type: 'default',
                style: 'background: white; border: 2px solid #525252; border-radius: 8px; padding: 12px; min-width: 150px;'
            });
        });

        // Add stream nodes
        data.diagramData?.streams?.forEach((stream, index) => {
            nodeList.push({
                id: `stream_${stream.streamName}`,
                position: { x: 250, y: index * 80 },
                data: { 
                    label: stream.displayName || stream.streamName
                },
                type: 'default',
                style: 'background: white; border: 1px solid #3b82f6; border-radius: 6px; padding: 10px; min-width: 140px;'
            });
        });

        // Add signal nodes
        data.diagramData?.signals?.forEach((signal, index) => {
            nodeList.push({
                id: `signal_${signal.id}`,
                position: { x: 500, y: index * 60 },
                data: { 
                    label: signal.displayName || signal.signalName
                },
                type: 'default',
                style: 'background: white; border: 1px solid #10b981; border-radius: 6px; padding: 8px; min-width: 130px; font-size: 12px;'
            });
        });

        // Add semantic nodes
        data.diagramData?.semantics?.forEach((semantic, index) => {
            nodeList.push({
                id: `semantic_${semantic.id}`,
                position: { x: 500, y: ((data.diagramData?.signals?.length || 0) + index) * 60 },
                data: { 
                    label: `📝 ${semantic.displayName || semantic.semanticName}`
                },
                type: 'default',
                style: 'background: white; border: 1px dashed #8b5cf6; border-radius: 6px; padding: 8px; min-width: 130px; font-size: 12px;'
            });
        });

        return nodeList;
    });

    const edges = $derived.by(() => {
        const edgeList: Edge[] = [];

        // Source to Stream edges
        data.diagramData?.sources?.forEach((source) => {
            const streams = data.diagramData?.streamsBySource?.[source.name] || [];
            streams.forEach((stream) => {
                edgeList.push({
                    id: `edge_${source.name}_${stream.streamName}`,
                    source: `source_${source.name}`,
                    target: `stream_${stream.streamName}`,
                    animated: stream.status === 'active',
                    style: 'stroke: #a3a3a3; stroke-dasharray: 4 4;'
                });
            });
        });

        // Stream to Signal edges
        data.diagramData?.streams?.forEach((stream) => {
            const signalNames = data.diagramData?.signalsByStream?.[stream.streamName] || [];
            const signals = (data.diagramData?.signals || []).filter((s) =>
                signalNames.includes(s.signalName)
            );
            
            signals.forEach((signal) => {
                edgeList.push({
                    id: `edge_${stream.streamName}_${signal.id}`,
                    source: `stream_${stream.streamName}`,
                    target: `signal_${signal.id}`,
                    animated: signal.status === 'active',
                    style: 'stroke: #3b82f6; stroke-dasharray: 4 4;'
                });
            });

            // Stream to Semantic edges
            const semantics = (data.diagramData?.semantics || []).filter((s) =>
                s.streamName === stream.streamName
            );
            
            semantics.forEach((semantic) => {
                edgeList.push({
                    id: `edge_${stream.streamName}_semantic_${semantic.id}`,
                    source: `stream_${stream.streamName}`,
                    target: `semantic_${semantic.id}`,
                    animated: semantic.status === 'active',
                    style: 'stroke: #8b5cf6; stroke-dasharray: 2 2;'
                });
            });
        });

        return edgeList;
    });


    // Get value type color
    function getValueTypeColor(valueType: string): string {
        switch (valueType) {
            case "continuous":
                return "#10B981"; // Green
            case "event":
                return "#3B82F6"; // Blue
            case "categorical":
                return "#8B5CF6"; // Purple
            case "binary":
                return "#F59E0B"; // Amber
            case "spatial":
                return "#EC4899"; // Pink
            case "count":
                return "#06B6D4"; // Cyan
            default:
                return "#6B7280"; // Gray
        }
    }

    // Get status badge variant
    function getStatusVariant(
        status: string,
    ): "success" | "warning" | "error" | "default" {
        switch (status) {
            case "active":
                return "success";
            case "pending_setup":
                return "warning";
            case "error":
                return "error";
            default:
                return "default";
        }
    }
</script>

<Page>
    <div class="">
        <!-- Header -->
        <div class="flex justify-between items-center mb-8">
            <div>
                <h1 class="text-3xl text-neutral-900 font-mono">Data Overview</h1>
                <p class="text-neutral-600 mt-2">
                    Overview of your connected sources, streams, and signals
                </p>
            </div>

            <!-- Stats -->
            <div class="grid grid-cols-2 gap-4">
                <div class="text-center">
                    <div class="text-2xl font-mono text-neutral-900">
                        {data.diagramData?.stats?.activeSources || 0}
                    </div>
                    <div class="text-sm text-neutral-600">Active Sources</div>
                </div>
                <div class="text-center">
                    <div class="text-2xl font-mono text-neutral-900">
                        {data.diagramData?.stats?.activeStreams || 0}
                    </div>
                    <div class="text-sm text-neutral-600">Active Streams</div>
                </div>
            </div>
        </div>

        <!-- Active Sources Table -->
        {#if data.diagramData?.activeSources && data.diagramData?.activeSources?.length > 0}
            <div class="mb-8 bg-white rounded-lg border border-neutral-200 overflow-hidden">
                <div class="px-6 py-4 border-b border-neutral-200">
                    <div class="flex items-center justify-between">
                        <div>
                            <h2 class="text-lg font-mono text-neutral-900">
                                Active Sources
                            </h2>
                            <p class="text-sm text-neutral-600 mt-1">
                                Currently connected and authenticated data sources
                            </p>
                        </div>
                        <div class="text-sm text-neutral-600">
                            {data.diagramData.activeSources.length} source{data.diagramData.activeSources.length !== 1 ? 's' : ''}
                        </div>
                    </div>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full">
                        <thead class="bg-neutral-100">
                            <tr>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Source
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Type
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Status
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Last Sync
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Actions
                                </th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-neutral-200">
                            {#each data.diagramData.activeSources as source}
                                <tr class="hover:bg-neutral-50">
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div>
                                            <div class="text-sm font-mono text-neutral-900">
                                                {source.instanceName}
                                            </div>
                                            <div class="text-xs text-neutral-500">
                                                {source.description || source.sourceName}
                                            </div>
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {source.sourceName}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge variant={getStatusVariant(source.status)} size="sm">
                                            {source.status}
                                        </Badge>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {source.lastSyncAt ? formatRelativeTime(new Date(source.lastSyncAt), currentTime) : 'Never'}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <a 
                                            href="/data/sources/{source.id}" 
                                            class="text-sm text-blue-600 hover:text-blue-800 font-mono"
                                        >
                                            View →
                                        </a>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>
        {/if}

        <!-- Connected Streams Table -->
        {#if connectedStreamsWithTiming && connectedStreamsWithTiming.length > 0}
            <div class="mb-8 bg-white rounded-lg border border-neutral-200 overflow-hidden">
                <div class="px-6 py-4 border-b border-neutral-200">
                    <div class="flex items-center justify-between">
                        <div>
                            <h2 class="text-lg font-mono text-neutral-900">
                                Connected Streams
                            </h2>
                            <p class="text-sm text-neutral-600 mt-1">
                                Real-time sync status for active data streams
                            </p>
                        </div>
                        <div class="text-sm text-neutral-600">
                            {connectedStreamsWithTiming.length} stream{connectedStreamsWithTiming.length !== 1 ? 's' : ''}
                        </div>
                    </div>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full">
                        <thead class="bg-neutral-100">
                            <tr>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Stream
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Status
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Last Sync
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Next Sync
                                </th>
                                <th scope="col" class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider">
                                    Schedule
                                </th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-neutral-200">
                            {#each connectedStreamsWithTiming as stream}
                                <tr class="hover:bg-neutral-50">
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="flex items-center">
                                            <div>
                                                <div class="text-sm font-mono text-neutral-900">
                                                    {stream.streamDisplayName || stream.streamName}
                                                </div>
                                                <div class="text-xs text-neutral-500">
                                                    {stream.sourceDisplayName || stream.sourceInstanceName} • {stream.sourceName}
                                                </div>
                                            </div>
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge 
                                            variant={stream.lastSyncStatus === 'success' ? 'success' : 
                                                   stream.lastSyncStatus === 'failed' ? 'error' : 
                                                   stream.lastSyncStatus === 'in_progress' ? 'warning' : 'default'} 
                                            size="sm"
                                        >
                                            {stream.lastSyncStatus || 'unknown'}
                                        </Badge>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {stream.lastSyncFormatted}
                                        </div>
                                        {#if stream.lastSyncError}
                                            <div class="text-xs text-red-600 mt-1" title={stream.lastSyncError}>
                                                Error occurred
                                            </div>
                                        {/if}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm font-mono text-neutral-900">
                                            {stream.nextSyncCountdown}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {stream.scheduleDescription}
                                        </div>
                                        {#if stream.syncSchedule || stream.cronSchedule}
                                            <div class="text-xs text-neutral-400 font-mono">
                                                {stream.syncSchedule || stream.cronSchedule}
                                            </div>
                                        {/if}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>
        {/if}

        <!-- Flow Diagram -->
        <div class="bg-white rounded-lg border border-neutral-200 overflow-hidden" style="height: 600px;">
            <SvelteFlow 
                {nodes} 
                {edges}
                fitView
                attributionPosition="bottom-left"
            >
                <Background variant="dots" gap={16} size={1} />
                <Controls />
                <MiniMap />
            </SvelteFlow>
        </div>

        <!-- Legend -->
        <div class="mt-6 flex items-center gap-8">
            <div class="flex items-center gap-2">
                <div
                    class="w-3 h-3 rounded-full"
                    style="background-color: {getValueTypeColor('continuous')}"
                ></div>
                <span class="text-sm text-neutral-600">Continuous</span>
            </div>
            <div class="flex items-center gap-2">
                <div
                    class="w-3 h-3 rounded-full"
                    style="background-color: {getValueTypeColor('event')}"
                ></div>
                <span class="text-sm text-neutral-600">Event</span>
            </div>
            <div class="flex items-center gap-2">
                <div
                    class="w-3 h-3 rounded-full"
                    style="background-color: {getValueTypeColor('categorical')}"
                ></div>
                <span class="text-sm text-neutral-600">Categorical</span>
            </div>
            <div class="flex items-center gap-2">
                <div
                    class="w-3 h-3 rounded border border-purple-500"
                    style="background-color: white; border-style: dashed"
                ></div>
                <span class="text-sm text-neutral-600">Semantic</span>
            </div>
        </div>

        <!-- Active Signals Table -->
        {#if data.diagramData?.signals && data.diagramData?.signals?.length > 0}
            {@const activeSignals = data.diagramData?.signals?.filter(s => s.status === 'active')}
            {#if activeSignals.length > 0}
                <div class="mt-8 bg-white rounded-lg border border-neutral-200 overflow-hidden">
                    <div class="px-6 py-4">
                        <h2 class="text-lg font-mono text-neutral-900">
                            Active Signals
                        </h2>
                    </div>
                    <table class="min-w-full">
                        <thead class="bg-neutral-100 border-y border-stone-200">
                            <tr>
                                <th
                                    scope="col"
                                    class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Signal
                                </th>
                                <th
                                    scope="col"
                                    class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Source
                                </th>
                                <th
                                    scope="col"
                                    class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Type
                                </th>
                                <th
                                    scope="col"
                                    class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Unit
                                </th>
                                <th
                                    scope="col"
                                    class="px-6 py-3 text-left text-xs font-mono font-medium text-neutral-500 uppercase tracking-wider"
                                >
                                    Status
                                </th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-neutral-200">
                            {#each activeSignals as signal}
                                <tr>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm font-mono text-neutral-900">
                                            {signal.displayName}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {signal.sourceName}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge variant="default" size="sm">
                                            {signal.computation?.value_type || 'unknown'}
                                        </Badge>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm text-neutral-600">
                                            {signal.unitUcum || '-'}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <Badge variant={getStatusVariant(signal.status)} size="sm">
                                            {signal.status}
                                        </Badge>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        {/if}
    </div>
</Page>