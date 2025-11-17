<script lang="ts">
	interface SearchResult {
		position: number;
		title: string;
		url: string;
		publishedDate?: string | null;
		author?: string | null;
		summary?: string | null;
		text?: string | null;
		score?: number;
	}

	interface SearchResultsData {
		query: string;
		resultsCount: number;
		searchType: string;
		results: SearchResult[];
		metadata?: {
			autopromptString?: string;
			requestId?: string;
		};
	}

	interface SearchResultsWidgetProps {
		data: SearchResultsData;
	}

	let { data }: SearchResultsWidgetProps = $props();

	// Helper functions
	function getDomain(url: string): string {
		try {
			const urlObj = new URL(url);
			return urlObj.hostname.replace('www.', '');
		} catch {
			return url;
		}
	}

	function formatDate(dateString: string | null | undefined): string {
		if (!dateString) return '';
		try {
			const date = new Date(dateString);
			return date.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' });
		} catch {
			return '';
		}
	}

	function getSearchTypeLabel(type: string): string {
		const labels: Record<string, string> = {
			auto: 'Hybrid',
			keyword: 'Keyword',
			neural: 'Semantic'
		};
		return labels[type] || type;
	}

	function truncateText(text: string | null | undefined, maxLength: number): string {
		if (!text) return '';
		if (text.length <= maxLength) return text;
		return text.substring(0, maxLength).trim() + '...';
	}
</script>

<div class="search-results-widget">
	<div class="search-header">
		<div class="search-query-section">
			<iconify-icon icon="ri:search-line" class="search-icon"></iconify-icon>
			<div class="search-query-info">
				<h3 class="search-query">"{data.query}"</h3>
				<div class="search-meta">
					<span class="search-count">{data.resultsCount} results</span>
					<span class="search-separator">·</span>
					<span class="search-type">{getSearchTypeLabel(data.searchType)} search</span>
				</div>
			</div>
		</div>
	</div>

	<div class="results-list">
		{#if data.results.length === 0}
			<div class="empty-state">
				<p class="empty-message">No results found</p>
			</div>
		{:else}
			{#each data.results as result}
				<div class="result-card">
					<div class="result-header">
						<span class="result-position">{result.position}</span>
						<div class="result-main">
							<a href={result.url} target="_blank" rel="noopener noreferrer" class="result-title">
								{result.title}
							</a>
							<div class="result-url-line">
								<span class="result-domain">{getDomain(result.url)}</span>
								{#if result.publishedDate}
									<span class="result-separator">·</span>
									<span class="result-date">{formatDate(result.publishedDate)}</span>
								{/if}
								{#if result.author}
									<span class="result-separator">·</span>
									<span class="result-author">{result.author}</span>
								{/if}
							</div>
							{#if result.summary}
								<p class="result-summary">{truncateText(result.summary, 200)}</p>
							{:else if result.text}
								<p class="result-summary">{truncateText(result.text, 200)}</p>
							{/if}
						</div>
					</div>
				</div>
			{/each}
		{/if}
	</div>

	{#if data.metadata?.autopromptString}
		<div class="search-footer">
			<div class="autoprompt">
				<iconify-icon icon="ri:lightbulb-line" class="autoprompt-icon"></iconify-icon>
				<span class="autoprompt-label">Refined query:</span>
				<span class="autoprompt-text">{data.metadata.autopromptString}</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.search-results-widget {
		background-color: var(--color-white);
		border-radius: 0.5rem;
		overflow: hidden;
	}

	.search-header {
		padding: 1rem;
		border-bottom: 1px solid var(--color-stone-200);
		background-color: var(--color-paper);
	}

	.search-query-section {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}

	.search-icon {
		color: var(--color-navy);
		font-size: 1.25rem;
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.search-query-info {
		flex: 1;
	}

	.search-query {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-navy);
		margin: 0 0 0.25rem 0;
	}

	.search-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8125rem;
		color: var(--color-stone-600);
	}

	.search-count {
		font-weight: 500;
	}

	.search-separator {
		color: var(--color-stone-400);
	}

	.search-type {
		color: var(--color-stone-600);
	}

	.results-list {
		padding: 0.5rem;
		max-height: 600px;
		overflow-y: auto;
	}

	.empty-state {
		padding: 2rem;
		text-align: center;
	}

	.empty-message {
		color: var(--color-stone-600);
		font-size: 0.875rem;
	}

	.result-card {
		padding: 1rem;
		margin-bottom: 0.5rem;
		border: 1px solid var(--color-stone-200);
		border-radius: 0.5rem;
		background-color: var(--color-white);
		transition: all 0.15s ease;
	}

	.result-card:last-child {
		margin-bottom: 0;
	}

	.result-card:hover {
		border-color: var(--color-navy);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
	}

	.result-header {
		display: flex;
		gap: 0.75rem;
	}

	.result-position {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		background-color: var(--color-paper);
		color: var(--color-stone-600);
		font-size: 0.75rem;
		font-weight: 600;
		border-radius: 0.25rem;
		flex-shrink: 0;
	}

	.result-main {
		flex: 1;
		min-width: 0;
	}

	.result-title {
		display: block;
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--color-navy);
		text-decoration: none;
		margin-bottom: 0.25rem;
		line-height: 1.4;
		transition: color 0.15s ease;
	}

	.result-title:hover {
		color: var(--color-primary);
		text-decoration: underline;
	}

	.result-url-line {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin-bottom: 0.5rem;
		font-size: 0.75rem;
		color: var(--color-stone-600);
	}

	.result-domain {
		font-family: 'IBM Plex Mono', monospace;
		color: var(--color-stone-700);
		font-weight: 500;
	}

	.result-separator {
		color: var(--color-stone-400);
	}

	.result-date,
	.result-author {
		color: var(--color-stone-600);
	}

	.result-summary {
		font-size: 0.875rem;
		color: var(--color-stone-700);
		line-height: 1.5;
		margin: 0;
	}

	.search-footer {
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-stone-200);
		background-color: var(--color-paper);
	}

	.autoprompt {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8125rem;
	}

	.autoprompt-icon {
		color: var(--color-stone-600);
		font-size: 1rem;
		flex-shrink: 0;
	}

	.autoprompt-label {
		color: var(--color-stone-600);
		font-weight: 500;
	}

	.autoprompt-text {
		color: var(--color-stone-700);
		font-style: italic;
	}
</style>
