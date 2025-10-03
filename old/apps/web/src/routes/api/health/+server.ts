import { json } from "@sveltejs/kit";
import { sql } from "drizzle-orm";
import Redis from "ioredis";
import { env } from "$env/dynamic/private";
import { db } from "$lib/db/client";
import type { RequestHandler } from "./$types";

export const GET: RequestHandler = async () => {
      const health: Record<string, any> = {
            status: "checking",
            timestamp: new Date().toISOString(),
            services: {},
      };

      // Check PostgreSQL
      try {
            const result = await db.execute(sql`SELECT 1 as health`);
            health.services.postgres = { status: "healthy", connected: true };
      } catch (error) {
            health.services.postgres = {
                  status: "unhealthy",
                  connected: false,
                  error: error instanceof Error ? error.message : "Unknown error",
            };
      }

      // Check Redis
      const redis = new Redis(env.REDIS_URL || "redis://localhost:6379");

      try {
            const pong = await redis.ping();
            health.services.redis = {
                  status: "healthy",
                  connected: pong === "PONG",
            };

            // Check Celery queues
            const celeryQueueLength = await redis.llen("celery");
            health.services.celery = {
                  status: "healthy",
                  queueLength: celeryQueueLength,
            };
      } catch (error) {
            health.services.redis = {
                  status: "unhealthy",
                  connected: false,
                  error: error instanceof Error ? error.message : "Unknown error",
            };
            health.services.celery = {
                  status: "unknown",
                  error: "Redis not available",
            };
      } finally {
            await redis.disconnect();
      }

      // Check MinIO (simple connectivity check)
      try {
            const minioUrl = env.MINIO_ENDPOINT || "http://minio:9000";
            const response = await fetch(`${minioUrl}/minio/health/live`, {
                  signal: AbortSignal.timeout(5000),
            });
            health.services.minio = {
                  status: response.ok ? "healthy" : "unhealthy",
                  statusCode: response.status,
            };
      } catch (error) {
            health.services.minio = {
                  status: "unhealthy",
                  error: error instanceof Error ? error.message : "Cannot reach MinIO",
            };
      }

      // Overall status
      const allHealthy = Object.values(health.services).every((service: any) => service.status === "healthy");

      health.status = allHealthy ? "healthy" : "degraded";
      health.ready = allHealthy;

      return json(health, {
            status: allHealthy ? 200 : 503,
            headers: {
                  "Cache-Control": "no-cache, no-store, must-revalidate",
            },
      });
};
