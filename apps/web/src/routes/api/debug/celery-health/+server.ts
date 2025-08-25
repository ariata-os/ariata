import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { createClient } from 'redis';
import { env } from '$env/dynamic/private';

export const GET: RequestHandler = async () => {
  const redis = createClient({
    url: env.REDIS_URL || 'redis://localhost:6379'
  });

  try {
    await redis.connect();
    
    // Check Redis connection
    const redisPing = await redis.ping();
    
    // Check Celery queues
    const celeryQueue = await redis.lLen('celery');
    const priorityQueue = await redis.lLen('priority');
    
    // Get some recent task IDs from the queue
    const recentTasks = await redis.lRange('celery', 0, 4);
    
    return json({
      success: true,
      redis: {
        connected: redisPing === 'PONG',
        ping: redisPing
      },
      queues: {
        celery: celeryQueue,
        priority: priorityQueue
      },
      recentTasks: recentTasks.slice(0, 5),
      timestamp: new Date().toISOString()
    });
    
  } catch (error) {
    console.error('Celery health check error:', error);
    return json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error'
    }, { status: 500 });
  } finally {
    await redis.disconnect();
  }
};