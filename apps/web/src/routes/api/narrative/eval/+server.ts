import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
    const { apiClient } = locals;

    try {
        const body = await request.json();

        // Call Rust API
        const response = await apiClient.post('/narrative/eval', body);

        if (!response.ok) {
            const error = await response.text();
            console.error('Failed to run narrative eval:', error);
            return json({ error: 'Failed to run narrative evaluation' }, { status: response.status });
        }

        const data = await response.json();
        return json(data);
    } catch (error) {
        console.error('Error running narrative eval:', error);
        return json({ error: 'Internal server error' }, { status: 500 });
    }
};
