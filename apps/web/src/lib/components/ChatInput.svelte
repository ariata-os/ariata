<script lang="ts">
	import { createEventDispatcher } from "svelte";

	export let value = "";
	export let disabled = false;
	export let placeholder = "Message...";
	export let maxWidth = "max-w-2xl";

	const dispatch = createEventDispatcher<{ submit: string }>();

	let textarea: HTMLTextAreaElement;

	function autoResize() {
		if (textarea) {
			textarea.style.height = "auto";
			textarea.style.height = textarea.scrollHeight + "px";
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleSubmit();
		}
	}

	function handleSubmit() {
		if (!value.trim() || disabled) return;
		dispatch("submit", value);
		value = "";
		// Reset to single row height
		if (textarea) {
			textarea.style.height = "auto";
		}
	}
</script>

<div class="chat-input-container {maxWidth} w-full">
	<div
		class="chat-input-wrapper bg-white border border-stone-300 rounded-lg"
	>
		<!-- Textarea Area -->
		<div class="textarea-section pt-4 px-4 pb-1">
			<label for="chat-input" class="sr-only">Message</label>
			<textarea
				id="chat-input"
				bind:this={textarea}
				bind:value
				on:input={autoResize}
				on:keydown={handleKeydown}
				{placeholder}
				{disabled}
				rows="1"
				class="w-full resize-none outline-none text-stone-800 placeholder:text-stone-500 font-sans text-base"
				style="max-height: 200px; overflow-y: auto;"
			></textarea>
		</div>

		<!-- Bottom Action Bar -->
		<div class="action-bar flex items-center justify-between px-2 pb-2">
			<div class="model-picker-slot">
				<slot name="modelPicker" />
			</div>
			<button
				type="button"
				on:click={handleSubmit}
				disabled={!value.trim() || disabled}
				class="send-button w-8 h-8 btn-primary hover:bg-blue cursor-pointer text-white rounded-full disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center group"
			>
				{#if disabled}
					<iconify-icon
						icon="ri:loader-4-line"
						class="animate-spin text-white"
						width="16"
					></iconify-icon>
				{:else}
					<iconify-icon
						icon="ri:arrow-up-line"
						width="16"
						class="text-white transition-transform duration-300 group-hover:rotate-45"
					></iconify-icon>
				{/if}
			</button>
		</div>
	</div>
</div>

<style>
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}

	textarea {
		font-family: var(--font-sans);
		transition: height 0.2s cubic-bezier(0.4, 0, 0.2, 1);
	}

	/* Custom scrollbar for textarea */
	textarea::-webkit-scrollbar {
		width: 6px;
	}

	textarea::-webkit-scrollbar-track {
		background: transparent;
	}

	textarea::-webkit-scrollbar-thumb {
		background: var(--color-stone-300);
		border-radius: 3px;
	}

	textarea::-webkit-scrollbar-thumb:hover {
		background: var(--color-stone-400);
	}

	.chat-input-wrapper {
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.textarea-section {
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
	}
</style>
