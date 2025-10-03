import { type RequestHandler } from '@sveltejs/kit';
import { getTaskStatus } from '$lib/redis';

export const GET: RequestHandler = async ({ params }) => {
	try {
		const taskId = params.task_id;

		if (!taskId) {
			return new Response(JSON.stringify({ error: 'Task ID is required' }), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		const status = await getTaskStatus(taskId);

		return new Response(JSON.stringify({
			task_id: taskId,
			...status
		}), {
			headers: { 'Content-Type': 'application/json' }
		});

	} catch (error) {
		console.error('Failed to get task status:', error);
		return new Response(JSON.stringify({ 
			error: 'Failed to get task status',
			status: 'UNKNOWN'
		}), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};