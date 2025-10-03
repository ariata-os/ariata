<script lang="ts">
	let { children } = $props();
</script>

<!-- Settings layout wrapper - currently just passes through children -->
<!-- Can be extended in the future for settings-specific layout needs -->
{@render children()}