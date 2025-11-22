import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, locals }) => {
	const { apiClient } = locals;

	try {
		// Get query parameters
		const category = url.searchParams.get('category');

		// Build query params for Rust API
		const params = new URLSearchParams();
		if (category) {
			params.set('category', category);
		}

		// Call Rust API
		const response = await apiClient.get(`/tools?${params.toString()}`);

		if (!response.ok) {
			const error = await response.text();
			console.error('Failed to fetch tools:', error);
			return json({ error: 'Failed to fetch tools' }, { status: response.status });
		}

		const tools = await response.json();
		return json(tools);
	} catch (error) {
		console.error('Error fetching tools:', error);
		return json({ error: 'Internal server error' }, { status: 500 });
	}
};
