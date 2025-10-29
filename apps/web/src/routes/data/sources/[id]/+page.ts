import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
	const sourceId = params.id;

	try {
		const [sourceRes, streamsRes, catalogRes] = await Promise.all([
			fetch(`/api/sources/${sourceId}`),
			fetch(`/api/sources/${sourceId}/streams`),
			fetch(`/api/catalog/sources`)
		]);

		if (!sourceRes.ok) {
			throw new Error(`Failed to load source: ${sourceRes.statusText}`);
		}
		if (!streamsRes.ok) {
			throw new Error(`Failed to load streams: ${streamsRes.statusText}`);
		}
		if (!catalogRes.ok) {
			throw new Error(`Failed to load catalog: ${catalogRes.statusText}`);
		}

		const source = await sourceRes.json();
		const streams = await streamsRes.json();
		const catalog = await catalogRes.json();

		return {
			source,
			streams,
			catalog
		};
	} catch (error) {
		console.error('Failed to load source details:', error);
		return {
			source: null,
			streams: [],
			catalog: [],
			error: error instanceof Error ? error.message : 'Failed to load source details'
		};
	}
};
