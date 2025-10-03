<script lang="ts">
	import { getTimelineContext } from "../timeline";

	export let signals: Array<{
		timestamp: Date;
		value: number;
		label?: string;
	}> = [];
	export let signalRange: {
		min: number;
		max: number;
		unit: string;
	};
	export let height = 48;
	export let color = "#40404040";

	const { state, timeToPixel, pixelToTime } = getTimelineContext();

	// Normalize Y value
	function normalizeY(value: number): number {
		if (signalRange.max === signalRange.min) return height / 2;
		return (
			height -
			4 -
			((value - signalRange.min) / (signalRange.max - signalRange.min)) *
				(height - 8)
		);
	}

	// Find closest signal to cursor
	$: closestSignal = (() => {
		if (!$state.isHovering && !$state.isCursorLocked) return null;
		if (signals.length === 0) return null;

		const targetTime = $state.isCursorLocked
			? $state.lockedTime
			: $state.hoveredTime;
		if (!targetTime) return null;

		let closest = signals[0];
		let minDiff = Math.abs(
			signals[0].timestamp.getTime() - targetTime.getTime(),
		);

		for (const signal of signals) {
			const diff = Math.abs(
				signal.timestamp.getTime() - targetTime.getTime(),
			);
			if (diff < minDiff) {
				minDiff = diff;
				closest = signal;
			}
		}

		// Only show if within 5 minutes
		if (minDiff > 5 * 60 * 1000) return null;
		return closest;
	})();

	// Generate SVG path
	$: svgPath = (() => {
		if (signals.length < 2) return "";

		const pathData = signals
			.map((signal, i) => {
				const x = timeToPixel(signal.timestamp);
				const y = normalizeY(signal.value);
				return i === 0 ? `M ${x} ${y}` : `L ${x} ${y}`;
			})
			.join(" ");

		return pathData;
	})();

	// Generate area path for gradient fill
	$: areaPath = (() => {
		if (signals.length < 2) return "";

		const firstX = timeToPixel(signals[0].timestamp);
		const lastX = timeToPixel(signals[signals.length - 1].timestamp);

		return svgPath + ` L ${lastX} ${height} ` + `L ${firstX} ${height} Z`;
	})();
</script>

<div class="continuous-viz" style="height: {height}px">
	<!-- Value labels -->
	<div class="value-labels">
		<span class="value-label top"
			>{signalRange.max.toFixed(1)} {signalRange.unit}</span
		>
		<span class="value-label bottom"
			>{signalRange.min.toFixed(1)} {signalRange.unit}</span
		>
	</div>

	<!-- SVG visualization -->
	<svg class="viz-svg" style="height: {height}px">
		<defs>
			<linearGradient
				id="areaGradient-{color}"
				x1="0%"
				y1="0%"
				x2="0%"
				y2="100%"
			>
				<stop offset="0%" style="stop-color:{color};stop-opacity:0.2" />
				<stop
					offset="100%"
					style="stop-color:{color};stop-opacity:0.05"
				/>
			</linearGradient>
		</defs>

		{#if signals.length > 1}
			<!-- Area fill -->
			<path d={areaPath} fill="url(#areaGradient-{color})" />

			<!-- Line -->
			<path
				d={svgPath}
				fill="none"
				stroke={color}
				stroke-width="1.5"
				opacity="0.8"
			/>
		{/if}

		<!-- Data points -->
		{#each signals as signal}
			{@const x = timeToPixel(signal.timestamp)}
			{@const y = normalizeY(signal.value)}
			<circle
				cx={x}
				cy={y}
				r="2"
				fill={color}
				class="data-point"
				class:highlighted={closestSignal === signal}
			/>
		{/each}

		<!-- Zero line if range includes zero -->
		{#if signalRange.min <= 0 && signalRange.max >= 0}
			<line
				x1="0"
				y1={normalizeY(0)}
				x2="100%"
				y2={normalizeY(0)}
				stroke="rgba(0,0,0,0.1)"
				stroke-dasharray="2,2"
			/>
		{/if}
	</svg>

	<!-- Hover tooltip -->
	{#if closestSignal}
		{@const x = timeToPixel(closestSignal.timestamp)}
		{@const y = normalizeY(closestSignal.value)}
		<div class="hover-tooltip" style="left: {x}px; top: {y}px">
			<div class="tooltip-value">
				{closestSignal.value.toFixed(2)}
				{signalRange.unit}
			</div>
			<div class="tooltip-time">
				{closestSignal.timestamp.toLocaleTimeString("en-US", {
					hour: "numeric",
					minute: "2-digit",
					hour12: true,
				})}
			</div>
		</div>
	{/if}
</div>

<style>
	.continuous-viz {
		position: relative;
		width: 100%;
	}

	.value-labels {
		position: absolute;
		right: 8px;
		top: 0;
		bottom: 0;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		pointer-events: none;
		z-index: 2;
	}

	.value-label {
		font-size: 9px;
		color: #6b7280;
		font-family: monospace;
		background: rgba(255, 255, 255, 0.8);
		padding: 0 2px;
		border-radius: 2px;
	}

	.viz-svg {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		overflow: visible;
	}

	.data-point {
		transition: r 0.2s ease;
	}

	.data-point.highlighted {
		r: 4;
		fill-opacity: 1;
	}

	.hover-tooltip {
		position: absolute;
		transform: translate(-50%, -100%) translateY(-8px);
		background: rgba(30, 41, 59, 0.95);
		color: white;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 11px;
		white-space: nowrap;
		pointer-events: none;
		z-index: 200;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
	}

	.tooltip-value {
		font-weight: 600;
		font-family: monospace;
	}

	.tooltip-time {
		font-size: 10px;
		opacity: 0.8;
	}
</style>
