<script lang="ts">
	/**
	 * BaseTool - Accordion wrapper for all tool result displays
	 * Provides consistent UI for expand/collapse, status, and timing
	 */
	import type { Snippet } from 'svelte';

	interface BaseToolProps {
		toolName: string;
		displayName?: string;
		reasoning: string;
		status: 'success' | 'error';
		statusText: string;
		timestamp: string;
		autoExpand?: boolean;
		errorMessage?: string;
		children: Snippet;
	}

	let {
		toolName,
		displayName,
		reasoning,
		status,
		statusText,
		timestamp,
		autoExpand = false,
		errorMessage,
		children
	}: BaseToolProps = $props();

	// Manage expand/collapse state
	let isExpanded = $state(autoExpand);

	// Format timestamp
	const formattedTime = new Date(timestamp).toLocaleTimeString([], {
		hour: '2-digit',
		minute: '2-digit'
	});
</script>

<div class="tool-call-item">
	<button
		class="tool-call-header"
		onclick={() => (isExpanded = !isExpanded)}
		aria-expanded={isExpanded}
	>
		<iconify-icon icon="ri:tools-line" class="tool-icon"></iconify-icon>
		<span class="tool-name">{displayName || toolName}</span>
		<span class="tool-action">"{reasoning}"</span>
		<span class="tool-status {status}">{statusText}</span>
		<iconify-icon
			icon={isExpanded ? 'ri:arrow-up-s-line' : 'ri:arrow-down-s-line'}
			class="expand-icon"
		></iconify-icon>
	</button>

	{#if isExpanded}
		<div class="tool-call-details">
			{#if errorMessage}
				<div class="detail-section">
					<div class="detail-label error">Error:</div>
					<div class="detail-value error">{errorMessage}</div>
				</div>
			{:else}
				{@render children()}
			{/if}

			<div class="detail-section">
				<div class="detail-timestamp">{formattedTime}</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.tool-call-item {
		background-color: var(--color-paper);
		border: 1px solid var(--color-stone-300);
		border-radius: 0.5rem;
		margin-bottom: 1.5rem;
		overflow: hidden;
	}

	.tool-call-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.625rem 0.75rem;
		cursor: pointer;
		background: transparent;
		border: none;
		transition: background-color 0.15s ease;
		text-align: left;
	}

	.tool-call-header:hover {
		background-color: var(--color-paper-dark);
	}

	.tool-icon {
		color: var(--color-stone-600);
		font-size: 1rem;
		flex-shrink: 0;
	}

	.tool-name {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-navy);
		flex-shrink: 0;
	}

	.tool-action {
		font-size: 0.875rem;
		color: var(--color-stone-600);
		flex-grow: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tool-status {
		font-size: 0.75rem;
		padding: 0.125rem 0.5rem;
		border-radius: 0.25rem;
		font-weight: 500;
		flex-shrink: 0;
	}

	.tool-status.success {
		background-color: transparent;
		color: var(--color-stone-600);
	}

	.tool-status.error {
		background-color: rgb(254 226 226);
		color: rgb(153 27 27);
	}

	.expand-icon {
		color: var(--color-stone-600);
		font-size: 1.25rem;
		flex-shrink: 0;
	}

	.tool-call-details {
		padding: 0.75rem;
		border-top: 1px solid var(--color-stone-300);
		background-color: var(--color-white);
	}

	.detail-section {
		margin-bottom: 0.75rem;
	}

	.detail-section:last-child {
		margin-bottom: 0;
	}

	.detail-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-stone-700);
		text-transform: uppercase;
		letter-spacing: 0.025em;
		margin-bottom: 0.375rem;
	}

	.detail-label.error {
		color: rgb(153 27 27);
	}

	.detail-value {
		font-size: 0.875rem;
		color: var(--color-stone-800);
	}

	.detail-value.error {
		color: rgb(153 27 27);
	}

	.detail-timestamp {
		font-size: 0.6875rem;
		color: var(--color-stone-600);
		text-align: right;
	}
</style>
