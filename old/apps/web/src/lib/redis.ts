import Redis from 'ioredis';

// Get Redis URL and handle different environments
function getRedisUrl(): string {
  const isDocker = process.env.DOCKER_CONTAINER === 'true';
  let redisUrl = process.env.REDIS_URL || 'redis://localhost:6379';
  
  // If we're in Docker but the URL points to localhost, fix it
  if (isDocker && redisUrl.includes('://localhost:')) {
    console.log('Docker environment detected, replacing localhost with redis hostname');
    redisUrl = redisUrl.replace('://localhost:', '://redis:');
  }
  
  console.log('Redis URL:', redisUrl);
  return redisUrl;
}

const REDIS_URL = getRedisUrl();

let redis: Redis | null = null;

export function getRedisClient(): Redis {
	if (!redis) {
		redis = new Redis(REDIS_URL, {
			enableReadyCheck: true,
			maxRetriesPerRequest: 3,
			lazyConnect: false, // Connect immediately
			keepAlive: 30000,
			connectTimeout: 10000,
			commandTimeout: 5000,
		});

		redis.on('error', (err) => {
			console.error('Redis connection error:', err);
		});

		redis.on('connect', () => {
			console.log('Redis connected successfully');
		});

		redis.on('reconnecting', () => {
			console.log('Redis reconnecting...');
		});
	}

	return redis;
}

export interface CeleryTask {
	id: string;
	task: string;
	args: any[];
	kwargs: Record<string, any>;
	retries: number;
	eta: string | null;
	expires: string | null;
	utc: boolean;
	callback: string | null;
	errback: string | null;
	timelimit: [number | null, number | null];
	taskset: string | null;
	chord: string | null;
	group: string | null;
	root_id: string | null;
	parent_id: string | null;
	origin: string | null;
}

export async function queueCeleryTask(
	taskName: string,
	args: any[] = [],
	kwargs: Record<string, any> = {},
	options: {
		taskId?: string;
		eta?: Date;
		expires?: Date;
		retries?: number;
	} = {}
): Promise<string> {
	const client = getRedisClient();
	
	const taskId = options.taskId || generateTaskId();
	
	const task: CeleryTask = {
		id: taskId,
		task: taskName,
		args,
		kwargs,
		retries: options.retries || 0,
		eta: options.eta?.toISOString() || null,
		expires: options.expires?.toISOString() || null,
		utc: true,
		callback: null,
		errback: null,
		timelimit: [null, null],
		taskset: null,
		chord: null,
		group: null,
		root_id: null,
		parent_id: null,
		origin: null,
	};

	// Celery expects messages in this format
	const message = {
		body: Buffer.from(JSON.stringify([args, kwargs, {}])).toString('base64'),
		'content-encoding': 'utf-8',
		'content-type': 'application/json',
		headers: {
			id: taskId,
			task: taskName,
			root_id: taskId,
			parent_id: null,
			group: null,
			origin: 'sveltekit-web',
			lang: 'py',
			retries: 0,
			eta: task.eta,
			expires: task.expires,
			utc: true,
			argsrepr: `(${args.map(a => JSON.stringify(a)).join(', ')})`,
			kwargsrepr: '{}',
		},
		properties: {
			correlation_id: taskId,
			reply_to: null,
			delivery_mode: 2,
			delivery_info: {
				exchange: '',
				routing_key: taskName === 'process_stream_batch' ? 'priority' : 'celery',
			},
			delivery_tag: generateDeliveryTag(),
			priority: 0,
			body_encoding: 'base64',
		},
	};

	// Push to Celery queue
	try {
		// In Celery with Redis, messages are stored as JSON strings
		const messageStr = JSON.stringify(message);
		const queueName = taskName === 'process_stream_batch' ? 'priority' : 'celery';
		
		await client.lpush(queueName, messageStr);
	} catch (error) {
		console.error('[REDIS] Failed to push task to Redis:', error);
		throw error;
	}
	
	return taskId;
}

export async function getTaskStatus(taskId: string): Promise<{
	status: string;
	result?: any;
	error?: string;
}> {
	const client = getRedisClient();
	
	try {
		const result = await client.get(`celery-task-meta-${taskId}`);
		
		if (!result) {
			return { status: 'PENDING' };
		}
		
		const parsed = JSON.parse(result);
		return {
			status: parsed.status,
			result: parsed.result,
			error: parsed.traceback,
		};
	} catch (error) {
		console.error('Error getting task status:', error);
		return { status: 'UNKNOWN', error: 'Failed to get task status' };
	}
}

function generateTaskId(): string {
	return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
}

function generateDeliveryTag(): number {
	// Celery expects delivery_tag to be an integer
	return Math.floor(Date.now() / 1000) + Math.floor(Math.random() * 1000);
}

export async function closeRedisConnection(): Promise<void> {
	if (redis) {
		await redis.quit();
		redis = null;
	}
}