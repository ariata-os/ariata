<script lang="ts">
	import { Page } from "$lib";
	import ChatInput from "$lib/components/ChatInput.svelte";
	import ModelPicker, { type ModelOption } from "$lib/components/ModelPicker.svelte";
	import Markdown from "$lib/components/Markdown.svelte";
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import type { PageData } from "./$types";

	// Get data from page loader
	let { data }: { data: PageData } = $props();

	// Chat state - initialize from loaded data
	let messages: Array<{ role: string; content: string; id: string }> = $state(
		data.messages || [],
	);
	let input = $state("");
	let isLoading = $state(false);
	let conversationId = $state(data.conversationId);
	let messagesContainer: HTMLDivElement | null = $state(null);
	let scrollContainer: HTMLDivElement | null = $state(null);

	// Title generation state
	let titleGenerated = $state(false);
	let inactivityTimer: ReturnType<typeof setTimeout> | null = null;
	const INACTIVITY_TIMEOUT = 15 * 60 * 1000; // 15 minutes

	// Model selection state
	let selectedModel: ModelOption = $state({
		id: data.conversation?.model || "claude-sonnet-4-20250514",
		displayName: "Claude Sonnet 4",
		provider: "Anthropic",
		description: "Balanced performance and speed",
	});
	let modelLocked = $state(!data.isNew);

	// Derived state for layout mode
	let isEmpty = $derived(messages.length === 0);

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		if (messages.length > 0 && scrollContainer) {
			scrollContainer.scrollTo({ top: scrollContainer.scrollHeight, behavior: 'smooth' });
		}
	});

	// Debug: Track selectedModel changes
	$effect(() => {
		console.log('[chat/[conversationId]] selectedModel changed:', {
			id: selectedModel.id,
			displayName: selectedModel.displayName,
			timestamp: Date.now()
		});
	});

	// Generate title after first assistant response
	async function generateTitle() {
		if (titleGenerated || messages.length < 2) return;

		try {
			const response = await fetch('/api/sessions/title', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					conversationId,
					messages: messages.map(m => ({ role: m.role, content: m.content }))
				})
			});

			if (response.ok) {
				const { title } = await response.json();
				console.log('Generated title:', title);
				titleGenerated = true;
				// Optionally update UI or trigger sidebar refresh
			}
		} catch (error) {
			console.error('Error generating title:', error);
		}
	}

	// Reset inactivity timer and potentially refine title
	function resetInactivityTimer() {
		if (inactivityTimer) {
			clearTimeout(inactivityTimer);
		}

		// Only set timer if we already have a title and messages
		if (titleGenerated && messages.length > 0) {
			inactivityTimer = setTimeout(async () => {
				console.log('15 minutes of inactivity - refining title');
				// Regenerate title with full conversation context
				try {
					const response = await fetch('/api/sessions/title', {
						method: 'POST',
						headers: {
							'Content-Type': 'application/json',
						},
						body: JSON.stringify({
							conversationId,
							messages: messages.map(m => ({ role: m.role, content: m.content }))
						})
					});

					if (response.ok) {
						const { title } = await response.json();
						console.log('Refined title after inactivity:', title);
						// Optionally trigger sidebar refresh
					}
				} catch (error) {
					console.error('Error refining title:', error);
				}
			}, INACTIVITY_TIMEOUT);
		}
	}

	async function handleChatSubmit(value: string) {
		if (!value.trim() || isLoading) return;

		// Reset inactivity timer on user activity
		resetInactivityTimer();

		const userMessage = {
			role: "user",
			content: value.trim(),
			id: `msg_${Date.now()}_user`,
		};

		// Add user message to UI
		messages = [...messages, userMessage];
		input = "";
		isLoading = true;

		// Lock model after first message
		if (!modelLocked) {
			modelLocked = true;
		}

		try {
			// Debug: Log model being sent
			console.log('[handleChatSubmit] Sending request with model:', {
				id: selectedModel.id,
				displayName: selectedModel.displayName,
				modelLocked: modelLocked,
				timestamp: Date.now()
			});

			// Send to API
			const response = await fetch("/api/chat", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					messages: messages.map((m) => ({
						role: m.role,
						content: m.content,
					})),
					conversationId,
					model: selectedModel.id,
				}),
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			// Create assistant message placeholder
			const assistantMessage = {
				role: "assistant",
				content: "",
				id: `msg_${Date.now()}_assistant`,
			};
			messages = [...messages, assistantMessage];

			// Stream the response (simple text stream)
			const reader = response.body?.getReader();
			const decoder = new TextDecoder();

			if (reader) {
				while (true) {
					const { done, value } = await reader.read();
					if (done) break;

					const chunk = decoder.decode(value);
					// Append text directly to the last message (assistant)
					messages[messages.length - 1].content += chunk;
				}
			}

			// Generate title after first exchange (user + assistant)
			if (messages.length === 2) {
				await generateTitle();
			}
		} catch (error) {
			console.error("Error sending message:", error);
			// Add error message
			messages = [
				...messages,
				{
					role: "assistant",
					content:
						"Sorry, there was an error processing your request.",
					id: `msg_${Date.now()}_error`,
				},
			];
		} finally {
			isLoading = false;
		}
	}

	// Cleanup timer on unmount
	onMount(() => {
		return () => {
			if (inactivityTimer) {
				clearTimeout(inactivityTimer);
			}
		};
	});
