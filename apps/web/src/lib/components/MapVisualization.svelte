<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Map as LeafletMap, Marker, Polyline } from 'leaflet';

	interface LocationPoint {
		latitude: number;
		longitude: number;
		timestamp: string;
		accuracy_meters?: number;
		speed_meters_per_second?: number;
	}

	interface LocationVisit {
		latitude: number;
		longitude: number;
		start_time: string;
		end_time: string;
		canonical_name?: string;
		category?: string;
		duration_minutes: number;
	}

	interface MapBounds {
		minLat: number;
		maxLat: number;
		minLon: number;
		maxLon: number;
	}

	interface MapData {
		points: LocationPoint[];
		visits: LocationVisit[];
		bounds: MapBounds | null;
		metadata: {
			startTime: string;
			endTime: string;
			pointCount: number;
			visitCount: number;
		};
	}

	interface MapVisualizationProps {
		data: MapData;
	}

	let { data }: MapVisualizationProps = $props();
	let mapContainer: HTMLDivElement;
	let map: LeafletMap | null = null;
	let markers: Marker[] = [];
	let polyline: Polyline | null = null;

	// Format date/time for display
	function formatDateTime(isoString: string): string {
		const date = new Date(isoString);
		return date.toLocaleString([], {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	// Format duration in a human-readable way
	function formatDuration(minutes: number): string {
		if (minutes < 60) {
			return `${Math.round(minutes)}m`;
		}
		const hours = Math.floor(minutes / 60);
		const mins = Math.round(minutes % 60);
		return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
	}

	onMount(async () => {
		// Dynamically import Leaflet to avoid SSR issues
		const L = await import('leaflet');

		// Import Leaflet CSS
		if (typeof window !== 'undefined') {
			const link = document.createElement('link');
			link.rel = 'stylesheet';
			link.href = 'https://unpkg.com/leaflet@1.9.4/dist/leaflet.css';
			document.head.appendChild(link);
		}

		// Initialize map
		map = L.map(mapContainer, {
			zoomControl: true,
			attributionControl: true
		});

		// Add OpenStreetMap tiles
		L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
			maxZoom: 19
		}).addTo(map);

		// Create custom icons for different marker types
		const visitIcon = L.divIcon({
			className: 'custom-marker-visit',
			html: '<div class="marker-pin"></div>',
			iconSize: [24, 24],
			iconAnchor: [12, 24],
			popupAnchor: [0, -24]
		});

		const pointIcon = L.circleMarker([0, 0], {
			radius: 3,
			fillColor: '#3b82f6',
			color: '#1e40af',
			weight: 1,
			opacity: 0.8,
			fillOpacity: 0.6
		});

		// Add location points as a polyline (path)
		if (data.points.length > 0) {
			const pathCoords: [number, number][] = data.points.map((p) => [p.latitude, p.longitude]);
			polyline = L.polyline(pathCoords, {
				color: '#3b82f6',
				weight: 2,
				opacity: 0.6,
				smoothFactor: 1
			}).addTo(map);

			// Add small circles at each point (optional, can be removed for cleaner look)
			data.points.forEach((point) => {
				const circle = L.circleMarker([point.latitude, point.longitude], {
					radius: 2,
					fillColor: '#3b82f6',
					color: '#1e40af',
					weight: 1,
					opacity: 0.5,
					fillOpacity: 0.4
				});

				// Add tooltip with timestamp
				circle.bindTooltip(formatDateTime(point.timestamp), {
					permanent: false,
					direction: 'top'
				});

				circle.addTo(map);
			});
		}

		// Add visit markers
		if (data.visits.length > 0) {
			data.visits.forEach((visit) => {
				const marker = L.marker([visit.latitude, visit.longitude], {
					icon: visitIcon
				});

				// Create popup content
				const popupContent = `
					<div class="visit-popup">
						<div class="visit-name">${visit.canonical_name || 'Unknown Place'}</div>
						${visit.category ? `<div class="visit-category">${visit.category}</div>` : ''}
						<div class="visit-time">
							<strong>Arrived:</strong> ${formatDateTime(visit.start_time)}<br/>
							<strong>Left:</strong> ${formatDateTime(visit.end_time)}<br/>
							<strong>Duration:</strong> ${formatDuration(visit.duration_minutes)}
						</div>
					</div>
				`;

				marker.bindPopup(popupContent);
				marker.addTo(map);
				markers.push(marker);
			});
		}

		// Fit bounds to show all data
		if (data.bounds) {
			const bounds = L.latLngBounds(
				[data.bounds.minLat, data.bounds.minLon],
				[data.bounds.maxLat, data.bounds.maxLon]
			);
			map.fitBounds(bounds, { padding: [30, 30] });
		} else if (data.points.length > 0) {
			map.setView([data.points[0].latitude, data.points[0].longitude], 13);
		} else if (data.visits.length > 0) {
			map.setView([data.visits[0].latitude, data.visits[0].longitude], 13);
		}
	});

	onDestroy(() => {
		if (map) {
			map.remove();
		}
	});
</script>

<div class="map-visualization">
	<div bind:this={mapContainer} class="map-container"></div>
	<div class="map-legend">
		<div class="legend-item">
			<div class="legend-icon path-icon"></div>
			<span class="legend-label">{data.metadata.pointCount} location points</span>
		</div>
		<div class="legend-item">
			<div class="legend-icon marker-icon"></div>
			<span class="legend-label">{data.metadata.visitCount} place visits</span>
		</div>
		<div class="legend-time">
			{formatDateTime(data.metadata.startTime)} → {formatDateTime(data.metadata.endTime)}
		</div>
	</div>
</div>

<style>
	.map-visualization {
		width: 100%;
		border-radius: 0.375rem;
		overflow: hidden;
		border: 1px solid rgb(229 229 229);
	}

	.map-container {
		width: 100%;
		height: 400px;
		background-color: rgb(243 244 246);
	}

	.map-legend {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem;
		background-color: rgb(250 250 250);
		border-top: 1px solid rgb(229 229 229);
		font-size: 0.8125rem;
		color: rgb(82 82 82);
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.legend-icon {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}

	.path-icon {
		background: linear-gradient(90deg, #3b82f6 0%, #3b82f6 100%);
		border-radius: 2px;
		opacity: 0.6;
	}

	.marker-icon {
		width: 12px;
		height: 12px;
		background-color: #ef4444;
		border-radius: 50% 50% 50% 0;
		transform: rotate(-45deg);
		border: 2px solid #7f1d1d;
	}

	.legend-label {
		font-size: 0.8125rem;
		color: rgb(64 64 64);
	}

	.legend-time {
		margin-left: auto;
		font-size: 0.75rem;
		color: rgb(115 115 115);
	}

	/* Custom marker styles (injected globally) */
	:global(.custom-marker-visit) {
		background: transparent;
		border: none;
	}

	:global(.marker-pin) {
		width: 20px;
		height: 20px;
		background-color: #ef4444;
		border-radius: 50% 50% 50% 0;
		transform: rotate(-45deg);
		border: 3px solid #7f1d1d;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
	}

	/* Popup styles */
	:global(.visit-popup) {
		font-family: system-ui, -apple-system, sans-serif;
		min-width: 200px;
	}

	:global(.visit-name) {
		font-size: 0.9375rem;
		font-weight: 600;
		color: rgb(23 23 23);
		margin-bottom: 0.25rem;
	}

	:global(.visit-category) {
		font-size: 0.8125rem;
		color: rgb(82 82 82);
		margin-bottom: 0.5rem;
		text-transform: capitalize;
	}

	:global(.visit-time) {
		font-size: 0.8125rem;
		color: rgb(64 64 64);
		line-height: 1.5;
	}

	:global(.visit-time strong) {
		font-weight: 600;
		color: rgb(38 38 38);
	}
</style>
