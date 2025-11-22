import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals }) => {
	const { apiClient } = locals;

	try {
		// Call Rust API
		const response = await apiClient.get('/ontologies/overview');

		if (!response.ok) {
			const error = await response.text();
			console.error('Failed to fetch ontologies overview:', error);
			return json({ error: 'Failed to fetch ontologies overview' }, { status: response.status });
		}

		const data = await response.json();
		return json(data);
	} catch (error) {
		console.error('Error fetching ontologies overview:', error);
		return json({ error: 'Internal server error' }, { status: 500 });
	}
};
