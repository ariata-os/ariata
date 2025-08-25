<script lang="ts">
    import { twMerge } from "tailwind-merge";
    let { children, className = "" } = $props();
</script>

<div class={twMerge("page-container p-12", className)}>
    {@render children()}
</div>

<style>
    .page-container {
        animation: slideInFromRight 0.3s ease-out forwards;
    }

    @keyframes slideInFromRight {
        from {
            opacity: 0;
            transform: translateX(8px);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }
</style>
