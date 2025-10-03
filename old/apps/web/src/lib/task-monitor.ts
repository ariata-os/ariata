export interface TaskStatus {
	task_id: string;
	status: 'PENDING' | 'RUNNING' | 'SUCCESS' | 'FAILURE' | 'RETRY' | 'REVOKED' | 'UNKNOWN';
	result?: any;
	error?: string;
}

export class TaskMonitor {
	private pollInterval: number;
	private maxRetries: number;
	private callbacks: Map<string, (status: TaskStatus) => void> = new Map();
	private timers: Map<string, NodeJS.Timeout> = new Map();

	constructor(pollInterval = 2000, maxRetries = 30) {
		this.pollInterval = pollInterval;
		this.maxRetries = maxRetries;
	}

	async monitorTask(
		taskId: string,
		callback: (status: TaskStatus) => void,
		onComplete?: (status: TaskStatus) => void
	): Promise<void> {
		this.callbacks.set(taskId, callback);
		
		let retries = 0;
		
		const poll = async () => {
			try {
				const response = await fetch(`/api/tasks/${taskId}/status`);
				const status: TaskStatus = await response.json();
				
				callback(status);
				
				// Check if task is complete
				if (['SUCCESS', 'FAILURE', 'REVOKED'].includes(status.status)) {
					this.stopMonitoring(taskId);
					onComplete?.(status);
					return;
				}
				
				// Check if we've exceeded max retries
				if (retries >= this.maxRetries) {
					this.stopMonitoring(taskId);
					callback({
						task_id: taskId,
						status: 'UNKNOWN',
						error: 'Monitoring timeout exceeded'
					});
					return;
				}
				
				retries++;
				
				// Schedule next poll
				const timer = setTimeout(poll, this.pollInterval);
				this.timers.set(taskId, timer);
				
			} catch (error) {
				console.error('Error polling task status:', error);
				retries++;
				
				if (retries < this.maxRetries) {
					const timer = setTimeout(poll, this.pollInterval);
					this.timers.set(taskId, timer);
				} else {
					this.stopMonitoring(taskId);
					callback({
						task_id: taskId,
						status: 'UNKNOWN',
						error: 'Failed to get task status'
					});
				}
			}
		};
		
		// Start polling
		poll();
	}

	stopMonitoring(taskId: string): void {
		const timer = this.timers.get(taskId);
		if (timer) {
			clearTimeout(timer);
			this.timers.delete(taskId);
		}
		this.callbacks.delete(taskId);
	}

	stopAllMonitoring(): void {
		for (const [taskId] of this.timers) {
			this.stopMonitoring(taskId);
		}
	}
}

// Export a singleton instance
export const taskMonitor = new TaskMonitor();