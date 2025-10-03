<script lang="ts">
	import { getTimelineContext } from '../timeline';
	
	export let signals: Array<{
		timestamp: Date;
		value: number;
		label?: string;
		coordinates?: any;
	}> = [];
	export let height = 40;
	export let speedThreshold = 0.5; // m/s threshold for movement
	
	const { state, timeToPixel } = getTimelineContext();
	
	// Calculate distance between two GPS coordinates using Haversine formula
	function calculateDistance(lat1: number, lon1: number, lat2: number, lon2: number): number {
		const R = 6371000; // Earth's radius in meters
		const φ1 = (lat1 * Math.PI) / 180;
		const φ2 = (lat2 * Math.PI) / 180;
		const Δφ = ((lat2 - lat1) * Math.PI) / 180;
		const Δλ = ((lon2 - lon1) * Math.PI) / 180;
		
		const a = Math.sin(Δφ / 2) * Math.sin(Δφ / 2) +
			Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) * Math.sin(Δλ / 2);
		const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
		
		return R * c; // Distance in meters
	}
	
	// Analyze spatial movement patterns
	function analyzeSpatialMovement(signals: typeof signals) {
		if (!signals || signals.length === 0) return [];
		
		const sorted = [...signals].sort((a, b) => 
			a.timestamp.getTime() - b.timestamp.getTime()
		);
		
		const segments = [];
		let currentSegment = {
			startTime: sorted[0].timestamp,
			endTime: sorted[0].timestamp,
			isMoving: false,
			speed: 0,
			distance: 0,
			points: 1
		};
		
		for (let i = 1; i < sorted.length; i++) {
			const prevSignal = sorted[i - 1];
			const signal = sorted[i];
			
			// Calculate speed and distance from actual coordinates
			let speed = 0;
			let distance = 0;
			
			if (signal.coordinates && prevSignal.coordinates) {
				// Parse coordinates - they might be stored as JSON string or object
				let coords1, coords2;
				
				try {
					coords1 = typeof prevSignal.coordinates === 'string' 
						? JSON.parse(prevSignal.coordinates) 
						: prevSignal.coordinates;
					coords2 = typeof signal.coordinates === 'string' 
						? JSON.parse(signal.coordinates) 
						: signal.coordinates;
				} catch (e) {
					console.error('Failed to parse coordinates:', e);
					continue;
				}
				
				// Extract lat/lon - handle different possible formats
				const lat1 = coords1.latitude || coords1.lat || coords1[0];
				const lon1 = coords1.longitude || coords1.lon || coords1.lng || coords1[1];
				const lat2 = coords2.latitude || coords2.lat || coords2[0];
				const lon2 = coords2.longitude || coords2.lon || coords2.lng || coords2[1];
				
				if (lat1 && lon1 && lat2 && lon2) {
					// Calculate actual distance
					distance = calculateDistance(lat1, lon1, lat2, lon2);
					
					// Calculate speed in m/s
					const timeDiff = (signal.timestamp.getTime() - prevSignal.timestamp.getTime()) / 1000; // seconds
					if (timeDiff > 0) {
						speed = distance / timeDiff;
					}
				}
			}
			
			const isMoving = speed > speedThreshold;
			
			// Check if we should continue current segment or start new one
			if (isMoving === currentSegment.isMoving) {
				// Continue segment
				currentSegment.endTime = signal.timestamp;
				currentSegment.speed = (currentSegment.speed * currentSegment.points + speed) / (currentSegment.points + 1);
				currentSegment.distance += distance;
				currentSegment.points++;
			} else {
				// Start new segment
				if (currentSegment.endTime.getTime() - currentSegment.startTime.getTime() === 0) {
					currentSegment.endTime = new Date(currentSegment.endTime.getTime() + 60 * 1000);
				}
				segments.push(currentSegment);
				currentSegment = {
					startTime: signal.timestamp,
					endTime: signal.timestamp,
					isMoving: isMoving,
					speed: speed,
					distance: distance,
					points: 1
				};
			}
		}
		
		// Add the last segment
		if (currentSegment.endTime.getTime() - currentSegment.startTime.getTime() === 0) {
			currentSegment.endTime = new Date(currentSegment.endTime.getTime() + 60 * 1000);
		}
		segments.push(currentSegment);
		
		return segments;
	}
	
	$: movementData = analyzeSpatialMovement(signals);
	$: totalDistance = movementData.reduce((acc, s) => acc + (s.distance || 0), 0);
	
	// Find segment at cursor position
	$: hoveredSegment = (() => {
		if (!$state.isHovering && !$state.isCursorLocked) return null;
		
		const targetTime = $state.isCursorLocked ? $state.lockedTime : $state.hoveredTime;
		if (!targetTime) return null;
		
		return movementData.find(segment => 
			targetTime >= segment.startTime && targetTime <= segment.endTime
		);
	})();
