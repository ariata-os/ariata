<script lang="ts">
    import { Page, Badge, Button } from "$lib/components";
    import { formatBytes, formatDate } from "$lib/utils/format";
    import { enhance } from "$app/forms";
    import { slide } from "svelte/transition";
    import { goto } from '$app/navigation';
    import "iconify-icon";

    export let data;
    export let form;

    let viewContent = "";
    let viewFilename = "";
    let showViewer = false;
    let showAudioPlayer = false;
    let audioPath = "";
    let audioFilename = "";
    let selectedEntry: any = null;
    let showRawData = false;

    // Format large numbers with commas
    function formatNumber(num: number): string {
        return num.toLocaleString();
    }

    function closeViewer() {
        showViewer = false;
        viewContent = "";
        viewFilename = "";
    }

    function closeAudioPlayer() {
        showAudioPlayer = false;
        audioPath = "";
        audioFilename = "";
    }

    // Check if a file is an audio file that can be played in browser
    function isAudioFile(filename: string): boolean {
        // Note: .caf and .opus files from iOS are not playable in browsers
        const playableAudioExtensions = [
            ".wav",
            ".m4a",
            ".mp3",
            ".aac",
            ".ogg",
            ".oga",
            ".webm",
            ".flac",
        ];
        const lowerName = filename.toLowerCase();
        return playableAudioExtensions.some((ext) => lowerName.endsWith(ext));
    }

    // Date formatting utilities for stream view
    function formatRelativeTime(date: Date): string {
        const now = new Date();
        const diff = now.getTime() - date.getTime();
        const seconds = Math.floor(diff / 1000);
        const minutes = Math.floor(seconds / 60);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);
        
        if (days > 0) return `${days} day${days > 1 ? 's' : ''} ago`;
        if (hours > 0) return `${hours} hour${hours > 1 ? 's' : ''} ago`;
        if (minutes > 0) return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
        return 'just now';
    }
    

    function selectEntry(key: string) {
        const entry = data.streamKeys.find(k => k.key === key);
        if (entry && data.sampleData) {
            selectedEntry = data.sampleData;
        }
    }

    function getDataPreview(data: any): string {
        if (!data) return 'No data available';
        
        // For array data, show count and first item
        if (Array.isArray(data)) {
            return `Array with ${data.length} items`;
        }
        
        // For object data, show key count
        if (typeof data === 'object') {
            const keys = Object.keys(data);
            return `Object with ${keys.length} fields: ${keys.slice(0, 3).join(', ')}${keys.length > 3 ? '...' : ''}`;
        }
        
        return String(data);
    }

    // Handle form responses
    $: if (form?.type === "view" && form.content && form.filename) {
        viewContent = form.content;
        viewFilename = form.filename;
        showViewer = true;
    } else if (form?.type === "play" && form.path && form.filename) {
        audioPath = form.path;
        audioFilename = form.filename;
        showAudioPlayer = true;
    }
</script>

