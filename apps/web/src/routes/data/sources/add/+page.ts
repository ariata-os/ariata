import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
	try {
		const catalogRes = await fetch('/api/catalog/sources');
		if (!catalogRes.ok) {
			throw new Error(`Failed to load catalog: ${catalogRes.statusText}`);
		}

		const catalog = await catalogRes.json();

		// Check if we're configuring an existing source (after OAuth callback)
		const sourceId = url.searchParams.get('source_id');

		if (sourceId) {
			// Fetch the source that was just created
			const sourceRes = await fetch(`/api/sources/${sourceId}`);
			const existingSource = await sourceRes.json();

			// Fetch available streams for this source
			const streamsRes = await fetch(`/api/sources/${sourceId}/streams`);
			const availableStreams = await streamsRes.json();

			return {
				catalog,
				selectedSource: null,
				existingSource,
				availableStreams
			};
		}

		// Check if a specific source type was requested via query param
		const typeParam = url.searchParams.get('type');
		const selectedSource = typeParam
			? catalog.find((s: any) => s.name === typeParam)
			: null;

		return {
			catalog,
			selectedSource,
			existingSource: null,
			availableStreams: []
		};
	} catch (error) {
		console.error('Failed to load sources catalog:', error);
		return {
			catalog: [],
			selectedSource: null,
			existingSource: null,
			availableStreams: [],
			error: error instanceof Error ? error.message : 'Failed to load catalog'
		};
	}
};
