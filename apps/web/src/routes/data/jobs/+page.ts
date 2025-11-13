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

		// Fetch jobs and sources in parallel
		const [jobsRes, sourcesRes] = await Promise.all([
			fetch(`/api/jobs?${queryParams}`),
			fetch(`/api/sources`)
		]);

		if (!jobsRes.ok) {
			throw new Error(`Failed to query jobs: ${jobsRes.statusText}`);
		}
		if (!sourcesRes.ok) {
			throw new Error(`Failed to query sources: ${sourcesRes.statusText}`);
		}

		const jobs: Job[] = await jobsRes.json();
		const sources: any[] = await sourcesRes.json();

		// Create a map of source_id -> source_name for fast lookup
		const sourceMap = new Map(
			sources.map((source) => [source.id, source.name])
		);

		// Enrich jobs with source names
		const enrichedJobs = jobs.map((job) => ({
			...job,
			source_name: job.source_id ? sourceMap.get(job.source_id) : undefined
		}));

		return {
			jobs: enrichedJobs,
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
