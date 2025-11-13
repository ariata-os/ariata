import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
	try {
		const [sourcesRes, catalogRes] = await Promise.all([
			fetch('/api/sources'),
			fetch('/api/catalog/sources')
		]);

		if (!sourcesRes.ok) {
			throw new Error(`Failed to load sources: ${sourcesRes.statusText}`);
		}
		if (!catalogRes.ok) {
			throw new Error(`Failed to load catalog: ${catalogRes.statusText}`);
		}

		const allSources = await sourcesRes.json();
		const catalog = await catalogRes.json();

		return {
			sources: allSources,
			catalog
		};
	} catch (error) {
		console.error('Failed to load sources:', error);
		return {
			sources: [],
			catalog: [],
			error: error instanceof Error ? error.message : 'Failed to load sources'
		};
	}
};
