<script lang="ts">
	import Page from "$lib/components/Page.svelte";
	import ChatInput from "$lib/components/ChatInput.svelte";
	import ModelPicker from "$lib/components/ModelPicker.svelte";
	import AgentPicker from "$lib/components/AgentPicker.svelte";
	import PinnedToolsBar from "$lib/components/PinnedToolsBar.svelte";
	import ContextIndicator from "$lib/components/ContextIndicator.svelte";
	import type { ModelOption } from "$lib/config/models";
	import {
		getModels,
		getSelectedModel,
		setSelectedModel,
		initializeSelectedModel,
		getInitializationPromise,
		isLoading as isModelsLoading,
	} from "$lib/stores/models.svelte";
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

	// UI preferences from assistant profile
	let uiPreferences = $state<{
		contextIndicator?: {
			alwaysVisible?: boolean;
			showThreshold?: number;
		};
	}>({});

	// Thinking state for animated indicator
	let thinkingState = $state<{
		isThinking: boolean;
		label: string;
		messageId: string;
	} | null>(null);
	let labelRotationInterval: ReturnType<typeof setInterval> | null = null;

	// Keep a map of message metadata (agentId, provider, etc.) for rendering
	let messageMetadata = $state<
		Map<string, { agentId?: string; provider?: string }>
	>(new Map());

	// Helper function to convert database messages to Chat parts
	// Formats stored messages to match AI SDK v6 UIMessagePart structure
	function convertMessageToParts(msg: any) {
		// Store metadata for rendering
		if (msg.agentId || msg.provider) {
			messageMetadata.set(msg.id, {
				agentId: msg.agentId,
				provider: msg.provider,
			});
		}

		const parts: any[] = [];

		// Add text content
		if (msg.content) {
			parts.push({
				type: "text" as const,
				text: msg.content,
			});
		}

		// Add tool calls if they exist
		// Tool calls from DB are already completed, so state is always "output-available"
		if (msg.tool_calls && Array.isArray(msg.tool_calls)) {
			console.log(
				"[convertMessageToParts] Processing message with",
				msg.tool_calls.length,
				"tool_calls",
			);
			for (const toolCall of msg.tool_calls) {
				console.log("[convertMessageToParts] Tool call:", {
					tool_name: toolCall.tool_name,
					has_tool_call_id: !!toolCall.tool_call_id,
					tool_call_id: toolCall.tool_call_id,
					has_result: !!toolCall.result,
				});
				parts.push({
					type: `tool-${toolCall.tool_name}` as const,
					toolCallId:
						toolCall.tool_call_id ||
						`${msg.id}_${toolCall.tool_name}_${Date.now()}`,
					toolName: toolCall.tool_name,
					input: toolCall.arguments,
					state: "output-available" as const,
					output: toolCall.result,
				});
			}
		}

		return parts;
	}

	// Initialize Chat instance (Svelte uses classes instead of hooks)
	const chat = new Chat({
		id: conversationId,
		transport: new DefaultChatTransport({
			api: "/api/chat",
			prepareSendMessagesRequest: ({ id, messages }) => {
				console.log(
					"[prepareSendMessagesRequest] Preparing request with:",
					{
						conversationId,
						selectedModelId: selectedModel?.id,
						selectedAgentId: selectedAgent,
						messageCount: messages.length,
					},
				);
				return {
					body: {
						sessionId: conversationId,
						model: selectedModel?.id || "openai/gpt-oss-120b",
						agentId: selectedAgent,
						messages,
					},
				};
			},
		}),
		messages:
			data.messages?.map((msg) => ({
				id: msg.id,
				role: msg.role as "user" | "assistant",
				parts: convertMessageToParts(msg),
			})) || [],
		// Error handling callback
		onError: (error) => {
			console.error("[Chat] Error occurred:", error);
			// Error is automatically stored in chat.error state
		},
	});

	// Local input state for ChatInput component
	let input = $state("");
	let inputFocused = $state(false);

	// Track the last loaded conversation ID to avoid overwriting messages during active chat
	// Initialize to null to ensure first load always triggers the effect
	let lastLoadedConversationId = $state<string | null>(null);

	// Update when conversation changes (navigation to different conversation)
	$effect(() => {
		console.log("[Page] Data changed:", {
			conversationId: data.conversationId,
			messageCount: data.messages?.length || 0,
			isNew: data.isNew,
			lastLoaded: lastLoadedConversationId,
		});

		// Only reload messages if we're navigating to a DIFFERENT conversation
		// Don't overwrite messages during active streaming in the same conversation
		if (data.conversationId !== lastLoadedConversationId) {
			console.log(
				"[Page] Loading new conversation:",
				data.conversationId,
			);

			// Disable transitions temporarily during navigation
			enableTransitions = false;

			// Update conversation ID
			conversationId = data.conversationId;
			lastLoadedConversationId = data.conversationId;

			// Update messages in Chat instance only when switching conversations
			chat.messages =
				data.messages?.map((msg) => ({
					id: msg.id,
					role: msg.role as "user" | "assistant",
					parts: convertMessageToParts(msg),
				})) || [];

			// Update modelLocked based on whether this is a new conversation
			modelLocked = !data.isNew;

			// Re-enable transitions after a brief moment
			setTimeout(() => {
				enableTransitions = true;
			}, 50);
		} else {
			console.log(
				"[Page] Same conversation, keeping existing messages to preserve streaming state",
			);
		}
	});

	// Title generation state
	let titleGenerated = $state(false);
	let inactivityTimer: ReturnType<typeof setTimeout> | null = null;
	const INACTIVITY_TIMEOUT = 15 * 60 * 1000; // 15 minutes

	// Model selection state
	const models = getModels();
	const selectedModel = $derived(getSelectedModel());
	let modelLocked = $state(!data.isNew);

	// Initialize selectedModel once models are loaded
	$effect(() => {
		if (models.length > 0 && !selectedModel) {
			console.log("[+page.svelte] Initializing selected model");
			initializeSelectedModel(data.conversation?.model);
		}
	});

	// Agent selection state (default to 'auto' for intelligent routing)
	let selectedAgent = $state("auto");

	// Context tracking - estimate tokens from message content
	// Rough estimate: ~4 characters per token
	let cumulativeTokens = $state(0);

	// Calculate cumulative tokens from messages
	// Only run after each turn completes (not during streaming) to avoid expensive recalculation
	$effect(() => {
		if (chat.status !== "streaming") {
			if (chat.messages.length > 0) {
				const totalChars = chat.messages.reduce((sum, msg) => {
					const content = msg.parts
						.filter((p: any) => p.type === "text")
						.map((p: any) => p.text)
						.join(" ");
					return sum + content.length;
				}, 0);
				// Rough estimate: 4 chars per token
				cumulativeTokens = Math.ceil(totalChars / 4);
			} else {
				cumulativeTokens = 0;
			}
		}
	});

	// Load assistant profile defaults on mount for new conversations
	onMount(async () => {
		// Wait for models to load first
		await getInitializationPromise();

		try {
			const response = await fetch("/api/assistant-profile");
			if (response.ok) {
				const profile = await response.json();

				// Load UI preferences (for all conversations)
				if (profile.ui_preferences) {
					uiPreferences = profile.ui_preferences;
				}

				// Apply defaults only for new conversations
				if (data.isNew) {
					// Apply default agent if set
					if (profile.default_agent_id) {
						selectedAgent = profile.default_agent_id;
					}

					// Apply default model if set and conversation is new (not locked)
					if (
						profile.default_model_id &&
						!modelLocked &&
						!selectedModel
					) {
						initializeSelectedModel(
							data.conversation?.model,
							profile.default_model_id,
						);
					}
				}
			}
		} catch (error) {
			console.error("Failed to load assistant profile defaults:", error);
			// Continue with hardcoded defaults if profile fetch fails
		}
	});

	// Clear thinking state once assistant starts responding
	$effect(() => {
		if (thinkingState?.isThinking && chat.status === "streaming") {
			const lastMessage = chat.messages[chat.messages.length - 1];
			if (lastMessage?.role === "assistant" && lastMessage.parts.some((p: any) => p.type === "text" && p.text)) {
				console.log("[effect] Clearing thinking state - assistant is responding");
				if (labelRotationInterval) {
					clearInterval(labelRotationInterval);
					labelRotationInterval = null;
				}
				thinkingState = null;
			}
		}
	});

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

	// Debug: Track selectedModel changes
	$effect(() => {
		if (selectedModel) {
			console.log("[chat/[conversationId]] selectedModel changed:", {
				id: selectedModel.id,
				displayName: selectedModel.displayName,
				timestamp: Date.now(),
			});
		}
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
						content:
							m.parts.find((p) => p.type === "text")?.text || "",
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
								content:
									m.parts.find((p) => p.type === "text")
										?.text || "",
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
		if (!value.trim() || chat.status === "streaming") return;

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
		labelRotationInterval = setInterval(() => {
			if (thinkingState?.isThinking) {
				thinkingState.label = getRandomThinkingLabel();
			}
		}, 5000);

		try {
			console.log(
				"[handleChatSubmit] Starting message send with model:",
				selectedModel?.id || "loading...",
			);

			// Send message using Chat class (handles streaming automatically)
			await chat.sendMessage({ text: value.trim() });

			console.log(
				"[handleChatSubmit] Message send completed successfully",
			);

			// Clear input
			input = "";

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
			console.error("[handleChatSubmit] Error sending message:", error);
			// Show error state to user if needed
		} finally {
			// Always clear thinking state
			if (labelRotationInterval) {
				clearInterval(labelRotationInterval);
				labelRotationInterval = null;
			}
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
						<div
							class="message-wrapper"
							data-role={message.role}
							data-agent-id={messageMetadata.get(message.id)
								?.agentId || "general"}
							data-loading={message.role === "assistant" &&
								!message.parts.some(
									(p) => p.type === "text" && p.text,
								)}
						>
							{#if message.role === "assistant"}
								<!-- Show thinking indicator if message has no content yet -->
								{#if thinkingState?.isThinking && thinkingState.messageId === message.id && !message.parts.some((p) => p.type === "text" && p.text)}
									<ThinkingIndicator
										label={thinkingState.label}
									/>
								{/if}

								<!-- Render message parts from AI SDK -->
								{#each message.parts as part, partIndex (part.type.startsWith("tool-") ? (part as any).toolCallId : `text-${partIndex}`)}
									{#if part.type === "text"}
										<div class="text-base text-neutral-900">
											<Markdown
												content={part.text}
												isStreaming={chat.status ===
													"streaming" &&
													message.id ===
														chat.messages[
															chat.messages
																.length - 1
														]?.id}
											/>
										</div>
									{:else if part.type.startsWith("tool-")}
										<!-- Tool invocation rendering based on state -->
										{#if (part as any).state === "output-available"}
											<!-- Render completed tool call -->
											<div
												class="tool-calls-container mb-3"
											>
												<ToolCall
													tool_name={(part as any)
														.toolName}
													arguments={(part as any)
														.input}
													result={(part as any)
														.output}
													timestamp={new Date().toISOString()}
												/>
											</div>
										{:else if (part as any).state === "output-error"}
											<div
												class="tool-calls-container mb-3 text-sm text-red-600"
											>
												Error executing {(part as any)
													.toolName}: {(part as any)
													.errorText ||
													"Unknown error"}
											</div>
										{/if}
									{/if}
								{/each}
							{:else}
								<!-- User messages: concatenate text parts -->
								<div
									class="text-base whitespace-pre-wrap font-serif text-blue"
								>
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
				{#if thinkingState?.isThinking && !chat.messages.find((m) => m.id === thinkingState!.messageId)}
					<div class="flex justify-start">
						<div class="w-full py-3">
							<ThinkingIndicator label={thinkingState.label} />
						</div>
					</div>
				{/if}

				<!-- Error message with retry button -->
				{#if chat.error}
					{@const isRateLimitError =
						chat.error.message?.includes("Rate limit exceeded") ||
						chat.error.message?.includes("rate limit") ||
						chat.error.message?.includes("429")}
					<div class="flex justify-start">
						<div
							class="error-container"
							class:rate-limit-error={isRateLimitError}
						>
							<div class="error-icon">
								<iconify-icon
									icon={isRateLimitError
										? "ri:time-line"
										: "ri:error-warning-line"}
									width="20"
								></iconify-icon>
							</div>
							<div class="error-content">
								<div class="error-title">
									{isRateLimitError
										? "Rate Limit Reached"
										: "An error occurred"}
								</div>
								<div class="error-message">
									{#if isRateLimitError}
										You've reached your API usage limit.
										Please wait for the limit to reset or
										check your usage dashboard for details.
									{:else}
										{chat.error.message ||
											"Something went wrong. Please try again."}
									{/if}
								</div>
								<div class="error-actions">
									{#if isRateLimitError}
										<a href="/usage" class="usage-link">
											<iconify-icon
												icon="ri:bar-chart-line"
												width="16"
											></iconify-icon>
											View Usage Dashboard
										</a>
									{:else}
										<button
											type="button"
											class="retry-button"
											onclick={() => {
												// Clear error and retry last message
												chat.regenerate();
											}}
										>
											<iconify-icon
												icon="ri:refresh-line"
												width="16"
											></iconify-icon>
											Retry
										</button>
									{/if}
								</div>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<!-- ChatInput - animates from center to bottom -->
		<div
			class="chat-input-wrapper"
			class:is-empty={isEmpty}
			class:has-messages={!isEmpty}
			class:transitions-enabled={enableTransitions}
			class:focused={inputFocused}
		>
			<!-- Hero section - fades out when chat starts -->
			<div
				class="hero-section"
				class:visible={isEmpty}
				class:transitions-enabled={enableTransitions}
			>
				<h1
					class="hero-title shiny-title font-serif text-4xl text-navy mb-6"
				>
					{greeting}, Adam
				</h1>
			</div>

			<ChatInput
				bind:value={input}
				bind:focused={inputFocused}
				disabled={chat.status === "streaming"}
				placeholder="Message..."
				maxWidth="max-w-3xl"
				on:submit={(e) => handleChatSubmit(e.detail)}
			>
				{#snippet agentPicker()}
					<AgentPicker
						bind:value={selectedAgent}
						disabled={chat.status === "streaming"}
					/>
				{/snippet}
				{#snippet modelPicker()}
					<ModelPicker
						value={selectedModel}
						disabled={modelLocked}
						onSelect={setSelectedModel}
					/>
				{/snippet}
				{#snippet contextIndicator()}
					{#if selectedModel}
						<ContextIndicator
							{cumulativeTokens}
							contextWindow={selectedModel.contextWindow}
							alwaysVisible={uiPreferences.contextIndicator
								?.alwaysVisible ?? false}
							showThreshold={uiPreferences.contextIndicator
								?.showThreshold ?? 70}
						/>
					{/if}
				{/snippet}
			</ChatInput>

			<!-- Pinned tools bar - only shown in empty state and when input is empty, below chat input -->
			{#if isEmpty}
				<div
					class="pinned-tools-container"
					class:hidden={!!input.trim()}
				>
					<PinnedToolsBar {chat} />
				</div>
			{/if}
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
		position: relative;
		z-index: 1;
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
		gap: 2rem;
		position: relative;
		z-index: 1;
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
		z-index: 10;
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

	/* Reduce bottom padding when there are active messages */
	.chat-input-wrapper.has-messages {
		padding-bottom: 1.5rem;
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

	.pinned-tools-container {
		display: flex;
		justify-content: center;
		width: 100%;
		max-width: 48rem; /* max-w-3xl */
		margin-top: 1.5rem;
		opacity: 1;
		transition: opacity 200ms ease-out;
		pointer-events: auto;
	}

	.pinned-tools-container.hidden {
		opacity: 0;
		pointer-events: none;
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

	/* Hide dot when message is still loading */
	.message-wrapper[data-loading="true"]::before {
		display: none;
	}

	/* Blue dot for user messages - simple alignment */
	.message-wrapper[data-role="user"]::before {
		background-color: rgb(59 130 246); /* blue-500 */
	}

	/* Agent-specific colors for assistant messages */
	.message-wrapper[data-role="assistant"][data-agent-id="analytics"]::before {
		background-color: #3b82f6; /* Analytics blue */
	}

	.message-wrapper[data-role="assistant"][data-agent-id="research"]::before {
		background-color: #8b5cf6; /* Research purple */
	}

	.message-wrapper[data-role="assistant"][data-agent-id="general"]::before {
		background-color: #6b7280; /* General gray */
	}

	.message-wrapper[data-role="assistant"][data-agent-id="action"]::before {
		background-color: #10b981; /* Action green */
	}

	/* Fallback for assistant messages without agentId */
	.message-wrapper[data-role="assistant"]::before {
		background-color: rgb(41 37 36); /* stone-800 */
	}

	/* Error container styles */
	.error-container {
		display: flex;
		gap: 0.75rem;
		padding: 1rem;
		background-color: rgb(254 242 242); /* red-50 */
		border: 1px solid rgb(254 226 226); /* red-100 */
		border-radius: 0.5rem;
		width: 100%;
		max-width: 100%;
	}

	.error-icon {
		flex-shrink: 0;
		color: rgb(220 38 38); /* red-600 */
		margin-top: 0.125rem;
	}

	.error-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.error-title {
		font-weight: 600;
		color: rgb(153 27 27); /* red-900 */
		font-size: 0.875rem;
	}

	.error-message {
		color: rgb(127 29 29); /* red-950 */
		font-size: 0.875rem;
		line-height: 1.5;
	}

	/* Rate limit specific error styles */
	.error-container.rate-limit-error {
		background-color: rgb(254 252 232); /* yellow-50 */
		border-color: rgb(254 249 195); /* yellow-100 */
	}

	.error-container.rate-limit-error .error-icon {
		color: rgb(202 138 4); /* yellow-700 */
	}

	.error-container.rate-limit-error .error-title {
		color: rgb(120 53 15); /* yellow-900 */
	}

	.error-container.rate-limit-error .error-message {
		color: rgb(113 63 18); /* yellow-950 */
	}

	.error-actions {
		margin-top: 0.5rem;
	}

	.retry-button {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.75rem;
		background-color: var(--color-white);
		border: 1px solid rgb(252 165 165); /* red-300 */
		border-radius: 0.375rem;
		color: rgb(153 27 27); /* red-900 */
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s ease;
		align-self: flex-start;
	}

	.retry-button:hover {
		background-color: rgb(254 242 242); /* red-50 */
		border-color: rgb(248 113 113); /* red-400 */
	}

	.retry-button:active {
		transform: scale(0.98);
	}

	.usage-link {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.75rem;
		background-color: var(--color-white);
		border: 1px solid rgb(253 224 71); /* yellow-300 */
		border-radius: 0.375rem;
		color: rgb(120 53 15); /* yellow-900 */
		font-size: 0.875rem;
		font-weight: 500;
		text-decoration: none;
		cursor: pointer;
		transition: all 0.15s ease;
		align-self: flex-start;
	}

	.usage-link:hover {
		background-color: rgb(254 252 232); /* yellow-50 */
		border-color: rgb(250 204 21); /* yellow-400 */
	}

	.usage-link:active {
		transform: scale(0.98);
	}

	.shiny-title {
		overflow: visible;
		padding-bottom: 0.25rem;
	}

	/* Shiny gradient effect for hero title when textarea is focused */
	.chat-input-wrapper.focused .shiny-title {
		background-image: -webkit-linear-gradient(
			left,
			var(--color-blue) 0%,
			var(--color-blue) 25%,
			transparent 50%,
			var(--color-navy) 70%,
			var(--color-navy) 100%
		);
		background-image: linear-gradient(
			90deg,
			var(--color-blue) 0%,
			var(--color-blue) 25%,
			transparent 50%,
			var(--color-navy) 70%,
			var(--color-navy) 100%
		);
		background-position: 100% center;
		background-size: 300% auto;
		-webkit-background-clip: text;
		background-clip: text;
		color: var(--color-navy);
		-webkit-text-fill-color: transparent;
		text-fill-color: transparent;
		animation: shiny-title 1.5s ease-out forwards;
	}

	@keyframes shiny-title {
		0% {
			background-position: 100% center;
		}
		5% {
			background-position: 100% center;
		}
		100% {
			background-position: 0% center;
		}
	}
</style>
