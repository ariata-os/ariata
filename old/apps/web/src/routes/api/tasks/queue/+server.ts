import { type RequestHandler } from '@sveltejs/kit';
import { getRedisClient } from '$lib/redis';

export const GET: RequestHandler = async () => {
	try {
		const client = getRedisClient();
		
		// Get queue length
		const queueLength = await client.llen('celery');
		
		// Get active tasks from Celery's active queue (if using Redis backend)
		// This is a simplified version - in production you might need to check
		// multiple queues based on your Celery configuration
		const activeKeys = await client.keys('celery-task-meta-*');
		
		let activeTasks = 0;
		let pendingTasks = 0;
		const taskDetails = [];
		
		// Check each task's status
		for (const key of activeKeys) {
			const taskData = await client.get(key);
			if (taskData) {
				try {
					const task = JSON.parse(taskData);
					if (task.status === 'PENDING' || task.status === 'STARTED') {
						activeTasks++;
						taskDetails.push({
							id: key.replace('celery-task-meta-', ''),
							status: task.status,
							task: task.task,
							args: task.args,
							timestamp: task.timestamp
						});
					}
				} catch (e) {
					// Skip malformed task data
				}
			}
		}
		
		return new Response(JSON.stringify({
			queue: {
				length: queueLength,
				name: 'celery'
			},
			active: {
				count: activeTasks,
				tasks: taskDetails.slice(0, 10) // Limit to 10 most recent
			},
			pending: {
				count: queueLength
			},
			timestamp: new Date().toISOString()
		}), {
			headers: { 'Content-Type': 'application/json' }
		});
		
	} catch (error) {
		console.error('Failed to get queue status:', error);
		return new Response(JSON.stringify({ 
			error: 'Failed to get queue status',
			queue: { length: 0, name: 'celery' },
			active: { count: 0, tasks: [] },
			pending: { count: 0 }
		}), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};