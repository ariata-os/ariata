<script lang="ts">
	import { getTimelineContext } from '../timeline';

	interface CountSignal {
		timestamp: Date;
		value: number;
		label?: string;
	}

	interface Props {
		signals?: CountSignal[];
		signalRange?: { min: number; max: number; unit: string };
	}

	let { signals = [], signalRange }: Props = $props();

	const { timeToPixel, pixelToTime, state } = getTimelineContext();

	// Process signals to calculate increments
	let increments = $derived.by(() => {
		if (signals.length < 2) return [];
		
		const sorted = [...signals].sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
		const results = [];
		
		for (let i = 1; i < sorted.length; i++) {
			const prev = sorted[i - 1];
			const curr = sorted[i];
			const increment = curr.value - prev.value;
			
			// Only show positive increments (counts should only go up)
			if (increment > 0) {
				results.push({
					startTime: prev.timestamp,
					endTime: curr.timestamp,
					value: increment,
					cumulativeValue: curr.value,
					x: timeToPixel(prev.timestamp),
					width: timeToPixel(curr.timestamp) - timeToPixel(prev.timestamp)
				});
			}
		}
		
		return results;
	});

	// Calculate total and rate
	let stats = $derived.by(() => {
		if (signals.length === 0) return { total: 0, rate: 0, unit: '' };
		
		const sorted = [...signals].sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
		const first = sorted[0];
		const last = sorted[sorted.length - 1];
		const total = last.value;
		
		// Calculate hourly rate
		const hours = (last.timestamp.getTime() - first.timestamp.getTime()) / (1000 * 60 * 60);
		const rate = hours > 0 ? Math.round(total / hours) : 0;
		
		return {
			total: Math.round(total),
			rate,
			unit: signalRange?.unit || ''
		};
	});

	// Find max increment for scaling
	let maxIncrement = $derived(
		increments.length > 0 ? Math.max(...increments.map(inc => inc.value)) : 1
	);
</script>

<div class="count-visualization">
	<!-- Stats summary -->
	<div class="stats-row">
		<div class="stat">
			<span class="stat-label">Total:</span>
			<span class="stat-value">{stats.total} {stats.unit}</span>
		</div>
		<div class="stat">
			<span class="stat-label">Rate:</span>
			<span class="stat-value">{stats.rate} {stats.unit}/hour</span>
		</div>
	</div>

	<!-- Bar chart -->
	<div class="chart-container">
		{#each increments as increment}
			{@const height = (increment.value / maxIncrement) * 40}
			{@const opacity = Math.min(0.8, 0.3 + (increment.value / maxIncrement) * 0.5)}
			
			<div
				class="increment-bar"
				style="
					left: {increment.x}px;
					width: {Math.max(2, increment.width - 1)}px;
					height: {height}px;
					opacity: {opacity};
				"
				title="+{increment.value} {stats.unit} (Total: {increment.cumulativeValue})"
			>
				{#if increment.width > 30}
					<span class="bar-label">+{increment.value}</span>
				{/if}
			</div>
		{/each}
		
		<!-- Zero line -->
		<div class="zero-line"></div>
	</div>
</div>

<style>
	.count-visualization {
		position: relative;
		width: 100%;
		padding: 0 24px;
	}

	.stats-row {
		display: flex;
		gap: 24px;
		margin-bottom: 12px;
		font-size: 12px;
		color: #666;
	}

	.stat {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.stat-label {
		color: #999;
	}

	.stat-value {
		font-weight: 600;
		color: #333;
		font-family: monospace;
	}

	.chart-container {
		position: relative;
		width: 100%;
		height: 60px;
		margin-top: 8px;
	}

	.increment-bar {
		position: absolute;
		bottom: 0;
		background: #60A5FA;
		border-radius: 2px 2px 0 0;
		transition: opacity 0.2s;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		padding-bottom: 2px;
	}

	.increment-bar:hover {
		opacity: 1 !important;
		background: #3B82F6;
	}

	.bar-label {
		font-size: 9px;
		color: white;
		font-weight: 600;
		font-family: monospace;
	}

	.zero-line {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 1px;
		background: #E5E7EB;
	}
</style>