import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals }) => {
	const { apiClient } = locals;

	try {
		// Call Rust API to get pinned tools with metadata
		const response = await apiClient.get('/assistant-profile/pinned-tools');

		if (!response.ok) {
			const error = await response.text();
			console.error('Failed to fetch pinned tools:', error);
			return json({ error: 'Failed to fetch pinned tools' }, { status: response.status });
		}

		const tools = await response.json();
		return json(tools);
	} catch (error) {
		console.error('Error fetching pinned tools:', error);
		return json({ error: 'Internal server error' }, { status: 500 });
	}
};
