<script lang="ts">
	import type { PageData } from './$types';

	export let data: PageData;

	// Format numbers with commas
	function formatNumber(num: number): string {
		return num.toLocaleString();
	}

	// Get color based on percentage
	function getProgressColor(percentage: number): string {
		if (percentage < 50) return '#10b981'; // green
		if (percentage < 75) return '#f59e0b'; // yellow
		if (percentage < 90) return '#f97316'; // orange
		return '#ef4444'; // red
	}
</script>

<svelte:head>
	<title>API Usage - Ariata</title>
</svelte:head>

<div class="usage-page">
	<div class="header">
		<h1>API Usage Dashboard</h1>
		<p class="subtitle">Monitor your AI API usage and costs</p>
	</div>

	<div class="stats-grid">
		<!-- Daily Requests Card -->
		<div class="stat-card">
			<div class="card-header">
				<h3>Daily Requests</h3>
				<span class="help-text">Resets at midnight UTC</span>
			</div>
			<div class="stat-value">
				<span class="big-number">{formatNumber(data.usage.daily.requests)}</span>
				<span class="limit">/ {formatNumber(data.usage.daily.requestsLimit)}</span>
			</div>
			<div class="progress-bar">
				<div
					class="progress-fill"
					style="width: {data.usage.daily.requestsPercentage}%; background-color: {getProgressColor(
						data.usage.daily.requestsPercentage
					)}"
				/>
			</div>
			<div class="percentage">{data.usage.daily.requestsPercentage}% used</div>
		</div>

		<!-- Daily Tokens Card -->
		<div class="stat-card">
			<div class="card-header">
				<h3>Daily Tokens</h3>
				<span class="help-text">Input + Output tokens</span>
			</div>
			<div class="stat-value">
				<span class="big-number">{formatNumber(data.usage.daily.tokens)}</span>
				<span class="limit">/ {formatNumber(data.usage.daily.tokensLimit)}</span>
			</div>
			<div class="progress-bar">
				<div
					class="progress-fill"
					style="width: {data.usage.daily.tokensPercentage}%; background-color: {getProgressColor(
						data.usage.daily.tokensPercentage
					)}"
				/>
			</div>
			<div class="percentage">{data.usage.daily.tokensPercentage}% used</div>
		</div>

		<!-- Daily Cost Card -->
		<div class="stat-card highlight">
			<div class="card-header">
				<h3>Estimated Cost Today</h3>
				<span class="help-text">Based on current pricing</span>
			</div>
			<div class="stat-value">
				<span class="big-number cost">${data.usage.daily.cost.toFixed(2)}</span>
			</div>
			<div class="cost-breakdown">
				<span>Max daily cost: ~$1.50</span>
			</div>
		</div>
	</div>

	<div class="info-section">
		<h2>Rate Limits</h2>
		<div class="info-grid">
			<div class="info-item">
				<div class="info-label">Chat Requests (Daily)</div>
				<div class="info-value">{formatNumber(data.usage.limits.chatRequestsPerDay)}</div>
			</div>
			<div class="info-item">
				<div class="info-label">Tokens (Daily)</div>
				<div class="info-value">{formatNumber(data.usage.limits.chatTokensPerDay)}</div>
			</div>
			<div class="info-item">
				<div class="info-label">Background Jobs (Daily)</div>
				<div class="info-value">{formatNumber(data.usage.limits.backgroundJobsPerDay)}</div>
			</div>
		</div>
	</div>

	<div class="info-section">
		<h2>About Rate Limiting</h2>
		<div class="about-text">
			<p>
				These limits protect against excessive API costs and ensure fair usage. Limits are enforced
				per-instance in your deployment.
			</p>
			<ul>
				<li><strong>Daily limits</strong> reset at midnight (UTC)</li>
				<li><strong>Token limits</strong> include both input and output tokens</li>
				<li>
					<strong>Cost estimates</strong> are based on current Anthropic Claude and OpenAI pricing
				</li>
			</ul>
		</div>
	</div>
</div>

<style>
	.usage-page {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
	}

	.header {
		margin-bottom: 2rem;
	}

	.header h1 {
		font-size: 2rem;
		font-weight: 700;
		margin-bottom: 0.5rem;
		color: var(--text-primary, #1f2937);
	}

	.subtitle {
		color: var(--text-secondary, #6b7280);
		font-size: 1rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1.5rem;
		margin-bottom: 3rem;
	}

	.stat-card {
		background: var(--card-bg, #ffffff);
		border: 1px solid var(--border, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
		transition: box-shadow 0.2s;
	}

	.stat-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
	}

	.stat-card.highlight {
		border-color: var(--primary, #3b82f6);
		background: linear-gradient(135deg, #ffffff 0%, #eff6ff 100%);
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.card-header h3 {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-secondary, #6b7280);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0;
	}

	.help-text {
		font-size: 0.75rem;
		color: var(--text-tertiary, #9ca3af);
	}

	.stat-value {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.big-number {
		font-size: 2.5rem;
		font-weight: 700;
		color: var(--text-primary, #1f2937);
		line-height: 1;
	}

	.big-number.cost {
		color: var(--primary, #3b82f6);
	}

	.limit {
		font-size: 1.25rem;
		color: var(--text-tertiary, #9ca3af);
	}

	.progress-bar {
		width: 100%;
		height: 8px;
		background-color: var(--progress-bg, #e5e7eb);
		border-radius: 4px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		transition: width 0.3s ease;
		border-radius: 4px;
	}

	.percentage {
		font-size: 0.875rem;
		color: var(--text-secondary, #6b7280);
	}

	.cost-breakdown {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border, #e5e7eb);
		font-size: 0.875rem;
		color: var(--text-secondary, #6b7280);
	}

	.info-section {
		margin-bottom: 2rem;
	}

	.info-section h2 {
		font-size: 1.5rem;
		font-weight: 600;
		margin-bottom: 1rem;
		color: var(--text-primary, #1f2937);
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		background: var(--card-bg, #ffffff);
		border: 1px solid var(--border, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.info-label {
		font-size: 0.875rem;
		color: var(--text-secondary, #6b7280);
	}

	.info-value {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text-primary, #1f2937);
	}

	.about-text {
		background: var(--card-bg, #ffffff);
		border: 1px solid var(--border, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
		line-height: 1.6;
	}

	.about-text p {
		margin-bottom: 1rem;
		color: var(--text-secondary, #6b7280);
	}

	.about-text ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.about-text li {
		padding: 0.5rem 0;
		color: var(--text-secondary, #6b7280);
	}

	.about-text strong {
		color: var(--text-primary, #1f2937);
	}

	@media (max-width: 768px) {
		.usage-page {
			padding: 1rem;
		}

		.header h1 {
			font-size: 1.5rem;
		}

		.stats-grid {
			grid-template-columns: 1fr;
		}

		.big-number {
			font-size: 2rem;
		}
	}
</style>
