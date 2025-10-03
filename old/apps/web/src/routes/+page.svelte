<script lang="ts">
	import { Button } from "$lib/components";
	import { marked } from "marked";
	import "iconify-icon";

	interface Message {
		id: string;
		role: "user" | "assistant";
		content: string;
	}

	let messages = $state<Message[]>([]);
	let input = $state("");
	let isLoading = $state(false);
	let textareaRef: HTMLTextAreaElement | null = $state(null);

	async function handleSubmit(event: Event) {
		event.preventDefault();

		if (!input.trim() || isLoading) return;

		// Add user message
		const userMessage: Message = {
			id: `user-${Date.now()}`,
			role: "user",
			content: input,
		};

		messages = [...messages, userMessage];
		const currentInput = input;
		input = "";

		// Reset textarea height
		if (textareaRef) {
			textareaRef.style.height = "auto";
		}

		isLoading = true;

		try {
			// Call API
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
				}),
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			// Read streaming response
			const reader = response.body?.getReader();
			const decoder = new TextDecoder();

			if (!reader) {
				throw new Error("No response body");
			}

			// Create assistant message placeholder
			let assistantContent = "";
			const assistantId = `assistant-${Date.now()}`;
			const assistantMessage: Message = {
				id: assistantId,
				role: "assistant",
				content: "",
			};

			// Add empty message to show loading state ended
			messages = [...messages, assistantMessage];
			const assistantIndex = messages.length - 1;

			let done = false;
			while (!done) {
				const { value, done: doneReading } = await reader.read();
				done = doneReading;

				if (value) {
					const chunk = decoder.decode(value);
					console.log('Received chunk:', chunk);

					// Parse AI SDK stream format (0:"text"\n)
					const lines = chunk.split("\n").filter((line) => line.trim());
					for (const line of lines) {
						console.log('Processing line:', line);
						if (line.startsWith('0:"')) {
							// Extract text between 0:" and "
							const text = line.slice(3, -1).replace(/\\"/g, '"').replace(/\\n/g, '\n');
							console.log('Extracted text:', text);
							assistantContent += text;

							// Update the specific message in the array
							messages[assistantIndex] = {
								...messages[assistantIndex],
								content: assistantContent
							};
							// Trigger reactivity by creating new array
							messages = [...messages];
						}
					}
				}
			}
		} catch (error) {
			console.error("Chat error:", error);
			const errorMessage: Message = {
				id: `error-${Date.now()}`,
				role: "assistant",
				content: "Sorry, I encountered an error. Please try again.",
			};
			messages = [...messages, errorMessage];
		} finally {
			isLoading = false;
		}
	}

	function handleKeyPress(event: KeyboardEvent) {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			if (input.trim() && !isLoading) {
				handleSubmit(event);
			}
		}
	}

	function autoResizeTextarea(event: Event) {
		const textarea = event.target as HTMLTextAreaElement;
		textarea.style.height = "auto";
		const newHeight = Math.min(textarea.scrollHeight, 8 * 24);
		textarea.style.height = `${newHeight}px`;
	}

	function renderMarkdown(text: string): string {
		try {
			return marked.parse(text) as string;
		} catch (error) {
			console.error("Markdown parsing error:", error);
			return text;
		}
	}
</script>

