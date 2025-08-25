<script lang="ts">
    import { getTimelineContext } from "./TimelineContext.svelte";
    
    interface Props {
        onHoverChange: (time: Date | null) => void;
    }
    
    let { onHoverChange }: Props = $props();
    
    const { state } = getTimelineContext();
    
    $effect(() => {
        const hoveredTime = $state.isCursorLocked ? $state.lockedTime : $state.hoveredTime;
        onHoverChange(hoveredTime);
    });
</script>

<!-- This component has no visual output, it just watches the timeline state -->