/**
 * iOS Test Endpoint Server - Vercel Serverless Function
 * Simple serverless API for testing iOS device pairing - matches Rust backend API exactly
 */

import type { VercelRequest, VercelResponse } from '@vercel/node';

interface DeviceInfo {
  device_id: string;
  device_name: string;
  device_model: string;
  os_version: string;
  app_version?: string;
}

interface InitiatePairingRequest {
  device_type: string;
  name?: string;
}

interface InitiatePairingResponse {
  source_id: string;
  code: string;
  expires_at: string;
}

interface CompletePairingRequest {
  code: string;
  device_info: DeviceInfo;
}

interface CompletePairingResponse {
  source_id: string;
  device_token: string;
  available_streams: StreamDescriptor[];
}

interface StreamDescriptor {
  name: string;
  display_name: string;
  description: string;
  table_name: string;
  config_schema: Record<string, unknown>;
  config_example: Record<string, unknown>;
  supports_incremental: boolean;
  supports_full_refresh: boolean;
  default_cron_schedule: string | null;
}

interface StreamInfo {
  stream_name: string;
  display_name: string;
  description: string;
  is_enabled: boolean;
  config: Record<string, unknown>;
  supports_incremental: boolean;
  default_cron_schedule: string | null;
  table_name: string;
  cron_schedule: string | null;
  last_sync_at: string | null;
  supports_full_refresh: boolean;
  config_schema: Record<string, unknown>;
  config_example: Record<string, unknown>;
}

interface VerifyResponse {
  source_id: string;
  configuration_complete: boolean;
  enabled_streams: StreamInfo[];
}

// Generate random pairing code
function generatePairingCode(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return code;
}

// Generate device token
function generateDeviceToken(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return Buffer.from(bytes).toString('base64');
}

// Generate UUID v4
function generateUUID(): string {
  return crypto.randomUUID();
}

// Get available iOS streams
function getAvailableStreams(): StreamDescriptor[] {
  return [
    {
      name: "healthkit",
      display_name: "HealthKit",
      description: "Health and fitness metrics including heart rate, steps, sleep, and workouts",
      table_name: "stream_ios_healthkit",
      config_schema: {
        type: "object",
        properties: {
          enabled_metrics: {
            type: "array",
            items: { type: "string" },
            description: "List of metrics to collect: heart_rate, steps, sleep, workouts, etc."
          },
          sampling_interval_seconds: {
            type: "integer",
            default: 60,
            minimum: 10,
            description: "How often to sample health metrics (in seconds)"
          }
        }
      },
      config_example: {
        enabled_metrics: ["heart_rate", "steps", "sleep"],
        sampling_interval_seconds: 60
      },
      supports_incremental: false,
      supports_full_refresh: false,
      default_cron_schedule: "*/5 * * * *"
    },
    {
      name: "location",
      display_name: "Location",
      description: "GPS coordinates, speed, altitude, and activity type",
      table_name: "stream_ios_location",
      config_schema: {
        type: "object",
        properties: {
          accuracy: {
            type: "string",
            enum: ["best", "high", "medium", "low"],
            default: "high",
            description: "GPS accuracy level"
          },
          update_interval_seconds: {
            type: "integer",
            default: 30,
            minimum: 5,
            description: "Location update frequency (in seconds)"
          },
          enable_background: {
            type: "boolean",
            default: false,
            description: "Track location in background"
          }
        }
      },
      config_example: {
        accuracy: "high",
        update_interval_seconds: 30,
        enable_background: false
      },
      supports_incremental: false,
      supports_full_refresh: false,
      default_cron_schedule: "*/5 * * * *"
    },
    {
      name: "microphone",
      display_name: "Microphone",
      description: "Audio levels, transcriptions, and recordings",
      table_name: "stream_ios_microphone",
      config_schema: {
        type: "object",
        properties: {
          enable_transcription: {
            type: "boolean",
            default: false,
            description: "Enable speech-to-text transcription"
          },
          store_audio: {
            type: "boolean",
            default: false,
            description: "Store raw audio files in MinIO"
          },
          sample_duration_seconds: {
            type: "integer",
            default: 5,
            minimum: 1,
            maximum: 60,
            description: "Duration of each audio sample"
          }
        }
      },
      config_example: {
        enable_transcription: true,
        store_audio: false,
        sample_duration_seconds: 5
      },
      supports_incremental: false,
      supports_full_refresh: false,
      default_cron_schedule: "*/5 * * * *"
    }
  ];
}

