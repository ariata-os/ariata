<script lang="ts">
	/**
	 * ToolCall - Renders tool execution results using modular tool components
	 * Each tool has its own component in components/tools/
	 * Components are lazy-loaded for better performance
	 */
	import BaseTool from './tools/BaseTool.svelte';
	import type { ComponentType } from 'svelte';

	// Lazy load tool components
	const toolComponents: Record<string, () => Promise<{ default: ComponentType }>> = {
		'query_location_map': () => import('./tools/LocationMap.svelte'),
		'query_pursuits': () => import('./tools/Pursuits.svelte'),
		'web_search': () => import('./tools/WebSearch.svelte'),
	};

	let ToolComponent = $state<ComponentType | null>(null);
	let toolDataToPass = $state<unknown>(null);

	interface ToolCallProps {
		tool_name: string;
		arguments: Record<string, unknown>;
		result?: unknown;
		timestamp: string;
	}

	let { tool_name, arguments: args, result: rawResult, timestamp }: ToolCallProps = $props();

	// Parse result if it's a JSON string
	let parsedResult: unknown = rawResult;
	if (typeof rawResult === 'string') {
		try {
			parsedResult = JSON.parse(rawResult);
		} catch (e) {
			console.error('[ToolCall] Failed to parse result JSON:', e);
			parsedResult = { error: rawResult };
		}
	}

	// Type guard for result object
	let result: {
		success?: boolean;
		type?: string;
		data?: unknown;
		rowCount?: number;
		rows?: unknown[];
		columns?: string[];
		error?: string;
		rawOutput?: string;
		narratives?: unknown[];
		narrative_count?: number;
		query?: string;
		resultsCount?: number;
		searchType?: string;
		results?: unknown[];
		metadata?: Record<string, unknown>;
	} | undefined;

	if (typeof parsedResult === 'object' && parsedResult !== null) {
		result = parsedResult as typeof result;
	} else {
		result = { error: String(parsedResult) };
	}

	// Extract reasoning from arguments
	let reasoning: string;
	if (tool_name === 'query_location_map') {
		const start = args?.startTime ? new Date(args.startTime as string).toLocaleDateString() : '';
		const end = args?.endTime ? new Date(args.endTime as string).toLocaleDateString() : '';
		reasoning = start && end ? `Map: ${start} to ${end}` : 'Visualizing location data';
	} else {
		reasoning = (args?.reasoning as string) || (args?.query as string) || 'Tool executed';
	}

	// Determine tool status and status text
	const hasError = !!result?.error;
	const status = hasError ? 'error' : 'success';
	let statusText: string;

	if (hasError) {
		statusText = 'Error';
	} else if (tool_name === 'query_location_map') {
		statusText = 'Map ready';
	} else if (tool_name === 'query_pursuits') {
		statusText = `${(result?.data as any)?.metadata?.totalCount || 0} pursuits`;
	} else if (tool_name === 'web_search') {
		statusText = `${result?.resultsCount || 0} results`;
	} else if (result?.narratives) {
		statusText = `${result.narrative_count || 0} narratives`;
	} else if (result?.rawOutput) {
		statusText = 'Success';
	} else if (result?.rows) {
		statusText = `${result.rowCount || 0} rows`;
	} else {
		statusText = 'Completed';
	}

	// Determine if tool should auto-expand
	const autoExpand = tool_name === 'query_location_map' || tool_name === 'query_pursuits' || tool_name === 'web_search';

	// Get display name for tool
	const displayNames: Record<string, string> = {
		query_location_map: 'Location Map',
		query_pursuits: 'Tasks & Goals',
		web_search: 'Web Search',
		ariata_query_ontology: 'Query Ontology',
		ariata_list_ontology_tables: 'List Tables',
		ariata_get_table_schema: 'Table Schema',
		ariata_query_narratives: 'Query Narratives',
		ariata_query_axiology: 'Query Axiology',
		ariata_trigger_sync: 'Trigger Sync'
	};
	const displayName = displayNames[tool_name] || tool_name;

	// Lazy load the appropriate tool component
	$effect(() => {
		if (tool_name && toolComponents[tool_name] && result?.success) {
			toolComponents[tool_name]().then(module => {
				ToolComponent = module.default;
				// Set the data to pass based on tool type
				if (tool_name === 'web_search') {
					toolDataToPass = result;
				} else {
					toolDataToPass = result?.data;
				}
			});
		} else {
			ToolComponent = null;
			toolDataToPass = null;
		}
	});
</script>

<BaseTool
	toolName={tool_name}
	{displayName}
	{reasoning}
	{status}
	{statusText}
	{timestamp}
	{autoExpand}
	errorMessage={result?.error}
>
	{#if !result?.error}
		{#if ToolComponent && toolDataToPass}
			<!-- Dynamically loaded tool component -->
			<div class="detail-section">
				<ToolComponent data={toolDataToPass} />
			</div>
		{:else if result?.narratives || (result && typeof result === 'object' && !result.success && !result.type && !result.rows)}
			<!-- MCP tool output or narratives -->
			<div class="detail-section">
				<div class="detail-label">Output:</div>
				<pre class="detail-code">{JSON.stringify(result, null, 2)}</pre>
			</div>
		{:else if result?.rawOutput}
			<!-- Legacy MCP tool output -->
			<div class="detail-section">
				<div class="detail-label">Output:</div>
				<pre class="detail-code">{result.rawOutput}</pre>
			</div>
		{:else if result?.rows}
			<!-- Database query results -->
			{#if args?.query}
				<div class="detail-section">
					<div class="detail-label">Query:</div>
					<pre class="detail-code">{args.query}</pre>
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
</BaseTool>

<style>
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

	.detail-value {
		font-size: 0.875rem;
		color: var(--color-stone-800);
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
</style>
