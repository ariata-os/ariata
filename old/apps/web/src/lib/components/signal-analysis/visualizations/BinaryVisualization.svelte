<script lang="ts">
	import { getTimelineContext } from '../timeline';
	
	export let signals: Array<{
		timestamp: Date;
		value: number;
		label?: string;
	}> = [];
	export let height = 32;
	export let gapThreshold = 5 * 60 * 1000; // 5 minutes in ms
	
	const { state, timeToPixel } = getTimelineContext();
	
	// Cluster binary signals into continuous activity periods
	function clusterBinarySignals(signals: typeof signals) {
		if (!signals || signals.length === 0) return [];
		
		const sorted = [...signals].sort((a, b) => 
			a.timestamp.getTime() - b.timestamp.getTime()
		);
		
		const clusters = [];
		let currentCluster = {
			startTime: sorted[0].timestamp,
			endTime: sorted[0].timestamp,
			label: sorted[0].label || sorted[0].value,
			count: 1
		};
		
		for (let i = 1; i < sorted.length; i++) {
			const signal = sorted[i];
			const gap = signal.timestamp.getTime() - currentCluster.endTime.getTime();
			
			if (gap <= gapThreshold && signal.label === currentCluster.label) {
				// Extend current cluster
				currentCluster.endTime = signal.timestamp;
				currentCluster.count++;
			} else {
				// Start new cluster
				clusters.push(currentCluster);
				currentCluster = {
					startTime: signal.timestamp,
					endTime: signal.timestamp,
					label: signal.label || signal.value,
					count: 1
				};
			}
		}
		
		// Add the last cluster
		clusters.push(currentCluster);
		
		return clusters;
	}
	
	$: clustered = clusterBinarySignals(signals);
	
	// Find cluster at cursor position
	$: hoveredCluster = (() => {
		if (!$state.isHovering && !$state.isCursorLocked) return null;
		
		const targetTime = $state.isCursorLocked ? $state.lockedTime : $state.hoveredTime;
		if (!targetTime) return null;
		
		return clustered.find(cluster => 
			targetTime >= cluster.startTime && targetTime <= cluster.endTime
		);
	})();
</script>

<div class="binary-viz" style="height: {height}px">
	<svg class="viz-svg" style="height: {height}px">
		{#each clustered as cluster}
			{@const x = timeToPixel(cluster.startTime)}
			{@const width = Math.max(
				timeToPixel(cluster.endTime) - x,
				2
			)}
			{@const isHovered = hoveredCluster === cluster}
			<g class="activity-bar" class:hovered={isHovered}>
				<rect
					x={x}
					y={height / 2 - 8}
					{width}
					height="16"
					fill="rgba(0, 0, 0, 0.7)"
					rx="2"
					class="bar-fill"
				/>
				{#if cluster.label && width > 30}
					<text
						x={x + 4}
						y={height / 2 + 3}
						font-size="9"
						fill="white"
						class="bar-label"
					>
						{cluster.label.substring(0, Math.floor(width / 6))}
					</text>
				{/if}
			</g>
		{/each}
	</svg>
	
	<!-- Hover tooltip -->
	{#if hoveredCluster}
		{@const x = timeToPixel(hoveredCluster.startTime) + 
			(timeToPixel(hoveredCluster.endTime) - timeToPixel(hoveredCluster.startTime)) / 2}
		<div 
			class="hover-tooltip"
			style="left: {x}px; top: {height / 2}px"
		>
			<div class="tooltip-label">{hoveredCluster.label || 'Activity'}</div>
			<div class="tooltip-time">
				{hoveredCluster.startTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})} - {hoveredCluster.endTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})}
			</div>
			<div class="tooltip-duration">
				{Math.round((hoveredCluster.endTime.getTime() - hoveredCluster.startTime.getTime()) / 60000)} min
			</div>
		</div>
	{/if}
</div>

<style>
	.binary-viz {
		position: relative;
		width: 100%;
	}
	
	.viz-svg {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		overflow: visible;
	}
	
	.activity-bar {
		cursor: pointer;
		transition: opacity 0.2s ease;
	}
	
	.bar-fill {
		transition: all 0.2s ease;
	}
	
	.activity-bar.hovered .bar-fill {
		fill: rgba(0, 0, 0, 0.9);
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
	}
	
	.bar-label {
		pointer-events: none;
		font-family: monospace;
	}
	
	.hover-tooltip {
		position: absolute;
		transform: translate(-50%, -100%) translateY(-8px);
		background: rgba(30, 41, 59, 0.95);
		color: white;
		padding: 6px 10px;
		border-radius: 4px;
		font-size: 11px;
		white-space: nowrap;
		pointer-events: none;
		z-index: 200;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
	}
	
	.tooltip-label {
		font-weight: 600;
		margin-bottom: 2px;
	}
	
	.tooltip-time {
		font-size: 10px;
		opacity: 0.9;
		font-family: monospace;
	}
	
	.tooltip-duration {
		font-size: 10px;
		opacity: 0.7;
		margin-top: 2px;
	}
</style>