<script lang="ts">
	import MapVisualization from './MapVisualization.svelte';
	import PursuitsWidget from './PursuitsWidget.svelte';

	interface ToolCallProps {
		tool_name: string;
		arguments: Record<string, unknown>;
		result?: unknown;
		timestamp: string;
	}

	let { tool_name, arguments: args, result: rawResult, timestamp }: ToolCallProps = $props();

	// Debug: Log what we received
	console.log('[ToolCall] Received props:', { tool_name, args, result: rawResult, timestamp });

	// Parse result if it's a JSON string
	let parsedResult: unknown = rawResult;
	if (typeof rawResult === 'string') {
		try {
			parsedResult = JSON.parse(rawResult);
			console.log('[ToolCall] Parsed JSON result:', parsedResult);
		} catch (e) {
			console.error('[ToolCall] Failed to parse result JSON:', e);
			// If parsing fails, treat it as an error result
			parsedResult = { error: rawResult };
		}
	}

	// Type guard and parse result
	let result: {
		success?: boolean;
		type?: string;
		data?: unknown;
		rowCount?: number;
		rows?: unknown[];
		columns?: string[];
		error?: string;
		rawOutput?: string; // For legacy MCP tools
		narratives?: unknown[]; // For narrative queries
		narrative_count?: number;
	} | undefined;

	// Only proceed with object operations if parsedResult is actually an object
	if (typeof parsedResult === 'object' && parsedResult !== null) {
		result = parsedResult as typeof result;
	} else {
		// If it's still not an object, wrap it as an error
		result = { error: String(parsedResult) };
	}

	// Check if this is a map visualization
	const isMapVisualization = result?.success && result?.type === 'map_visualization';

	// Check if this is a pursuits widget
	const isPursuitsWidget = result?.success && result?.type === 'pursuits_widget';

	// Check if this is a narrative result (new MCP format) - with safe type guard
	const isNarrativeResult = result && typeof result === 'object' && 'narratives' in result;

	// Check if this is a generic MCP tool result (any object without success/type/rows)
	const isGenericMcpResult = result &&
		!result.success &&
		!result.type &&
		!result.rows &&
		!isMapVisualization &&
		typeof result === 'object';

	// Auto-expand map visualizations and pursuits widget
	let isExpanded = $state(isMapVisualization || isPursuitsWidget);

	// Extract reasoning from arguments (with safety checks)
	// For map tool, construct a description from the time range
	let reasoning: string;
	if (isMapVisualization) {
		const start = args?.startTime ? new Date(args.startTime as string).toLocaleDateString() : '';
		const end = args?.endTime ? new Date(args.endTime as string).toLocaleDateString() : '';
		reasoning = start && end ? `Map: ${start} to ${end}` : 'Visualizing location data';
	} else {
		reasoning = (args?.reasoning as string) || 'Tool executed';
	}
	const query = (args?.query as string) || '';

	// Format timestamp
	const formattedTime = new Date(timestamp).toLocaleTimeString([], {
		hour: '2-digit',
		minute: '2-digit'
	});
</script>

