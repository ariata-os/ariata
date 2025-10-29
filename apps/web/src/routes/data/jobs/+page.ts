import type { Job } from '$lib/api/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ url, fetch }) => {
	try {
		// Get query parameters for filtering
		const sourceId = url.searchParams.get('source_id') || undefined;
		const statusParam = url.searchParams.get('status');
		const limitParam = url.searchParams.get('limit');

		// Parse status filter (comma-separated or single value)
		const status = statusParam
			? statusParam.split(',').map((s) => s.trim())
			: undefined;

		// Parse limit with default
		const limit = limitParam ? parseInt(limitParam, 10) : 100;

		// Build query params for API call
		const queryParams = new URLSearchParams();
		if (sourceId) queryParams.set('source_id', sourceId);
		if (status && status.length > 0) {
			queryParams.set('status', status.join(','));
		}
		if (limit) queryParams.set('limit', limit.toString());

		// Fetch jobs from API using SvelteKit's fetch with relative URL
		const res = await fetch(`/api/jobs?${queryParams}`);
		if (!res.ok) {
			throw new Error(`Failed to query jobs: ${res.statusText}`);
		}
		const jobs: Job[] = await res.json();

		return {
			jobs,
			filters: {
				sourceId,
				status,
				limit
			}
		};
	} catch (err) {
		console.error('Failed to load jobs:', err);
		return {
			jobs: [],
			filters: {
				sourceId: undefined,
				status: undefined,
				limit: 100
			},
			error: err instanceof Error ? err.message : 'Failed to load jobs'
		};
	}
};
