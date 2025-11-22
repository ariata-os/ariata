import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, locals }) => {
	const { apiClient } = locals;

	try {
		// Forward query parameters (start and end dates)
		const start = url.searchParams.get('start');
		const end = url.searchParams.get('end');

		if (!start || !end) {
			return json(
				{ error: 'Missing required query parameters: start and end' },
				{ status: 400 }
			);
		}

		const queryParams = new URLSearchParams({ start, end });

		// Call Rust API
		const response = await apiClient.get(`/seed/data-quality?${queryParams}`);

		if (!response.ok) {
			const error = await response.text();
			console.error('Failed to fetch data quality metrics:', error);
			return json(
				{ error: 'Failed to fetch data quality metrics' },
				{ status: response.status }
			);
		}

		const data = await response.json();
		return json(data);
	} catch (error) {
		console.error('Error fetching data quality metrics:', error);
		return json({ error: 'Internal server error' }, { status: 500 });
	}
};
