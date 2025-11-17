<script lang="ts">
	import { browser } from '$app/environment';
	import { Streamdown } from 'svelte-streamdown';

	interface Props {
		content: string;
		isStreaming?: boolean;
	}

	let { content, isStreaming = false }: Props = $props();

	const customTheme = {
		code: {
			container: 'my-4 w-full overflow-hidden rounded-xl border border-stone-300 bg-paper-dark',
			header: 'flex items-center justify-between bg-stone-200/80 px-4 py-2 text-stone-600 text-xs font-mono',
			languageLabel: 'text-stone-700 font-medium',
			copyButton: 'px-2 py-1 rounded hover:bg-stone-300/50 transition-colors text-stone-600',
			copyIcon: 'w-4 h-4',
			pre: 'overflow-x-auto p-4 text-sm bg-navy',
			downloadButton: 'px-2 py-1 rounded hover:bg-stone-300/50 transition-colors text-stone-600',
			downloadIcon: 'w-4 h-4'
		}
	};
</script>

{#if browser}
	<div class="markdown">
		<Streamdown
			{content}
			class="streamdown-content"
			shikiTheme="dark-plus"
			parseIncompleteMarkdown={isStreaming}
			theme={customTheme}
			controls={{ table: false }}
			animation={{
				enabled: isStreaming,
				type: 'fade',
				duration: 300,
				tokenize: 'word',
				animateOnMount: false
			}}
		/>
	</div>
{:else}
	<!-- SSR fallback: plain text with basic styling -->
	<div class="markdown markdown-ssr">
		<pre class="whitespace-pre-wrap text-stone-800" style="line-height: 1.8;">{content}</pre>
	</div>
{/if}
