import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { createClient } from 'redis';
import { env } from '$env/dynamic/private';

export const GET: RequestHandler = async ({ url }) => {
  const taskId = url.searchParams.get('taskId');
  
  if (!taskId) {
    return json({ 
      success: false, 
      error: 'Task ID is required' 
    }, { status: 400 });
  }

  const redis = createClient({
    url: env.REDIS_URL || 'redis://localhost:6379'
  });

  try {
    await redis.connect();
    
    // Check Celery task result
    const resultKey = `celery-task-meta-${taskId}`;
    const result = await redis.get(resultKey);
    
    if (!result) {
      return json({
        success: false,
        error: 'Task not found or not yet processed',
        taskId
      });
    }
    
    const taskResult = JSON.parse(result);
    
    return json({
      success: true,
      taskId,
      status: taskResult.status,
      result: taskResult.result,
      traceback: taskResult.traceback,
      date_done: taskResult.date_done,
      task_id: taskResult.task_id
    });
    
  } catch (error) {
    console.error('Task status check error:', error);
    return json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error',
      taskId
    }, { status: 500 });
  } finally {
    await redis.disconnect();
  }
};