</script>

<div class="spatial-viz" style="height: {height}px">
	<svg class="viz-svg" style="height: {height}px">
		<defs>
			<linearGradient id="movementGradient" x1="0%" y1="0%" x2="0%" y2="100%">
				<stop offset="0%" style="stop-color:rgba(34,197,94,0.3);stop-opacity:1" />
				<stop offset="100%" style="stop-color:rgba(34,197,94,0.05);stop-opacity:1" />
			</linearGradient>
		</defs>
		
		<!-- Movement intensity bars -->
		{#each movementData as segment}
			{@const x = timeToPixel(segment.startTime)}
			{@const width = Math.max(
				timeToPixel(segment.endTime) - x,
				1
			)}
			{@const intensity = segment.isMoving ? Math.min(segment.speed / 10, 1) : 0}
			{@const barHeight = 8 + intensity * (height - 16)}
			{@const opacity = segment.isMoving ? 0.3 + intensity * 0.7 : 0.2}
			{@const isHovered = hoveredSegment === segment}
			
			<rect
				x={x}
				y={height - barHeight - 4}
				{width}
				height={barHeight}
				fill={segment.isMoving 
					? `rgba(34,197,94,${opacity})` 
					: 'rgba(156,163,175,0.3)'}
				class="movement-bar"
				class:hovered={isHovered}
			/>
		{/each}
		
		<!-- Distance traveled indicator -->
		<text
			x="4"
			y="12"
			font-size="9"
			fill="rgba(0,0,0,0.5)"
			class="distance-label"
		>
			Total: {(totalDistance / 1000).toFixed(1)}km
		</text>
	</svg>
	
	<!-- Hover tooltip -->
	{#if hoveredSegment}
		{@const x = timeToPixel(hoveredSegment.startTime) + 
			(timeToPixel(hoveredSegment.endTime) - timeToPixel(hoveredSegment.startTime)) / 2}
		<div 
			class="hover-tooltip"
			style="left: {x}px; top: {height / 2}px"
		>
			<div class="tooltip-status">
				{hoveredSegment.isMoving ? 'Moving' : 'Stationary'}
			</div>
			{#if hoveredSegment.isMoving}
				<div class="tooltip-speed">
					{hoveredSegment.speed.toFixed(1)} m/s ({(hoveredSegment.speed * 2.237).toFixed(1)} mph)
				</div>
				<div class="tooltip-distance">
					{(hoveredSegment.distance / 1000).toFixed(2)} km traveled
				</div>
			{/if}
			<div class="tooltip-time">
				{hoveredSegment.startTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})} - {hoveredSegment.endTime.toLocaleTimeString('en-US', { 
					hour: 'numeric', 
					minute: '2-digit',
					hour12: true 
				})}
			</div>
		</div>
	{/if}
</div>

<style>
	.spatial-viz {
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
	
	.movement-bar {
		transition: all 0.2s ease;
	}
	
	.movement-bar.hovered {
		filter: brightness(1.2) drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
	}
	
	.distance-label {
		font-family: monospace;
		pointer-events: none;
	}
	
	.hover-tooltip {
		position: absolute;
		transform: translate(-50%, -50%);
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
	
	.tooltip-status {
		font-weight: 600;
		margin-bottom: 2px;
	}
	
	.tooltip-speed,
	.tooltip-distance {
		font-size: 10px;
		opacity: 0.9;
		font-family: monospace;
	}
	
	.tooltip-time {
		font-size: 10px;
		opacity: 0.7;
		margin-top: 2px;
		font-family: monospace;
	}
</style>