<div class="tool-call-item">
	<button
		class="tool-call-header"
		onclick={() => (isExpanded = !isExpanded)}
		aria-expanded={isExpanded}
	>
		<iconify-icon icon="ri:tools-line" class="tool-icon"></iconify-icon>
		<span class="tool-name">{tool_name}</span>
		<span class="tool-action">"{reasoning}"</span>
		{#if result}
			{#if result.error}
				<span class="tool-status error">Error</span>
			{:else if isMapVisualization}
				<span class="tool-status success">Map ready</span>
			{:else if isPursuitsWidget}
				<span class="tool-status success">{result.data?.metadata?.totalCount || 0} pursuits</span>
			{:else if isNarrativeResult}
				<span class="tool-status success">{result.narrative_count || 0} narratives</span>
			{:else if result.rawOutput}
				<span class="tool-status success">Success</span>
			{:else if result.rows}
				<span class="tool-status success">{result.rowCount || 0} rows</span>
			{:else if isGenericMcpResult}
				<span class="tool-status success">Success</span>
			{:else}
				<span class="tool-status success">Completed</span>
			{/if}
		{/if}
		<iconify-icon
			icon={isExpanded ? 'ri:arrow-up-s-line' : 'ri:arrow-down-s-line'}
			class="expand-icon"
		></iconify-icon>
	</button>

	{#if isExpanded}
		<div class="tool-call-details">
			{#if result}
				{#if result.error}
					<div class="detail-section">
						<div class="detail-label error">Error:</div>
						<div class="detail-value error">{result.error || 'Unknown error'}</div>
					</div>
				{:else if isMapVisualization}
					<!-- Render map visualization -->
					<div class="detail-section">
						<MapVisualization data={result.data} />
					</div>
				{:else if isPursuitsWidget}
					<!-- Render pursuits widget -->
					<div class="detail-section">
						<PursuitsWidget data={result.data} />
					</div>
				{:else if isNarrativeResult || isGenericMcpResult}
					<!-- Render MCP tool output (narratives or other generic results) -->
					<div class="detail-section">
						<div class="detail-label">Output:</div>
						<pre class="detail-code">{JSON.stringify(result, null, 2)}</pre>
					</div>
				{:else if result.rawOutput}
					<!-- Render legacy MCP tool output -->
					<div class="detail-section">
						<div class="detail-label">Output:</div>
						<pre class="detail-code">{result.rawOutput}</pre>
					</div>
				{:else if result.rows}
					<!-- Render standard query results -->
					{#if query}
						<div class="detail-section">
							<div class="detail-label">Query:</div>
							<pre class="detail-code">{query}</pre>
						</div>
					{/if}

					<div class="detail-section">
						<div class="detail-label">Results:</div>
						<div class="detail-value">
							{result.rowCount || 0} row{(result.rowCount || 0) !== 1 ? 's' : ''} returned
						</div>

						{#if result.columns && result.columns.length > 0}
							<div class="detail-value text-neutral-600 text-xs mt-1">
								Columns: {result.columns.join(', ')}
							</div>
						{/if}

						{#if result.rows && result.rows.length > 0 && result.rows.length <= 3}
							<div class="detail-value mt-2">
								<pre class="detail-code">{JSON.stringify(result.rows, null, 2)}</pre>
							</div>
						{:else if result.rows && result.rows.length > 3}
							<div class="detail-value text-neutral-500 text-xs mt-1">
								(Preview limited to first few rows)
							</div>
						{/if}
					</div>
				{/if}
			{/if}

			<div class="detail-section">
				<div class="detail-timestamp">{formattedTime}</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.tool-call-item {
		background-color: var(--color-paper);
		border: 1px solid var(--color-stone-300);
		border-radius: 0.5rem;
		margin-bottom: 1.5rem;
		overflow: hidden;
	}

	.tool-call-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.625rem 0.75rem;
		cursor: pointer;
		background: transparent;
		border: none;
		transition: background-color 0.15s ease;
		text-align: left;
	}

	.tool-call-header:hover {
		background-color: var(--color-paper-dark);
	}

	.tool-icon {
		color: var(--color-stone-600);
		font-size: 1rem;
		flex-shrink: 0;
	}

	.tool-name {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-navy);
		flex-shrink: 0;
	}

	.tool-action {
		font-size: 0.875rem;
		color: var(--color-stone-600);
		flex-grow: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tool-status {
		font-size: 0.75rem;
		padding: 0.125rem 0.5rem;
		border-radius: 0.25rem;
		font-weight: 500;
		flex-shrink: 0;
	}

	.tool-status.success {
		background-color: transparent;
		color: var(--color-stone-600);
	}

	.tool-status.error {
		background-color: rgb(254 226 226);
		color: rgb(153 27 27);
	}

	.expand-icon {
		color: var(--color-stone-600);
		font-size: 1.25rem;
		flex-shrink: 0;
	}

	.tool-call-details {
		padding: 0.75rem;
		border-top: 1px solid var(--color-stone-300);
		background-color: var(--color-white);
	}

	.detail-section {
		margin-bottom: 0.75rem;
	}

	.detail-section:last-child {
		margin-bottom: 0;
	}

	.detail-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-stone-700);
		text-transform: uppercase;
		letter-spacing: 0.025em;
		margin-bottom: 0.375rem;
	}

	.detail-label.error {
		color: rgb(153 27 27);
	}

	.detail-value {
		font-size: 0.875rem;
		color: var(--color-stone-800);
	}

	.detail-value.error {
		color: rgb(153 27 27);
	}

	.detail-code {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.8125rem;
		background-color: var(--color-paper);
		padding: 0.5rem;
		border-radius: 0.25rem;
		overflow-x: auto;
		color: var(--color-stone-800);
		margin: 0;
	}

	.detail-timestamp {
		font-size: 0.6875rem;
		color: var(--color-stone-600);
		text-align: right;
	}
</style>
