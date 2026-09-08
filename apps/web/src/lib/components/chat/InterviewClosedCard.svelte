<!--
	InterviewClosedCard.svelte

	The close of the narrative interview, in place of the composer: two doors,
	the document ("In your own words", the person's first-person account, on
	its own page) and the chapters (their partition of the life, one page
	each). Before this card existed the interview simply went on: the tool
	ran, a page opened beside, and nothing in the room said it was over, so
	people kept typing into a chat whose drafter had already run once and
	would never run again.

	It rendered a second time inline for the tool result, so the same tiles
	showed twice; now only the standing card carries the doors. The copy reads
	the OUTCOME: when the chapters were not written, the lede says one place,
	not two, and the Chapters door is a note rather than a link to an empty
	page.
-->

<script lang="ts">
	import Icon from '$lib/components/Icon.svelte';
	import { windowShellStore } from '$lib/stores/window-shell.svelte';

	interface Props {
		/** The document's page id (write_it_up's `document_page_id`). */
		pageId: string | null;
		/** Chapters written by THIS call; 0 when they already stood. */
		chaptersWritten?: number;
		/** The document already stood when the tool ran (one writer, once). */
		alreadyExisted?: boolean;
		/** The chapters extraction failed; the document is safe. */
		chaptersError?: string | null;
		/** In place of the composer (standing), or inline in the transcript. */
		standing?: boolean;
	}

	let {
		pageId,
		chaptersWritten = 0,
		alreadyExisted = false,
		chaptersError = null,
		standing = false,
	}: Props = $props();

	function openDocument() {
		if (pageId) windowShellStore.openRouteBeside(`/page/${pageId}`, 'In your own words');
		else windowShellStore.openRouteBeside('/wiki/identity', 'In your own words');
	}

	function openChapters() {
		windowShellStore.openRouteBeside('/wiki/chapters', 'Chapters');
	}

	const chaptersNote = $derived(
		chaptersError
			? 'Not written this time. The document is safe.'
			: chaptersWritten > 0
				? `${chaptersWritten} ${chaptersWritten === 1 ? 'chapter' : 'chapters'}, a page each.`
				: 'Your partition of the life, a page each.'
	);

	const lede = $derived(
		chaptersError
			? 'What you said is arranged on one page. It is yours: the machine never rewrites it, and anything to add or correct is done on the page.'
			: 'What you said is arranged in two places. Both are yours: the machine never rewrites them, and anything to add or correct is done on the page.'
	);
</script>

<div class="closed" class:standing>
	<p class="eyebrow">
		{standing
			? alreadyExisted
				? 'This interview is closed. The document already stood.'
				: 'This interview is closed'
			: alreadyExisted
				? 'Already written'
				: 'Written up'}
	</p>
	{#if standing}
		<p class="lede">{lede}</p>
	{/if}
	<div class="doors">
		<button type="button" class="door" onclick={openDocument}>
			<span class="door-title">In your own words</span>
			<span class="door-note">Your account, in the first person.</span>
			<Icon icon="ri:arrow-right-up-line" width="14" class="door-arrow" />
		</button>
		{#if chaptersError}
			<div class="door door-inert" aria-disabled="true">
				<span class="door-title">Chapters</span>
				<span class="door-note">{chaptersNote}</span>
			</div>
		{:else}
			<button type="button" class="door" onclick={openChapters}>
				<span class="door-title">Chapters</span>
				<span class="door-note">{chaptersNote}</span>
				<Icon icon="ri:arrow-right-up-line" width="14" class="door-arrow" />
			</button>
		{/if}
	</div>
</div>

<style>
	.closed {
		margin: 0.75rem 0;
		padding: 1rem 1.125rem 1.125rem;
		border: 1px solid var(--color-border);
		border-radius: 0.625rem;
		background: var(--color-surface);
		max-width: 34rem;
	}

	.closed.standing {
		margin: 0 auto;
		max-width: 48rem;
		background: var(--color-background);
	}

	.eyebrow {
		margin: 0 0 0.375rem;
		font-size: 0.6875rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--color-foreground-muted);
	}

	.lede {
		margin: 0 0 0.875rem;
		font-family: var(--font-serif, Georgia, serif);
		font-size: 1rem;
		line-height: 1.5;
		color: var(--color-foreground);
		max-width: 36rem;
	}

	.doors {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.625rem;
	}

	@media (max-width: 560px) {
		.doors {
			grid-template-columns: 1fr;
		}
	}

	.door {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.25rem;
		padding: 0.75rem 2rem 0.75rem 0.875rem;
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		background: var(--color-background);
		text-align: left;
		cursor: pointer;
		color: var(--color-foreground);
		transition: border-color 120ms ease;
	}

	.door-inert {
		cursor: default;
		color: var(--color-foreground-muted);
	}

	.door-inert:hover {
		border-color: var(--color-border);
		background: var(--color-background);
	}

	.door:hover {
		border-color: var(--color-foreground-muted);
	}

	.door-title {
		font-family: var(--font-serif, Georgia, serif);
		font-size: 1.0625rem;
		line-height: 1.25;
	}

	.door-note {
		font-size: 0.8125rem;
		line-height: 1.4;
		color: var(--color-foreground-muted);
	}

	.door :global(.door-arrow) {
		position: absolute;
		top: 0.75rem;
		right: 0.75rem;
		color: var(--color-foreground-muted);
	}
</style>
