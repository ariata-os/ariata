import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { boundaryDetectionRuns, boundaryDetections, users } from '$lib/db/schema';
import { eq, and, gte, lte } from 'drizzle-orm';
import { z } from 'zod';

const runBoundaryDetectionSchema = z.object({
  userId: z.string().uuid(),
  startTime: z.string().datetime(),
  endTime: z.string().datetime(),
  useCustomRange: z.boolean().optional().default(false),
  timezone: z.string().optional()
});

export const POST: RequestHandler = async ({ request, locals }) => {
  try {
    const body = await request.json();
    const { userId, startTime, endTime, useCustomRange, timezone } = runBoundaryDetectionSchema.parse(body);

    // Get user's timezone if not provided
    let userTimezone = timezone;
    if (!userTimezone) {
      const [user] = await db.select({ timezone: users.timezone })
        .from(users)
        .where(eq(users.id, userId))
        .limit(1);

      userTimezone = user?.timezone || 'America/Chicago';
    }

    // Convert start/end times to date string (YYYY-MM-DD)
    const date = new Date(startTime).toISOString().split('T')[0];

    // Create a new run record
    const [run] = await db.insert(boundaryDetectionRuns).values({
      userId,
      runDate: new Date(),
      startTime: new Date(startTime),
      endTime: new Date(endTime),
      status: 'pending'
    }).returning();

    // Trigger Celery task via Redis
    try {
      // Import the queueCeleryTask function
      const { queueCeleryTask } = await import('$lib/redis');

      // Queue the boundary detection task with run ID
      // If using custom range, pass the times as additional parameters
      const taskArgs = useCustomRange
        ? [userId, date, 'manual', run.id, startTime, endTime, userTimezone]
        : [userId, date, 'manual', run.id, null, null, userTimezone];

      const taskId = await queueCeleryTask(
        'start_signal_analysis',
        taskArgs,
        {}
      );

      // For immediate feedback, we'll return the pending status
      // The client can poll the GET endpoint to check status
      return json({
        success: true,
        run,
        taskId,
        status: 'pending',
        message: 'Boundary detection task queued. Poll the status endpoint to check progress.'
      });

    } catch (error) {
      // Update run status to failed
      await db.update(boundaryDetectionRuns)
        .set({ status: 'failed' })
        .where(eq(boundaryDetectionRuns.id, run.id));

      throw error;
    }

  } catch (error) {
    console.error('Boundary detection error:', error);

    if (error instanceof z.ZodError) {
      return json({
        success: false,
        error: 'Invalid request data',
        details: error.errors
      }, { status: 400 });
    }

    return json({
      success: false,
      error: 'Failed to queue boundary detection',
      message: error instanceof Error ? error.message : 'Unknown error'
    }, { status: 500 });
  }
};

// GET endpoint to check the status of a run
export const GET: RequestHandler = async ({ url }) => {
  const runId = url.searchParams.get('runId');

  if (!runId) {
    return json({ error: 'Run ID required' }, { status: 400 });
  }

  const [run] = await db.select()
    .from(boundaryDetectionRuns)
    .where(eq(boundaryDetectionRuns.id, runId))
    .limit(1);

  if (!run) {
    return json({ error: 'Run not found' }, { status: 404 });
  }

  // If completed, also return the boundaries
  if (run.status === 'completed') {
    // Query boundaries created around the same time as the run
    const boundaries = await db.select()
      .from(boundaryDetections)
      .where(
        and(
          eq(boundaryDetections.userId, run.userId),
          // Look for boundaries created within a few seconds of the run completion
          gte(boundaryDetections.createdAt, new Date(run.createdAt.getTime() - 5000)),
          lte(boundaryDetections.createdAt, new Date(run.createdAt.getTime() + 60000))
        )
      )
      .orderBy(boundaryDetections.startTime);

    return json({
      run,
      boundaries
    });
  }

  return json({ run });
};