<script lang="ts">
	import { createEventDispatcher } from "svelte";

	let {
		value = $bindable(""),
		disabled = false,
		sendDisabled = false,
		placeholder = "Message...",
		maxWidth = "max-w-3xl",
		focused = $bindable(false),
	} = $props();

	const dispatch = createEventDispatcher<{ submit: string }>();

	let textarea: HTMLTextAreaElement;
	let isFocused = $state(false);

	// Sync internal focus state with external bindable prop
	$effect(() => {
		focused = isFocused;
	});

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

	function handleWrapperClick(e: MouseEvent) {
		// Don't focus if clicking on a button, dropdown, or interactive element
		const target = e.target as HTMLElement;
		if (
			target.tagName === "BUTTON" ||
			target.closest("button") ||
			target.closest(".pickers-slot") || // Don't focus when clicking in the pickers area
			target.classList.contains("z-50") || // Don't focus when clicking dropdown menus
			target.closest(".z-50") // Don't focus when clicking inside dropdown menus
		) {
			return;
		}
		// Focus the textarea
		if (textarea) {
			textarea.focus();
		}
	}
</script>

<div class="chat-input-container {maxWidth} w-full">
	<div
		aria-label="Chat input"
		class="chat-input-wrapper bg-white border border-stone-300 rounded-xl shadow-sm transition-all duration-300 hover:border-blue-200 hover:shadow-blue-200/50 cursor-text"
		class:focused={isFocused}
		onclick={handleWrapperClick}
		role="button"
		tabindex="-1"
	>
		<!-- Textarea Area -->
		<div class="textarea-section pt-4 px-4 pb-1">
			<label for="chat-input" class="sr-only">Message</label>
			<textarea
				id="chat-input"
				bind:this={textarea}
				bind:value
				oninput={autoResize}
				onkeydown={handleKeydown}
				onfocus={() => {
					isFocused = true;
				}}
				onblur={() => {
					isFocused = false;
				}}
				{placeholder}
				{disabled}
				rows="1"
				class="chat-textarea w-full resize-none outline-none text-stone-800 placeholder:text-stone-500 font-sans text-base"
				style="max-height: 200px; overflow-y: auto;"
			></textarea>
		</div>

		<!-- Bottom Action Bar -->
		<div
			class="action-bar flex items-center justify-between px-2 pb-2 pt-2"
		>
			<div class="pickers-slot flex items-center gap-2">
				<slot name="agentPicker" />
				<slot name="contextIndicator" />
			</div>
			<button
				type="button"
				onclick={handleSubmit}
				disabled={!value.trim() || sendDisabled}
				class="send-button w-8 h-8 btn-primary hover:bg-blue cursor-pointer text-white rounded-full disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center group"
			>
				{#if sendDisabled}
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
		transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.chat-input-wrapper.focused {
		border-color: var(--color-blue) !important;
		box-shadow:
			0 1px 2px 0 rgb(0 0 0 / 0.05),
			0 0 0 3px rgba(40, 131, 222, 0.3) !important;
	}

	.textarea-section {
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
	}
</style>
