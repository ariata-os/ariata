<script lang="ts">
	import "iconify-icon";
	
	interface StreamHealth {
		id: string;
		streamName: string;
		displayName: string;
		ingestionType: 'push' | 'pull';
		status: string;
		health: {
			status: 'healthy' | 'warning' | 'error' | 'stale';
			message: string;
			lastIngestionAt: string | null;
			minutesSinceLastIngestion: number;
		};
		metrics: {
			totalRuns: number;
			recentRuns: number;
			failedRuns: number;
			errorRate: string;
			totalRecords: number;
			avgProcessingTime: number | null;
			recordsPerHour: number;
		};
	}
	
	export let stream: StreamHealth;
	
	function getHealthIcon(status: string): string {
		switch (status) {
			case 'healthy':
				return 'ri:check-circle-line';
			case 'warning':
				return 'ri:alert-line';
			case 'error':
				return 'ri:error-warning-line';
			default:
				return 'ri:time-line';
		}
	}
	
	function getHealthColor(status: string) {
		switch (status) {
			case 'healthy':
				return 'text-green-600 bg-green-50';
			case 'warning':
				return 'text-yellow-600 bg-yellow-50';
			case 'error':
				return 'text-red-600 bg-red-50';
			default:
				return 'text-gray-600 bg-gray-50';
		}
	}
	
	function formatTime(minutes: number): string {
		if (minutes < 60) return `${minutes}m`;
		if (minutes < 1440) return `${Math.round(minutes / 60)}h`;
		return `${Math.round(minutes / 1440)}d`;
	}
</script>

<div class="bg-white rounded-lg border border-neutral-200 p-4">
	<div class="flex items-start justify-between mb-3">
		<div class="flex-1">
			<h4 class="font-medium text-neutral-900">{stream.displayName}</h4>
			<p class="text-sm text-neutral-500">{stream.streamName}</p>
		</div>
		<div class="flex items-center gap-2">
			<span class="text-xs px-2 py-1 rounded-full bg-neutral-100">
				{stream.ingestionType}
			</span>
		</div>
	</div>
	
	<!-- Health Status -->
	<div class="flex items-center gap-2 mb-3">
		<div class="flex items-center gap-1.5 {getHealthColor(stream.health.status)} px-2 py-1 rounded-md">
			<iconify-icon icon={getHealthIcon(stream.health.status)} width="16" height="16"></iconify-icon>
			<span class="text-sm font-medium capitalize">{stream.health.status}</span>
		</div>
		<span class="text-sm text-neutral-600">{stream.health.message}</span>
	</div>
	
	<!-- Metrics Grid -->
	<div class="grid grid-cols-3 gap-2 text-sm">
		<div>
			<p class="text-neutral-500">Last Data</p>
			<p class="font-medium">
				{stream.health.lastIngestionAt ? formatTime(stream.health.minutesSinceLastIngestion) + ' ago' : 'Never'}
			</p>
		</div>
		<div>
			<p class="text-neutral-500">Records/hr</p>
			<p class="font-medium">{stream.metrics.recordsPerHour.toLocaleString()}</p>
		</div>
		<div>
			<p class="text-neutral-500">Error Rate</p>
			<p class="font-medium {parseFloat(stream.metrics.errorRate) > 10 ? 'text-red-600' : ''}">
				{stream.metrics.errorRate}%
			</p>
		</div>
	</div>
	
	{#if stream.metrics.recentRuns > 0}
		<div class="mt-3 pt-3 border-t border-neutral-100">
			<div class="flex items-center justify-between text-xs text-neutral-500">
				<span>{stream.metrics.recentRuns} runs in last hour</span>
				{#if stream.metrics.avgProcessingTime}
					<span>Avg: {stream.metrics.avgProcessingTime}s</span>
				{/if}
			</div>
		</div>
	{/if}
</div>