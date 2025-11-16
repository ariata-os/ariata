<script lang="ts">
	import type { Task, Initiative, Aspiration } from '$lib/types/axiology';

	type TemporalPursuit =
		| { type: 'task'; data: Task }
		| { type: 'initiative'; data: Initiative }
		| { type: 'aspiration'; data: Aspiration };

	interface PursuitsData {
		pursuits: TemporalPursuit[];
		metadata: {
			totalCount: number;
			taskCount: number;
			initiativeCount: number;
			aspirationCount: number;
		};
	}

	interface PursuitsWidgetProps {
		data: PursuitsData;
	}

	let { data }: PursuitsWidgetProps = $props();

	// Helper functions
	function getPursuitIcon(type: string): string {
		switch (type) {
			case 'task':
				return '✓';
			case 'initiative':
				return '→';
			case 'aspiration':
				return '★';
			default:
				return '•';
		}
	}

	function getPursuitColorClass(type: string): string {
		switch (type) {
			case 'task':
				return 'border-blue-200 bg-blue-50';
			case 'initiative':
				return 'border-purple-200 bg-purple-50';
			case 'aspiration':
				return 'border-amber-200 bg-amber-50';
			default:
				return 'border-stone-200 bg-stone-50';
		}
	}

	function getPursuitTypeLabel(type: string): string {
		return type.charAt(0).toUpperCase() + type.slice(1);
	}

	function formatDate(dateString: string | null | undefined): string {
		if (!dateString) return '';
		const date = new Date(dateString);
		return date.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' });
	}
</script>

<div class="pursuits-widget">
	<div class="pursuits-header">
		<h3 class="pursuits-title">Pursuits</h3>
		<div class="pursuits-summary">
			<span class="summary-badge task-badge">{data.metadata.taskCount} tasks</span>
			<span class="summary-badge initiative-badge">{data.metadata.initiativeCount} initiatives</span>
			<span class="summary-badge aspiration-badge"
				>{data.metadata.aspirationCount} aspirations</span
			>
		</div>
	</div>

	<div class="pursuits-list">
		{#if data.pursuits.length === 0}
			<div class="empty-state">
				<p class="empty-message">No pursuits found</p>
			</div>
		{:else}
			{#each data.pursuits as pursuit}
				<div class="pursuit-card {getPursuitColorClass(pursuit.type)}">
					<div class="pursuit-header">
						<span class="pursuit-icon">{getPursuitIcon(pursuit.type)}</span>
						<div class="pursuit-main">
							<h4 class="pursuit-title">{pursuit.data.title}</h4>
							{#if pursuit.data.description}
								<p class="pursuit-description">{pursuit.data.description}</p>
							{/if}
						</div>
					</div>

					<div class="pursuit-meta">
						<span class="pursuit-type-badge">{getPursuitTypeLabel(pursuit.type)}</span>
						{#if pursuit.data.status}
							<span class="pursuit-status">· {pursuit.data.status}</span>
						{/if}
						{#if 'progress_percent' in pursuit.data && pursuit.data.progress_percent !== null && pursuit.data.progress_percent !== undefined}
							<span class="pursuit-progress">· {pursuit.data.progress_percent}%</span>
						{/if}
						{#if 'target_date' in pursuit.data && pursuit.data.target_date}
							<span class="pursuit-date">· {formatDate(pursuit.data.target_date)}</span>
						{/if}
					</div>

					{#if pursuit.data.tags && pursuit.data.tags.length > 0}
						<div class="pursuit-tags">
							{#each pursuit.data.tags as tag}
								<span class="tag">{tag}</span>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>

<style>
	.pursuits-widget {
		width: 100%;
		border-radius: 0.375rem;
		overflow: hidden;
		border: 1px solid var(--color-stone-300);
		background-color: var(--color-paper);
	}

	.pursuits-header {
		padding: 1rem;
		border-bottom: 1px solid var(--color-stone-300);
		background-color: rgb(250 250 250);
	}

	.pursuits-title {
		font-family: var(--font-serif);
		font-size: 1.125rem;
		font-weight: 600;
		color: rgb(23 23 23);
		margin: 0 0 0.75rem 0;
	}

	.pursuits-summary {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.summary-badge {
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		font-weight: 500;
	}

	.task-badge {
		background-color: rgb(219 234 254);
		color: rgb(30 64 175);
	}

	.initiative-badge {
		background-color: rgb(233 213 255);
		color: rgb(88 28 135);
	}

	.aspiration-badge {
		background-color: rgb(254 243 199);
		color: rgb(146 64 14);
	}

	.pursuits-list {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-height: 600px;
		overflow-y: auto;
	}

	.empty-state {
		padding: 2rem;
		text-align: center;
	}

	.empty-message {
		font-size: 0.875rem;
		color: rgb(115 115 115);
	}

	.pursuit-card {
		border: 1px solid;
		border-radius: 0.375rem;
		padding: 0.875rem;
		transition: box-shadow 0.2s;
	}

	.pursuit-card:hover {
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.08);
	}

	.pursuit-header {
		display: flex;
		gap: 0.75rem;
		align-items: start;
	}

	.pursuit-icon {
		font-size: 1.25rem;
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.pursuit-main {
		flex: 1;
		min-width: 0;
	}

	.pursuit-title {
		font-family: var(--font-serif);
		font-size: 0.9375rem;
		font-weight: 600;
		color: rgb(23 23 23);
		margin: 0 0 0.25rem 0;
	}

	.pursuit-description {
		font-size: 0.8125rem;
		color: rgb(82 82 82);
		margin: 0;
		line-height: 1.5;
	}

	.pursuit-meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.375rem;
		margin-top: 0.5rem;
		font-size: 0.75rem;
		color: rgb(115 115 115);
	}

	.pursuit-type-badge {
		text-transform: capitalize;
		font-weight: 500;
		color: rgb(64 64 64);
	}

	.pursuit-status {
		text-transform: capitalize;
	}

	.pursuit-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.5rem;
	}

	.tag {
		font-size: 0.6875rem;
		padding: 0.1875rem 0.4375rem;
		background-color: rgba(0, 0, 0, 0.05);
		border-radius: 0.25rem;
		color: rgb(64 64 64);
		font-weight: 500;
	}
</style>
