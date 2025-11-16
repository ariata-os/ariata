<script lang="ts">
	import { onMount } from "svelte";
	import { fade, fly } from "svelte/transition";
	import type { Chat } from "@continuedev/ai-sdk";

	interface Tool {
		id: string;
		name: string;
		description: string | null;
		icon: string | null;
		default_params: Record<string, any> | null;
	}

	interface Props {
		chat: Chat;
		onMountedChange?: (mounted: boolean) => void;
	}

	let { chat, onMountedChange }: Props = $props();

	let tools: Tool[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			const response = await fetch("/api/assistant-profile/pinned-tools");
			if (!response.ok) {
				throw new Error("Failed to fetch pinned tools");
			}
			tools = await response.json();
		} catch (err) {
			console.error("Error loading pinned tools:", err);
			error =
				err instanceof Error
					? err.message
					: "Failed to load pinned tools";
		} finally {
			loading = false;
		}
	});

	async function executeTool(tool: Tool) {
		console.log("[PinnedToolsBar] Executing tool:", tool.id);

		// Create a user message asking to use the tool
		const userMessage = tool.default_params
			? `Show me ${tool.name.toLowerCase()}`
			: `Show me ${tool.name.toLowerCase()}`;

		// Add user message to chat
		await chat.append({
			role: "user",
			content: userMessage,
		});

		// Submit the chat to trigger AI response
		await chat.submit();
	}
</script>

{#if !loading && !error && tools.length > 0}
	<div
		class="flex items-center gap-2 overflow-x-auto scrollbar-hide pb-1"
	>
		{#each tools as tool (tool.id)}
			<button
				onclick={() => executeTool(tool)}
				class="group cursor-pointer flex hover:border-blue items-center gap-2 rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-700 border border-stone-300 transition-all duration-150 whitespace-nowrap flex-shrink-0"
				title={tool.description || tool.name}
			>
				{#if tool.icon}
					<iconify-icon
						icon={tool.icon}
						width="14"
						class="text-stone-500 group-hover:text-stone-700 transition-colors"
					></iconify-icon>
				{/if}
				<span>{tool.name}</span>
			</button>
		{/each}
	</div>
{:else if !loading && tools.length === 0}
	<div
		class="text-xs text-stone-400 text-center py-1"
		transition:fade={{ duration: 200 }}
	>
		Pin your favorite tools in <a
			href="/profile/assistant"
			class="underline hover:text-stone-600">settings</a
		>
	</div>
{/if}

<style>
	.scrollbar-hide {
		-ms-overflow-style: none;
		scrollbar-width: none;
	}
	.scrollbar-hide::-webkit-scrollbar {
		display: none;
	}
</style>
