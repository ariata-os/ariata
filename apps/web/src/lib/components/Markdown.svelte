<script lang="ts">
	import { marked } from 'marked';
	import DOMPurify from 'isomorphic-dompurify';

	interface Props {
		content: string;
	}

	let { content }: Props = $props();

	// Configure marked options
	marked.setOptions({
		breaks: false, // Don't convert single \n to <br> (requires double space or double newline)
		gfm: true, // GitHub Flavored Markdown
	});

	// Parse and sanitize markdown
	let html = $derived.by(() => {
		const rawHtml = marked.parse(content) as string;
		return DOMPurify.sanitize(rawHtml, {
			ALLOWED_TAGS: [
				'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
				'p', 'br', 'strong', 'em', 'del', 's',
				'code', 'pre',
				'ul', 'ol', 'li',
				'blockquote',
				'a',
				'hr',
				'table', 'thead', 'tbody', 'tr', 'th', 'td',
				'img'
			],
			ALLOWED_ATTR: ['href', 'title', 'src', 'alt', 'class']
		});
	});
</script>

<div class="markdown">
	{@html html}
</div>

<style>
	/* Apply markdown styles to nested content */
	.markdown :global(h1),
	.markdown :global(h2),
	.markdown :global(h3),
	.markdown :global(h4),
	.markdown :global(h5),
	.markdown :global(h6) {
		margin-top: 1.5em;
		margin-bottom: 0.5em;
		font-weight: 600;
		line-height: 1.25;
	}

	.markdown :global(h1) {
		font-size: 2em;
	}
	.markdown :global(h2) {
		font-size: 1.5em;
	}
	.markdown :global(h3) {
		font-size: 1.25em;
	}

	.markdown :global(p) {
		margin: 0;
	}

	.markdown :global(p + p) {
		margin-top: 0.75em;
	}

	.markdown :global(code) {
		background-color: #f6f8fa;
		padding: 0.2em 0.4em;
		border-radius: 3px;
		font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, Consolas,
			'DejaVu Sans Mono', monospace;
		font-size: 0.9em;
	}

	.markdown :global(pre) {
		background-color: #f6f8fa;
		padding: 1em;
		border-radius: 6px;
		overflow-x: auto;
		margin: 1em 0;
	}

	.markdown :global(pre code) {
		background-color: transparent;
		padding: 0;
	}

	.markdown :global(blockquote) {
		margin: 1em 0;
		padding-left: 1em;
		border-left: 4px solid #d0d7de;
		color: #57606a;
	}

	.markdown :global(a) {
		color: #0969da;
		text-decoration: none;
	}

	.markdown :global(a:hover) {
		text-decoration: underline;
	}

	.markdown :global(em) {
		font-style: italic;
	}

	.markdown :global(hr) {
		border: none;
		border-top: 1px solid #d0d7de;
		margin: 1.5em 0;
	}

	.markdown :global(table) {
		border-collapse: collapse;
		width: 100%;
		margin: 1em 0;
	}

	.markdown :global(th),
	.markdown :global(td) {
		border: 1px solid #d0d7de;
		padding: 0.5em;
		text-align: left;
	}

	.markdown :global(th) {
		background-color: #f6f8fa;
		font-weight: 600;
	}

	.markdown :global(img) {
		max-width: 100%;
		height: auto;
	}
</style>