<Page>
    {#if data.isStream}
        <!-- Stream View -->
        <div class="container mx-auto max-w-7xl">
            <!-- Header -->
            <div class="mb-6">
                <Button variant="ghost" size="sm" on:click={() => goto('/views/streams')}>
                    <iconify-icon icon="ri:arrow-left-line" class="text-base mr-2"></iconify-icon>
                    Back to Streams
                </Button>
            </div>
            
            <div class="mb-8">
                <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100">
                    {data.stream.displayName}
                </h1>
                <p class="text-gray-600 dark:text-gray-400 mt-2">
                    {data.stream.description || `Raw data stream: ${data.stream.streamName}`}
                </p>
            </div>
            
            <!-- Stream Stats -->
            <div class="grid gap-4 md:grid-cols-4 mb-8">
                <div class="border border-neutral-200 rounded-lg p-6 bg-white">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-600 dark:text-gray-400">Total Entries</p>
                            <p class="text-2xl font-bold">{data.stats.totalEntries}</p>
                        </div>
                        <iconify-icon icon="ri:database-2-line" class="text-3xl text-gray-400"></iconify-icon>
                    </div>
                </div>
                
                <div class="border border-neutral-200 rounded-lg p-6 bg-white">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-600 dark:text-gray-400">Data Size</p>
                            <p class="text-2xl font-bold">{formatBytes(data.stats.dataSize)}</p>
                        </div>
                        <iconify-icon icon="ri:database-2-line" class="text-3xl text-gray-400"></iconify-icon>
                    </div>
                </div>
                
                <div class="border border-neutral-200 rounded-lg p-6 bg-white">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-600 dark:text-gray-400">Last Updated</p>
                            <p class="text-lg font-semibold">
                                {#if data.stats.lastModified}
                                    {formatRelativeTime(new Date(data.stats.lastModified))}
                                {:else}
                                    Never
                                {/if}
                            </p>
                        </div>
                        <iconify-icon icon="ri:calendar-line" class="text-3xl text-gray-400"></iconify-icon>
                    </div>
                </div>
                
                <div class="border border-neutral-200 rounded-lg p-6 bg-white">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm text-gray-600 dark:text-gray-400">Type</p>
                            <Badge class="mt-1">
                                {data.stream.ingestionType}
                            </Badge>
                        </div>
                        <iconify-icon icon="ri:refresh-line" class="text-3xl text-gray-400"></iconify-icon>
                    </div>
                </div>
            </div>
            
            <!-- Recent Entries -->
            <div class="border border-neutral-200 rounded-lg bg-white">
                <div class="p-6 border-b border-neutral-200">
                    <h3 class="text-lg font-semibold">Recent Stream Entries</h3>
                </div>
                <div class="p-6">
                    {#if data.streamKeys.length === 0}
                        <p class="text-gray-600 dark:text-gray-400 text-center py-8">
                            No stream data available yet
                        </p>
                    {:else}
                        <div class="space-y-3">
                            {#each data.streamKeys as entry}
                                <div class="flex items-center justify-between p-4 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors">
                                    <div>
                                        <p class="font-medium text-sm">{entry.key.split('/').pop()}</p>
                                        <p class="text-xs text-gray-500 dark:text-gray-400">
                                            {formatDate(new Date(entry.lastModified))} • {formatBytes(entry.size)}
                                        </p>
                                    </div>
                                    <div class="flex gap-2">
                                        <Button 
                                            variant="ghost" 
                                            size="sm"
                                            on:click={() => selectEntry(entry.key)}
                                        >
                                            <iconify-icon icon="ri:eye-line" class="text-base mr-1"></iconify-icon>
                                            Preview
                                        </Button>
                                        <Button 
                                            variant="ghost" 
                                            size="sm"
                                            disabled
                                            title="Download coming soon"
                                        >
                                            <iconify-icon icon="ri:download-line" class="text-base"></iconify-icon>
                                        </Button>
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            </div>
            
            <!-- Data Preview -->
            {#if data.sampleData}
                <div class="border border-neutral-200 rounded-lg bg-white mt-6">
                    <div class="p-6 border-b border-neutral-200">
                        <div class="flex items-center justify-between">
                            <h3 class="text-lg font-semibold">Sample Data</h3>
                            <Button 
                                variant="ghost" 
                                size="sm"
                                on:click={() => showRawData = !showRawData}
                            >
                                {showRawData ? 'Hide' : 'Show'} Raw JSON
                            </Button>
                        </div>
                    </div>
                    <div class="p-6">
                        {#if showRawData}
                            <pre class="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto text-xs">
{JSON.stringify(data.sampleData, null, 2)}
                            </pre>
                        {:else}
                            <div class="space-y-4">
                                <!-- Data summary -->
                                <div class="grid gap-4 md:grid-cols-2">
                                    <div>
                                        <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Data Type</p>
                                        <p class="mt-1">{getDataPreview(data.sampleData)}</p>
                                    </div>
                                    
                                    {#if data.sampleData.device_id}
                                        <div>
                                            <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Device ID</p>
                                            <p class="mt-1 font-mono text-sm">{data.sampleData.device_id}</p>
                                        </div>
                                    {/if}
                                    
                                    {#if data.sampleData.timestamp || data.sampleData.fetched_at}
                                        <div>
                                            <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Timestamp</p>
                                            <p class="mt-1">
                                                {formatDate(new Date(data.sampleData.timestamp || data.sampleData.fetched_at))}
                                            </p>
                                        </div>
                                    {/if}
                                    
                                    {#if data.sampleData.total_records || data.sampleData.total_points}
                                        <div>
                                            <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Records</p>
                                            <p class="mt-1">{data.sampleData.total_records || data.sampleData.total_points}</p>
                                        </div>
                                    {/if}
                                </div>
                                
                                <!-- Show data array preview if it exists -->
                                {#if data.sampleData.data && Array.isArray(data.sampleData.data) && data.sampleData.data.length > 0}
                                    <div>
                                        <p class="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">
                                            First Record Preview
                                        </p>
                                        <div class="bg-gray-100 dark:bg-gray-800 p-3 rounded text-sm">
                                            {#each Object.entries(data.sampleData.data[0]) as [key, value]}
                                                <div class="flex justify-between py-1">
                                                    <span class="font-medium">{key}:</span>
                                                    <span class="text-gray-600 dark:text-gray-400">{value}</span>
                                                </div>
                                            {/each}
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>
    {:else}
        <!-- File Browser View -->
        <div class="flex gap-6 h-full">
            <!-- Left Panel - Directory Browser -->
            <div
                class="{showViewer || showAudioPlayer
                    ? 'w-1/2'
                    : 'w-full'} transition-all duration-300 flex flex-col space-y-6"
            >
                <div class="flex justify-between items-start">
                    <div>
                        <h1 class="text-3xl text-neutral-900 mb-2 font-mono">
                            Raw Storage
                        </h1>
                        <!-- <div class="flex gap-4 text-sm text-neutral-600">
                            <span>Total Used: {formatBytes(data.bucketStats?.totalSize || 0)} & Total Objects: {formatNumber(data.bucketStats?.totalObjects || 0)}</span>
                        </div> -->
                    </div>
                    <div class="text-sm text-neutral-500">
                        {data.items.length} items in current directory
                    </div>
                </div>

                <!-- Directory/Files Table -->
                <div class="flex-1">
                    <div class="bg-white rounded-lg overflow-hidden">
                        <div class="overflow-x-auto">
                            <table class="w-full">
                                <thead class="bg-neutral-100">
                                    <tr>
                                        <th
                                            class="px-4 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider font-mono"
                                        >
                                            Name
                                        </th>
                                        <th
                                            class="px-4 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider font-mono {showViewer ||
                                            showAudioPlayer
                                                ? 'hidden'
                                                : ''}"
                                        >
                                            Type
                                        </th>
                                        <th
                                            class="px-4 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider font-mono {showViewer ||
                                            showAudioPlayer
                                                ? 'hidden'
                                                : ''}"
                                        >
                                            Size
                                        </th>
                                        <th
                                            class="px-4 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider font-mono {showViewer ||
                                            showAudioPlayer
                                                ? 'hidden'
                                                : ''}"
                                        >
                                            Last Modified
                                        </th>
                                        <th
                                            class="px-4 py-3 text-left text-xs font-medium text-neutral-500 uppercase tracking-wider font-mono"
                                        >
                                            Actions
                                        </th>
                                    </tr>
                                </thead>
                                <tbody class="bg-white">
                                    {#each data.items as item}
                                        <tr class="hover:bg-neutral-50">
                                            <td
                                                class="px-4 py-3 text-sm text-neutral-900"
                                            >
                                                {#if item.type === "directory"}
                                                    <a
                                                        href="/data/raw/{item.path}"
                                                        class="flex items-center text-neutral-700 hover:text-neutral-900 font-medium"
                                                    >
                                                        <iconify-icon
                                                            icon="ri:folder-fill"
                                                            class="w-4 h-4 mr-2 text-neutral-600"
                                                        ></iconify-icon>
                                                        {item.name}
                                                    </a>
                                                {:else}
                                                    <span
                                                        class="flex items-center font-mono text-sm"
                                                    >
                                                        <iconify-icon
                                                            icon="ri:file-text-line"
                                                            class="w-4 h-4 mr-2 text-neutral-400"
                                                        ></iconify-icon>
                                                        {item.name}
                                                    </span>
                                                {/if}
                                            </td>
                                            <td
                                                class="px-4 py-3 text-sm text-neutral-600 {showViewer ||
                                                showAudioPlayer
                                                    ? 'hidden'
                                                    : ''}"
                                            >
                                                {item.type === "directory"
                                                    ? "Directory"
                                                    : "File"}
                                            </td>
                                            <td
                                                class="px-4 py-3 text-sm text-neutral-600 font-mono {showViewer ||
                                                showAudioPlayer
                                                    ? 'hidden'
                                                    : ''}"
                                            >
                                                {item.type === "file" && item.size
                                                    ? formatBytes(item.size)
                                                    : "-"}
                                            </td>
                                            <td
                                                class="px-4 py-3 text-sm text-neutral-600 {showViewer ||
                                                showAudioPlayer
                                                    ? 'hidden'
                                                    : ''}"
                                            >
                                                {item.lastModified
                                                    ? formatDate(item.lastModified)
                                                    : "-"}
                                            </td>
                                            <td class="px-4 py-3 text-sm">
                                                {#if item.type === "file"}
                                                    <div class="flex items-center gap-2">
                                                        {#if item.name.endsWith(".json") || item.name.endsWith(".json.gz")}
                                                            <form
                                                                method="POST"
                                                                action="?/view"
                                                                use:enhance
                                                                class="inline-block"
                                                            >
                                                                <input
                                                                    type="hidden"
                                                                    name="file"
                                                                    value={item.path}
                                                                />
                                                                <button
                                                                    type="submit"
                                                                    class="text-neutral-600 hover:text-neutral-900 transition-colors"
                                                                    title="View JSON"
                                                                >
                                                                    <iconify-icon
                                                                        icon="ri:eye-line"
                                                                        class="w-4 h-4"
                                                                    ></iconify-icon>
                                                                </button>
                                                            </form>
                                                        {/if}
                                                        {#if isAudioFile(item.name)}
                                                            <form
                                                                method="POST"
                                                                action="?/play"
                                                                use:enhance
                                                                class="inline-block"
                                                            >
                                                                <input
                                                                    type="hidden"
                                                                    name="file"
                                                                    value={item.path}
                                                                />
                                                                <button
                                                                    type="submit"
                                                                    class="text-neutral-600 hover:text-neutral-900 transition-colors"
                                                                    title="Play Audio"
                                                                >
                                                                    <iconify-icon
                                                                        icon="ri:play-circle-line"
                                                                        class="w-4 h-4"
                                                                    ></iconify-icon>
                                                                </button>
                                                            </form>
                                                        {/if}
                                                        <a
                                                            href="/api/raw/download?file={encodeURIComponent(
                                                                item.path
                                                            )}"
                                                            download
                                                            class="text-neutral-600 hover:text-neutral-900 transition-colors"
                                                            title="Download file"
                                                        >
                                                            <iconify-icon
                                                                icon="ri:download-line"
                                                                class="w-4 h-4"
                                                            ></iconify-icon>
                                                        </a>
                                                    </div>
                                                {:else}
                                                    <!-- For directories -->
                                                    <span class="text-neutral-400 text-xs"
                                                        >-</span
                                                    >
                                                {/if}
                                            </td>
                                        </tr>
                                    {/each}
                                </tbody>
                            </table>

                            {#if data.items.length === 0}
                                <div
                                    class="text-center py-8 text-neutral-500 text-sm"
                                >
                                    This directory is empty
                                </div>
                            {/if}
                        </div>
                    </div>
                </div>
            </div>

            <!-- Right Panel - File Viewer/Audio Player -->
            {#if showViewer || showAudioPlayer}
                <div
                    class="w-1/2 bg-white rounded-lg p-6"
                    transition:slide={{ duration: 300 }}
                >
                    {#if showViewer}
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-medium text-neutral-900">
                                {viewFilename}
                            </h3>
                            <button
                                on:click={closeViewer}
                                class="text-neutral-500 hover:text-neutral-700 transition-colors"
                                title="Close viewer"
                            >
                                <iconify-icon
                                    icon="ri:close-line"
                                    class="w-5 h-5"
                                ></iconify-icon>
                            </button>
                        </div>
                        <div
                            class="bg-neutral-100 rounded-md p-4 overflow-auto max-h-[calc(100vh-250px)]"
                        >
                            <pre
                                class="text-xs text-neutral-800 font-mono whitespace-pre-wrap">{viewContent}</pre>
                        </div>
                    {/if}

                    {#if showAudioPlayer}
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-medium text-neutral-900">
                                {audioFilename}
                            </h3>
                            <button
                                on:click={closeAudioPlayer}
                                class="text-neutral-500 hover:text-neutral-700 transition-colors"
                                title="Close player"
                            >
                                <iconify-icon
                                    icon="ri:close-line"
                                    class="w-5 h-5"
                                ></iconify-icon>
                            </button>
                        </div>
                        <div class="space-y-4">
                            <audio
                                controls
                                class="w-full"
                                src="/api/raw/stream?file={encodeURIComponent(
                                    audioPath
                                )}"
                            >
                                Your browser does not support the audio element.
                            </audio>
                            <div class="text-sm text-neutral-600">
                                <p>
                                    <strong>Note:</strong> Audio playback is streamed
                                    directly from storage.
                                </p>
                                <p class="mt-1">
                                    <a
                                        href="/api/raw/download-audio?file={encodeURIComponent(
                                            audioPath
                                        )}"
                                        download
                                        class="text-blue-600 hover:text-blue-700 underline"
                                    >
                                        Download Audio File
                                    </a>
                                </p>
                            </div>
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
    {/if}
</Page>