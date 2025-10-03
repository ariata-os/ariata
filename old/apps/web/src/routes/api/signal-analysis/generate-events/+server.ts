import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	const { date, minClusterSize = 3, epsilon = 300 } = await request.json();

	if (!date) {
		return json({ error: 'Date is required' }, { status: 400 });
	}

	try {
		// For now, directly call the processing API endpoint
		const processingUrl = process.env.PUBLIC_PROCESSING_URL || 'http://processing:8001';
		
		const response = await fetch(`${processingUrl}/api/events/generate`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				date,
				min_cluster_size: minClusterSize,
				epsilon
			})
		});

		if (!response.ok) {
			throw new Error(`Processing service error: ${response.statusText}`);
		}

		const result = await response.json();
		
		return json({
			success: true,
			...result
		});
	} catch (error) {
		console.error('Error generating events:', error);
		return json(
			{ error: error instanceof Error ? error.message : 'Failed to generate events' },
			{ status: 500 }
		);
	}
};