</script>

<Page className="h-full p-0!">
	<div class="page-container" class:is-empty={isEmpty}>
		<!-- Messages area - scrollable, fades in when not empty -->
		<div bind:this={scrollContainer} class="flex-1 overflow-y-auto chat-layout" class:visible={!isEmpty}>
			<div class="messages-container">
				{#each messages as message (message.id)}
					<div class="flex justify-start">
						<div class="w-full py-3">
							{#if message.role === "assistant"}
								<div class="text-base text-neutral-900">
									{#if message.content}
										<Markdown content={message.content} />
									{:else}
										<span class="text-neutral-400">...</span>
									{/if}
								</div>
							{:else}
								<div class="text-base whitespace-pre-wrap text-neutral-900">
									{message.content}
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- ChatInput - animates from center to bottom -->
		<div class="chat-input-wrapper" class:is-empty={isEmpty}>
			<!-- Hero section - fades out when chat starts -->
			<div class="hero-section" class:visible={isEmpty}>
				<h1 class="hero-title font-serif text-5xl text-neutral-900 mb-4">
					Virtues
				</h1>
				<p class="hero-description text-neutral-600 text-lg mb-8">
					Your personal AI that knows your facts, values, and patterns.
				</p>
			</div>

			<ChatInput
				bind:value={input}
				disabled={isLoading}
				placeholder="Message..."
				maxWidth="max-w-3xl"
				on:submit={(e) => handleChatSubmit(e.detail)}
			>
				{#snippet modelPicker()}
					<ModelPicker
						bind:value={selectedModel}
						disabled={modelLocked}
					/>
				{/snippet}
			</ChatInput>
		</div>
	</div>
</Page>

<style>
	.page-container {
		height: 100%;
		position: relative;
	}

	.chat-layout {
		height: 100%;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.2s ease-in-out;
	}

	.chat-layout.visible {
		opacity: 1;
		pointer-events: auto;
	}

	.messages-container {
		max-width: 48rem; /* max-w-3xl */
		margin: 0 auto;
		width: 100%;
		padding: 3rem 3rem 12rem 3rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.chat-input-wrapper {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		margin: 0 auto;
		width: 100%;
		max-width: 48rem; /* max-w-3xl */
		padding: 0 3rem 3rem 3rem;
		background: white;
		box-sizing: border-box;
		transition: bottom 0.6s cubic-bezier(0.4, 0, 0.2, 1), transform 0.6s cubic-bezier(0.4, 0, 0.2, 1);
	}

	/* When empty, center the input vertically */
	.chat-input-wrapper.is-empty {
		bottom: auto;
		top: 50%;
		transform: translateY(-50%);
	}

	/* When not empty, make it sticky so it doesn't scroll */
	.page-container:not(.is-empty) .chat-input-wrapper {
		position: sticky;
	}

	.hero-section {
		text-align: center;
		opacity: 0;
		max-height: 0;
		overflow: hidden;
		transition:
			opacity 0.3s ease-in-out,
			max-height 0.3s ease-in-out;
	}

	.hero-section.visible {
		opacity: 1;
		max-height: 300px;
	}

	.hero-title {
		text-align: center;
	}

	.hero-description {
		text-align: center;
	}
</style>