// Convert StreamDescriptor to StreamInfo
function toStreamInfo(descriptor: StreamDescriptor, isEnabled: boolean = true): StreamInfo {
  return {
    stream_name: descriptor.name,
    display_name: descriptor.display_name,
    description: descriptor.description,
    is_enabled: isEnabled,
    config: descriptor.config_example,
    supports_incremental: descriptor.supports_incremental,
    default_cron_schedule: descriptor.default_cron_schedule,
    table_name: descriptor.table_name,
    cron_schedule: descriptor.default_cron_schedule,
    last_sync_at: null,
    supports_full_refresh: descriptor.supports_full_refresh,
    config_schema: descriptor.config_schema,
    config_example: descriptor.config_example
  };
}

// CORS headers
function setCORS(res: VercelResponse) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  res.setHeader('Access-Control-Max-Age', '86400');
}

// In-memory store (note: will reset on cold starts)
const pairingCodes = new Map<string, { source_id: string, expires_at: Date }>();

export default async function handler(req: VercelRequest, res: VercelResponse) {
  setCORS(res);

  // Handle CORS preflight
  if (req.method === 'OPTIONS') {
    return res.status(204).end();
  }

  const path = req.url || '/';

  // Root endpoint
  if (path === '/' && req.method === 'GET') {
    return res.status(200).json({
      service: 'ariata-ios-test',
      version: '1.0.0',
      endpoints: [
        'POST /api/devices/pairing/initiate - Initiate device pairing (web/CLI)',
        'POST /api/devices/pairing/complete - Complete device pairing (iOS app)',
        'POST /api/devices/verify - Verify device token',
        'GET /health - Health check'
      ]
    });
  }

  // Health check
  if (path === '/health' && req.method === 'GET') {
    return res.status(200).json({
      status: 'ok',
      timestamp: new Date().toISOString()
    });
  }

  // Initiate pairing
  if (path === '/api/devices/pairing/initiate' && req.method === 'POST') {
    try {
      const body = req.body as InitiatePairingRequest;

      const sourceId = generateUUID();
      const code = generatePairingCode();
      const expiresAt = new Date(Date.now() + 10 * 60 * 1000);

      pairingCodes.set(code, { source_id: sourceId, expires_at: expiresAt });

      const response: InitiatePairingResponse = {
        source_id: sourceId,
        code: code,
        expires_at: expiresAt.toISOString()
      };

      console.log(`[INITIATE] Generated pairing code: ${code}`);
      return res.status(200).json(response);
    } catch (error) {
      return res.status(400).json({
        error: 'Invalid request body',
        message: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  }

  // Complete pairing
  if (path === '/api/devices/pairing/complete' && req.method === 'POST') {
    try {
      const body = req.body as CompletePairingRequest;

      const pairing = pairingCodes.get(body.code);
      if (!pairing) {
        return res.status(400).json({ error: 'Invalid or expired pairing code' });
      }

      if (pairing.expires_at < new Date()) {
        pairingCodes.delete(body.code);
        return res.status(400).json({ error: 'Pairing code expired' });
      }

      const token = generateDeviceToken();

      const response: CompletePairingResponse = {
        source_id: pairing.source_id,
        device_token: token,
        available_streams: getAvailableStreams()
      };

      console.log(`[COMPLETE] Device paired: ${body.device_info.device_name}`);
      pairingCodes.delete(body.code);

      return res.status(200).json(response);
    } catch (error) {
      return res.status(400).json({
        error: 'Invalid request body',
        message: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  }

  // Verify
  if (path === '/api/devices/verify' && req.method === 'POST') {
    try {
      const authHeader = req.headers.authorization;

      if (!authHeader || !authHeader.startsWith('Bearer ')) {
        return res.status(401).json({ error: 'Missing or invalid Authorization header' });
      }

      const token = authHeader.substring(7);
      const enabledStreams = getAvailableStreams().map(stream => toStreamInfo(stream, true));

      const response: VerifyResponse = {
        source_id: generateUUID(),
        configuration_complete: true,
        enabled_streams: enabledStreams
      };

      console.log(`[VERIFY] Token verified: ${token.substring(0, 10)}...`);
      return res.status(200).json(response);
    } catch (error) {
      return res.status(500).json({
        error: 'Verification failed',
        message: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  }

  // 404
  return res.status(404).json({ error: 'Not found' });
}
