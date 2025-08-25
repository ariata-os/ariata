<script lang="ts">
	// Hot reload test - should update immediately!
	import { Button } from "$lib/components";
	import { marked } from "marked";
	import "iconify-icon";

	interface Message {
		id: string;
		content: string;
		sender: "user" | "bot";
		createdAt: Date;
		metadata?: any;
		showActions?: boolean;
	}

	// State management
	let messages = $state<Message[]>([
		{
			id: "user-initial",
			content: "Book a rental car for my upcoming conference",
			sender: "user",
			createdAt: new Date(Date.now() - 5000), // 5 seconds ago
		},
		{
			id: "bot-initial",
			content: `Hi Adam, I see a flight confirmation for the SENT Ventures conference just came in. I've taken a look at rental cars for your trip.

Here's what I found:

**Logistics:** Based on your flights <span class="source-badge">Gmail</span>, I've set the pickup for 10:00 AM on the 24th and return for 4:00 PM on the 27th to give you a comfortable buffer.

**Preferences:** I've automatically excluded 'Thrifty Rentals' due to your past 1-star rating <span class="source-badge">Yelp Reviews</span> and used your average rental cost of ~$75/day as a budget baseline <span class="source-badge">Plaid Transactions</span>.

**Goal Alignment:** Since one of your initiatives this year is 'intentional dating' <span class="source-badge">Axiological Framework</span>, I prioritized premium sedans. A more comfortable car could be a great asset if you happen to meet someone for coffee.

**Recommendation:** With that in mind, I found a Hertz Full-Size Sedan available for your exact travel times for $82/day.

Would you like to review the details and book using your Amex card on file?`,
			sender: "bot",
			createdAt: new Date(),
			showActions: true
		}
	]);
	let inputMessage = $state("");
	let textareaRef: HTMLTextAreaElement | null = $state(null);

	function handleKeyPress(event: KeyboardEvent) {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			if (inputMessage.trim()) {
				onSend();
			}
		}
	}

	function autoResizeTextarea(event: Event) {
		const textarea = event.target as HTMLTextAreaElement;
		textarea.style.height = "auto";
		const newHeight = Math.min(textarea.scrollHeight, 8 * 24);
		textarea.style.height = `${newHeight}px`;
	}

	async function onSend() {
		if (!inputMessage.trim()) return;

		const userMessage: Message = {
			id: `user-${Date.now()}`,
			content: inputMessage,
			sender: "user",
			createdAt: new Date(),
		};

		messages = [...messages, userMessage];
		inputMessage = "";

		// Reset textarea height
		if (textareaRef) {
			textareaRef.style.height = "auto";
		}

		// Simulate bot response
		setTimeout(() => {
			const botMessage: Message = {
				id: `bot-${Date.now()}`,
				content: `I understand you'd like to know more about that. Let me help you with your question.`,
				sender: "bot",
				createdAt: new Date(),
			};
			messages = [...messages, botMessage];
		}, 1000);
	}

	function handleBooking() {
		// Hide actions on the initial message
		messages = messages.map(msg => 
			msg.id === 'bot-initial' ? { ...msg, showActions: false } : msg
		);

		// Add user confirmation message
		const userMessage: Message = {
			id: `user-${Date.now()}`,
			content: "Yes, book it",
			sender: "user",
			createdAt: new Date(),
		};
		messages = [...messages, userMessage];

		// Add bot confirmation message
		setTimeout(() => {
			const botMessage: Message = {
				id: `bot-${Date.now()}`,
				content: `Perfect! I'm booking the Hertz Full-Size Sedan for you now.

**Booking Details:**
- Pickup: January 24th at 10:00 AM
- Return: January 27th at 4:00 PM  
- Total Cost: $246 ($82/day × 3 days)
- Payment: Amex ending in 4242

You'll receive a confirmation email shortly at adam@example.com. The reservation includes free cancellation up to 24 hours before pickup.

Is there anything else you'd like me to help with for your trip?`,
				sender: "bot",
				createdAt: new Date(),
			};
			messages = [...messages, botMessage];
		}, 1500);
	}

	function handleOtherOptions() {
		// Hide actions on the initial message
		messages = messages.map(msg => 
			msg.id === 'bot-initial' ? { ...msg, showActions: false } : msg
		);

		// Add user message
		const userMessage: Message = {
			id: `user-${Date.now()}`,
			content: "Show me other options",
			sender: "user",
			createdAt: new Date(),
		};
		messages = [...messages, userMessage];

		// Add bot response with alternatives
		setTimeout(() => {
			const botMessage: Message = {
				id: `bot-${Date.now()}`,
				content: `Of course! Here are some alternative options for your trip:

**Premium Options:**
- Avis Premium SUV - $95/day (Great for client dinners)
- Enterprise Luxury Sedan - $89/day (BMW 3 Series available)

**Budget-Friendly Options:**
- Budget Standard Car - $58/day (Toyota Corolla or similar)
- National Midsize - $65/day (Your usual go-to option)

**Electric Options:**
- Hertz Tesla Model 3 - $110/day (Aligns with your sustainability goals)

All prices include the same pickup/return times and free cancellation. Which category interests you most?`,
				sender: "bot",
				createdAt: new Date(),
			};
			messages = [...messages, botMessage];
		}, 1000);
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
			<span class="text-neutral-900 font-mono text-sm">Ariata AI: Booking Transportation</span>
		</div>
	</div>

	<!-- Chat interface -->
	<div class="flex-1 overflow-y-auto">
		<div class="max-w-3xl mx-auto px-4 py-8">
			{#each messages as message (message.id)}
				<div class="mb-6">
					{#if message.sender === "user"}
						<div class="flex justify-end">
							<div class="bg-neutral-900 text-white rounded-2xl px-5 py-3 max-w-[80%]">
								<p class="text-sm leading-relaxed">{message.content}</p>
								<div class="text-[10px] text-neutral-400 mt-1 font-mono">
									{new Date(message.createdAt).toLocaleTimeString([], {
										hour: "2-digit",
										minute: "2-digit",
									})}
								</div>
							</div>
						</div>
					{:else}
						<div class="flex justify-start">
							<div class="max-w-[80%]">
								<div class="bg-neutral-50 rounded-2xl px-5 py-4">
									<div class="prose prose-sm max-w-none">
										{@html renderMarkdown(message.content)}
									</div>
									<div class="text-[10px] text-neutral-400 mt-2 font-mono">
										{new Date(message.createdAt).toLocaleTimeString([], {
											hour: "2-digit",
											minute: "2-digit",
										})}
									</div>
								</div>
								
								{#if message.showActions}
									<div class="flex gap-3 mt-4">
										<Button
											text="Yes, book it"
											variant="filled"
											onclick={handleBooking}
											className="text-sm"
										/>
										<Button
											text="Show me other options"
											variant="outline"
											onclick={handleOtherOptions}
											className="text-sm"
										/>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>

	<!-- Fixed input at bottom -->
	<div class="border-t border-neutral-100 bg-white px-4 py-4">
		<div class="max-w-3xl mx-auto">
			<div class="w-full rounded-xl border border-neutral-200 bg-white flex items-center transition-all duration-300 hover:border-neutral-300 shadow-sm">
				<textarea
					bind:this={textareaRef}
					bind:value={inputMessage}
					onkeypress={handleKeyPress}
					oninput={autoResizeTextarea}
					class="flex-1 resize-none border-0 bg-transparent placeholder:text-neutral-500 text-neutral-700 focus:ring-0 focus:outline-none px-4 py-3 text-sm"
					placeholder="Ask a follow-up question..."
					rows={1}
				></textarea>

				<button
					aria-label="Send"
					onclick={() => onSend()}
					class="p-2 m-2 bg-neutral-900 flex items-center justify-center rounded-lg hover:bg-neutral-800 focus:outline-none disabled:opacity-50 cursor-pointer transition-colors"
					disabled={!inputMessage.trim()}
					type="button"
				>
					<iconify-icon
						icon="ri:send-plane-fill"
						class="text-sm text-white"
					></iconify-icon>
				</button>
			</div>
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

	/* Custom styles for data source citations */
	:global(.prose p) {
		line-height: 1.625;
	}

	/* Source badge styles */
	:global(.source-badge) {
		display: inline-flex;
		align-items: center;
		padding: 0.0625rem 0.375rem;
		margin: 0 0.25rem;
		background-color: rgb(229 229 229);
		color: rgb(64 64 64);
		border-radius: 9999px;
		font-size: 0.625rem;
		font-weight: 500;
		white-space: nowrap;
		vertical-align: middle;
	}

	:global(.prose .source-badge) {
		margin: 0 0.125rem;
	}
</style>