<div class="h-full flex flex-col bg-white">
	<!-- Header -->
	<div class="border-b border-neutral-100 px-4 py-3">
		<div class="max-w-3xl mx-auto flex items-center gap-2">
			<span class="text-neutral-900 font-serif text-sm"
				>Ariata AI Chat</span
			>
		</div>
	</div>

	<!-- Chat interface -->
	<div class="flex-1 overflow-y-auto">
		<div class="max-w-3xl mx-auto px-4 py-8">
			{#if messages.length === 0}
				<div class="text-center text-neutral-500 mt-12">
					<p class="text-lg mb-2">Welcome to Ariata AI</p>
					<p class="text-sm">Start a conversation by typing a message below</p>
				</div>
			{/if}

			{#each messages as message (message.id)}
				<div class="mb-6">
					{#if message.role === "user"}
						<div class="flex justify-end">
							<div
								class="bg-neutral-900 text-white rounded-2xl px-5 py-3 max-w-[80%]"
							>
								<p class="text-sm leading-relaxed">
									{message.content}
								</p>
							</div>
						</div>
					{:else}
						<div class="flex justify-start">
							<div class="max-w-[80%]">
								<div class="bg-neutral-50 rounded-2xl px-5 py-4">
									<div class="prose prose-sm max-w-none">
										{@html renderMarkdown(message.content)}
									</div>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{/each}

			{#if isLoading}
				<div class="flex justify-start mb-6">
					<div class="max-w-[80%]">
						<div class="bg-neutral-50 rounded-2xl px-5 py-4">
							<div class="flex gap-1">
								<div class="w-2 h-2 bg-neutral-400 rounded-full animate-bounce"></div>
								<div class="w-2 h-2 bg-neutral-400 rounded-full animate-bounce" style="animation-delay: 0.1s"></div>
								<div class="w-2 h-2 bg-neutral-400 rounded-full animate-bounce" style="animation-delay: 0.2s"></div>
							</div>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Fixed input at bottom -->
	<div class="border-t border-neutral-100 bg-white px-4 py-4">
		<div class="max-w-3xl mx-auto">
			<form onsubmit={handleSubmit}>
				<div
					class="w-full rounded-xl border border-neutral-200 bg-white flex items-center transition-all duration-300 hover:border-neutral-300 shadow-sm"
				>
					<textarea
						bind:this={textareaRef}
						bind:value={input}
						onkeypress={handleKeyPress}
						oninput={autoResizeTextarea}
						disabled={isLoading}
						class="flex-1 resize-none border-0 bg-transparent placeholder:text-neutral-500 text-neutral-700 focus:ring-0 focus:outline-none px-4 py-3 text-sm disabled:opacity-50"
						placeholder="Type your message..."
						rows={1}
					></textarea>

					<button
						aria-label="Send"
						type="submit"
						class="p-2 m-2 bg-neutral-900 flex items-center justify-center rounded-lg hover:bg-neutral-800 focus:outline-none disabled:opacity-50 cursor-pointer transition-colors"
						disabled={!input.trim() || isLoading}
					>
						<iconify-icon
							icon="ri:send-plane-fill"
							class="text-sm text-white"
						></iconify-icon>
					</button>
				</div>
			</form>
		</div>
	</div>
</div>

<style>
	/* Ensure proper markdown styling */
	:global(.prose) {
		color: rgb(64 64 64);
	}

	:global(.prose strong) {
		color: rgb(23 23 23);
		font-weight: 600;
	}

	:global(.prose pre) {
		background-color: rgb(38 38 38);
		color: rgb(245 245 245);
		border-radius: 0.5rem;
		padding: 1rem;
		overflow-x: auto;
	}

	:global(.prose code) {
		background-color: rgb(229 229 229);
		color: rgb(38 38 38);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
		font-size: 0.875rem;
	}

	:global(.prose pre code) {
		background-color: transparent;
		padding: 0;
		color: inherit;
	}

	:global(.prose p) {
		line-height: 1.625;
	}

	/* Table styles */
	:global(.prose table) {
		width: 100%;
		border-collapse: collapse;
		margin: 1rem 0;
		font-size: 0.875rem;
	}

	:global(.prose th) {
		background-color: rgb(245 245 245);
		font-weight: 600;
		text-align: left;
		padding: 0.5rem 0.75rem;
		border: 1px solid rgb(229 229 229);
	}

	:global(.prose td) {
		padding: 0.5rem 0.75rem;
		border: 1px solid rgb(229 229 229);
	}

	:global(.prose tr:nth-child(even)) {
		background-color: rgb(250 250 250);
	}

	:global(.prose tr:hover) {
		background-color: rgb(245 245 245);
	}
</style>
