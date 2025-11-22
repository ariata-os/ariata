import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, locals }) => {
	const { apiClient } = locals;
	const { date } = params;

	try {
		// Call Rust API
		const response = await apiClient.get(`/timeline/day/${date}`);

		if (!response.ok) {
			const error = await response.text();
			console.error('Failed to fetch timeline day view:', error);
			return json({ error: 'Failed to fetch timeline day view' }, { status: response.status });
		}

		const data = await response.json();
		return json(data);
	} catch (error) {
		console.error('Error fetching timeline day view:', error);
		return json({ error: 'Internal server error' }, { status: 500 });
	}
};
