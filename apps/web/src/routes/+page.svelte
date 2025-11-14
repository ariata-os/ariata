<script lang="ts">
	import { Page } from "$lib";
	import ChatInput from "$lib/components/ChatInput.svelte";
	import ModelPicker, {
		type ModelOption,
	} from "$lib/components/ModelPicker.svelte";
	import Markdown from "$lib/components/Markdown.svelte";
	import ToolCall from "$lib/components/ToolCall.svelte";
	import ThinkingIndicator from "$lib/components/ThinkingIndicator.svelte";
	import { getRandomThinkingLabel } from "$lib/utils/thinkingLabels";
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import type { PageData } from "./$types";
	import { chatSessions } from "$lib/stores/chatSessions.svelte";
	import { Chat } from "@ai-sdk/svelte";
	import { DefaultChatTransport } from "ai";

	// Get data from page loader
	let { data }: { data: PageData } = $props();

	// UI state
	let conversationId = $state(data.conversationId);
	let messagesContainer: HTMLDivElement | null = $state(null);
	let scrollContainer: HTMLDivElement | null = $state(null);
	let enableTransitions = $state(false);
	let streamingMessageId = $state<string | null>(null);

	// Thinking state for animated indicator
	let thinkingState = $state<{
		isThinking: boolean;
		label: string;
		messageId: string;
	} | null>(null);

	// Initialize Chat instance (Svelte uses classes instead of hooks)
	const chat = new Chat({
		id: conversationId,
		transport: new DefaultChatTransport({
			api: "/api/chat",
			prepareSendMessagesRequest: ({ id, messages }) => {
				console.log("[prepareSendMessagesRequest] Preparing request with:", {
					conversationId,
					selectedModelId: selectedModel.id,
					messageCount: messages.length
				});
				return {
					body: {
						sessionId: conversationId,
						model: selectedModel.id,
						messages
					}
				};
			}
		}),
		initialMessages: data.messages?.map((msg) => ({
			id: msg.id,
			role: msg.role as "user" | "assistant",
			parts: [{ type: "text" as const, text: msg.content }]
		})) || []
	});

	// Local input state for ChatInput component
	let input = $state("");

	// Update when conversation changes
	$effect(() => {
		console.log('[Page] Data changed:', {
			conversationId: data.conversationId,
			messageCount: data.messages?.length || 0,
			isNew: data.isNew
		});

		// Disable transitions temporarily during navigation
		enableTransitions = false;

		// Update conversation ID
		conversationId = data.conversationId;

		// Update Chat instance's ID
		chat.id = data.conversationId;

		// Update messages in Chat instance using setter
		chat.messages = data.messages?.map((msg) => ({
			id: msg.id,
			role: msg.role as "user" | "assistant",
			parts: [{ type: "text" as const, text: msg.content }]
		})) || [];

		// Update modelLocked based on whether this is a new conversation
		modelLocked = !data.isNew;

		// Re-enable transitions after a brief moment
		setTimeout(() => {
			enableTransitions = true;
		}, 50);
	});

	// Title generation state
	let titleGenerated = $state(false);
	let inactivityTimer: ReturnType<typeof setTimeout> | null = null;
	const INACTIVITY_TIMEOUT = 15 * 60 * 1000; // 15 minutes

	// Model selection state
	let selectedModel: ModelOption = $state({
		id: data.conversation?.model || "claude-sonnet-4-20250514",
		displayName: "Claude 3.5 Sonnet v2",
		provider: "Anthropic",
		description: "Latest and most capable model",
	});
	let modelLocked = $state(!data.isNew);

	// Derived state for layout mode
	let isEmpty = $derived(chat.messages.length === 0);

	// Time-based greeting
	function getTimeBasedGreeting(): string {
		const hour = new Date().getHours();

		if (hour >= 3 && hour < 12) {
			return "Good Morning";
		} else if (hour >= 12 && hour < 17) {
			return "Good Afternoon";
		} else {
			return "Good Evening";
		}
	}

	let greeting = $state(getTimeBasedGreeting());

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		if (chat.messages.length > 0 && scrollContainer) {
			scrollContainer.scrollTo({
				top: scrollContainer.scrollHeight,
				behavior: "smooth",
			});
		}
	});

	// Debug: Track selectedModel changes
	$effect(() => {
		console.log("[chat/[conversationId]] selectedModel changed:", {
			id: selectedModel.id,
			displayName: selectedModel.displayName,
			timestamp: Date.now(),
		});
	});

	// Generate title after first assistant response
	async function generateTitle() {
		if (titleGenerated || chat.messages.length < 2) return;

		try {
			const response = await fetch("/api/sessions/title", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					sessionId: conversationId,
					messages: chat.messages.map((m) => ({
						role: m.role,
						content: m.parts.find(p => p.type === 'text')?.text || '',
					})),
				}),
			});

			if (response.ok) {
				const { title } = await response.json();
				console.log("Generated title:", title);
				titleGenerated = true;
				// Optionally update UI or trigger sidebar refresh
			}
		} catch (error) {
			console.error("Error generating title:", error);
		}
	}

	// Reset inactivity timer and potentially refine title
	function resetInactivityTimer() {
		if (inactivityTimer) {
			clearTimeout(inactivityTimer);
		}

		// Only set timer if we already have a title and messages
		if (titleGenerated && chat.messages.length > 0) {
			inactivityTimer = setTimeout(async () => {
				console.log("15 minutes of inactivity - refining title");
				// Regenerate title with full conversation context
				try {
					const response = await fetch("/api/sessions/title", {
						method: "POST",
						headers: {
							"Content-Type": "application/json",
						},
						body: JSON.stringify({
							sessionId: conversationId,
							messages: chat.messages.map((m) => ({
								role: m.role,
								content: m.parts.find(p => p.type === 'text')?.text || '',
							})),
						}),
					});

					if (response.ok) {
						const { title } = await response.json();
						console.log("Refined title after inactivity:", title);
						// Optionally trigger sidebar refresh
					}
				} catch (error) {
					console.error("Error refining title:", error);
				}
			}, INACTIVITY_TIMEOUT);
		}
	}

	async function handleChatSubmit(value: string) {
		if (!value.trim() || chat.status === 'loading') return;

		// Reset inactivity timer on user activity
		resetInactivityTimer();

		// Lock model after first message
		if (!modelLocked) {
			modelLocked = true;
		}

		// Show thinking indicator
		const thinkingId = `msg_${Date.now()}_thinking`;
		thinkingState = {
			isThinking: true,
			label: getRandomThinkingLabel(),
			messageId: thinkingId,
		};

		// Rotate thinking label every 5 seconds
		const labelRotationInterval = setInterval(() => {
			if (thinkingState?.isThinking) {
				thinkingState.label = getRandomThinkingLabel();
			}
		}, 5000);

		try {
			console.log("[handleChatSubmit] Sending message with model:", selectedModel.id);

			// Send message using Chat class (handles streaming automatically)
			await chat.sendMessage({ text: value.trim() });

			// Clear input
			input = "";

			// Clear thinking indicator
			clearInterval(labelRotationInterval);
			thinkingState = null;

			// Generate title after first exchange
			if (chat.messages.length === 2) {
				await generateTitle();
				// Update URL with conversationId and refresh sidebar
				goto(`/?conversationId=${conversationId}`, {
					replaceState: true,
					noScroll: true,
				});
				// Refresh sidebar to show new conversation
				await chatSessions.refresh();
			}
		} catch (error) {
			console.error("Error sending message:", error);
			// Clear thinking state
			clearInterval(labelRotationInterval);
			thinkingState = null;
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

<Page scrollable={false} className="h-full p-0!">
	<div class="page-container" class:is-empty={isEmpty}>
		<!-- Messages area - scrollable, fades in when not empty -->
		<div
			bind:this={scrollContainer}
			class="flex-1 overflow-y-auto chat-layout"
			class:visible={!isEmpty}
		>
			<div class="messages-container">
				{#each chat.messages as message (message.id)}
					<div class="flex justify-start">
						<div class="message-wrapper" data-role={message.role}>
							{#if message.role === "assistant"}
								<!-- Show thinking indicator if message has no content yet -->
								{#if thinkingState?.isThinking && thinkingState.messageId === message.id && !message.parts.some(p => p.type === 'text' && p.text)}
									<ThinkingIndicator
										label={thinkingState.label}
									/>
								{/if}

								<!-- Render message parts from AI SDK -->
								{#each message.parts as part, i}
									{#if part.type === "text"}
										<div class="text-base text-neutral-900">
											<Markdown content={part.text} />
										</div>
									{:else if part.type.startsWith("tool-") || part.type === "dynamic-tool"}
										<!-- Debug: Log the part to console -->
										{console.log('[Page] Rendering tool part:', $state.snapshot(part))}
										<!-- Render tool invocations -->
										<div class="tool-calls-container mb-3">
											<ToolCall
												tool_name={part.toolName}
												arguments={part.input || part.args}
												result={part.state === "output-available" ? part.output : undefined}
												timestamp={new Date().toISOString()}
											/>
										</div>
									{/if}
								{/each}
							{:else}
								<!-- User messages: concatenate text parts -->
								<div class="text-base whitespace-pre-wrap text-neutral-900">
									{#each message.parts as part}
										{#if part.type === "text"}
											{part.text}
										{/if}
									{/each}
								</div>
							{/if}
						</div>
					</div>
				{/each}

				<!-- Show thinking indicator if waiting for assistant message to be created -->
				{#if thinkingState?.isThinking && !chat.messages.find((m) => m.id === thinkingState.messageId)}
					<div class="flex justify-start">
						<div class="w-full py-3">
							<ThinkingIndicator label={thinkingState.label} />
						</div>
					</div>
				{/if}
			</div>
		</div>

		<!-- ChatInput - animates from center to bottom -->
		<div
			class="chat-input-wrapper"
			class:is-empty={isEmpty}
			class:transitions-enabled={enableTransitions}
		>
			<!-- Hero section - fades out when chat starts -->
			<div
				class="hero-section"
				class:visible={isEmpty}
				class:transitions-enabled={enableTransitions}
			>
				<h1 class="hero-title font-serif text-4xl text-navy mb-6">
					{greeting}, Adam
				</h1>
			</div>

			<ChatInput
				bind:value={input}
				disabled={chat.status === 'loading'}
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
		background: var(--color-paper);
		box-sizing: border-box;
	}

	/* Only apply transitions when explicitly enabled */
	.chat-input-wrapper.transitions-enabled {
		transition:
			bottom 0.6s cubic-bezier(0.4, 0, 0.2, 1),
			transform 0.6s cubic-bezier(0.4, 0, 0.2, 1);
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
	}

	/* Only apply transitions when explicitly enabled */
	.hero-section.transitions-enabled {
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

	.message-wrapper {
		position: relative;
		width: 100%;
		padding: 0.75rem 0 0.75rem 0.875rem; /* py-3 pl-3.5 */
	}

	/* Colored dot indicator for messages */
	.message-wrapper::before {
		content: "";
		position: absolute;
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 9999px;
		left: -0.5rem;
		top: 1.3rem;
	}

	/* Blue dot for user messages - simple alignment */
	.message-wrapper[data-role="user"]::before {
		background-color: rgb(59 130 246); /* blue-500 */
	}

	/* Stone-800 dot for assistant messages - account for markdown p margin */
	.message-wrapper[data-role="assistant"]::before {
		background-color: rgb(41 37 36); /* stone-800 */
	}
</style>
