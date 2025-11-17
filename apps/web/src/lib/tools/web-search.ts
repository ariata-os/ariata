import { z } from 'zod';

/**
 * Exa Web Search Tool for AI SDK v6
 *
 * Provides web search capabilities using Exa AI's search API.
 * Exa specializes in AI-optimized web search with neural and keyword search.
 *
 * @see https://docs.exa.ai for API documentation
 */
export async function createWebSearchTool() {
	const apiKey = process.env.EXA_API_KEY;

	if (!apiKey) {
		console.warn('[webSearch] EXA_API_KEY not configured - tool will not be available');
		return null;
	}

	const inputSchema = z.object({
		query: z.string().describe('The search query to find relevant web content'),
		numResults: z
			.number()
			.min(1)
			.max(10)
			.optional()
			.describe('Number of search results to return (default: 5, max: 10)'),
		type: z
			.enum(['auto', 'keyword', 'neural'])
			.optional()
			.describe('Search type: auto (intelligent hybrid), keyword (exact matches), neural (semantic search). Default: auto'),
		category: z
			.enum(['company', 'research paper', 'news', 'pdf', 'github', 'personal site', 'linkedin profile', 'financial report'])
			.optional()
			.describe('Filter results by content category'),
		includeDomains: z
			.array(z.string())
			.optional()
			.describe('Only include results from these domains (e.g., ["github.com", "stackoverflow.com"])'),
		excludeDomains: z
			.array(z.string())
			.optional()
			.describe('Exclude results from these domains'),
		startPublishedDate: z
			.string()
			.optional()
			.describe('Filter results published after this date (ISO 8601 format)'),
		endPublishedDate: z
			.string()
			.optional()
			.describe('Filter results published before this date (ISO 8601 format)')
	});

	return {
		description:
			'Search the web using Exa AI. Returns relevant, high-quality web search results with content summaries. Best for finding recent information, research papers, company info, or specific domain knowledge.',
		inputSchema,
		execute: async ({
			query,
			numResults = 5,
			type = 'auto',
			category,
			includeDomains,
			excludeDomains,
			startPublishedDate,
			endPublishedDate
		}: z.infer<typeof inputSchema>) => {
			console.log('[webSearch] Executing search:', { query, numResults, type, category });

			try {
				// Build the Exa API request
				const requestBody: any = {
					query,
					numResults,
					type,
					contents: {
						text: {
							maxCharacters: 1000,
							includeHtmlTags: false
						},
						summary: true
					}
				};

				// Add optional filters
				if (category) requestBody.category = category;
				if (includeDomains && includeDomains.length > 0) {
					requestBody.includeDomains = includeDomains;
				}
				if (excludeDomains && excludeDomains.length > 0) {
					requestBody.excludeDomains = excludeDomains;
				}
				if (startPublishedDate) requestBody.startPublishedDate = startPublishedDate;
				if (endPublishedDate) requestBody.endPublishedDate = endPublishedDate;

				// Call Exa API
				const response = await fetch('https://api.exa.ai/search', {
					method: 'POST',
					headers: {
						'Content-Type': 'application/json',
						'x-api-key': apiKey
					},
					body: JSON.stringify(requestBody)
				});

				if (!response.ok) {
					const errorText = await response.text();
					console.error('[webSearch] API error:', response.status, errorText);
					throw new Error(`Exa API error (${response.status}): ${errorText}`);
				}

				const data = await response.json();
				console.log(`[webSearch] Found ${data.results?.length || 0} results`);

				// Format results for the AI
				const formattedResults = (data.results || []).map((result: any, index: number) => ({
					position: index + 1,
					title: result.title,
					url: result.url,
					publishedDate: result.publishedDate,
					author: result.author,
					summary: result.summary,
					text: result.text?.substring(0, 500), // Truncate to 500 chars for context
					score: result.score
				}));

				return {
					success: true,
					type: 'web_search_results',
					query,
					resultsCount: formattedResults.length,
					searchType: type,
					results: formattedResults,
					metadata: {
						autopromptString: data.autopromptString, // Exa's query refinement suggestion
						requestId: data.requestId
					}
				};
			} catch (error: any) {
				console.error('[webSearch] Error:', error.message);
				console.error('[webSearch] Stack:', error.stack);

				return {
					success: false,
					error: error.message,
					query,
					resultsCount: 0,
					results: []
				};
			}
		}
	};
}
