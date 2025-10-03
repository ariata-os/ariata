<script lang="ts">
    import { getTimelineContext } from "./TimelineContext.svelte";
    
    interface Props {
        coordinateSignals: any[];
        selectedDate: string;
        userTimezone: string;
    }
    
    let { coordinateSignals, selectedDate, userTimezone }: Props = $props();
    
    const { timeToPixel } = getTimelineContext();
</script>

<div class="absolute inset-0 pointer-events-none z-10">
    <!-- Background line showing no data -->
    <div class="absolute top-1/2 -translate-y-1/2 h-px w-full bg-neutral-300"></div>
    
    <!-- Data availability segments -->
    {#if coordinateSignals.length > 1}
        {#each coordinateSignals as signal, i}
            {#if i < coordinateSignals.length - 1}
                {@const x1 = timeToPixel(new Date(signal.timestamp))}
                {@const x2 = timeToPixel(new Date(coordinateSignals[i + 1].timestamp))}
                {@const gap = x2 - x1}
                <!-- Only show line if points are reasonably close (less than 5 minutes apart) -->
                {@const timeDiff = new Date(coordinateSignals[i + 1].timestamp).getTime() - new Date(signal.timestamp).getTime()}
                {#if timeDiff < 5 * 60 * 1000}
                    <div 
                        class="absolute top-1/2 -translate-y-1/2 h-1 bg-blue-600/50"
                        style="left: {x1}px; width: {gap}px;"
                    ></div>
                {/if}
            {/if}
        {/each}
    {/if}
    
    <!-- Individual data points -->
    {#each coordinateSignals as signal}
        {@const x = timeToPixel(new Date(signal.timestamp))}
        <div 
            class="absolute top-1/2 -translate-y-1/2 w-1.5 h-1.5 bg-blue-600 rounded-full"
            style="left: {x}px; margin-left: -3px;"
        ></div>
    {/each}
</div>