/**
 * iOS Test Endpoint Server
 * Simple Bun server for testing iOS device pairing - matches Rust backend API exactly
 */

interface DeviceInfo {
  device_id: string;
  device_name: string;
  device_model: string;
  os_version: string;
  app_version?: string;
}

// Initiate pairing request (web/CLI initiated)
interface InitiatePairingRequest {
  device_type: string;
  name?: string;
}

// Initiate pairing response
interface InitiatePairingResponse {
  source_id: string;
  code: string;
  expires_at: string;
}

// Complete pairing request (iOS app uses this)
interface CompletePairingRequest {
  code: string;
  device_info: DeviceInfo;
}

// Complete pairing response
interface CompletePairingResponse {
  source_id: string;
  device_token: string;
  available_streams: StreamDescriptor[];
}

// Stream descriptor (used in pairing response)
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

// Stream info (used in verify response - more detailed)
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

// Verify response
interface VerifyResponse {
  source_id: string;
  configuration_complete: boolean;
  enabled_streams: StreamInfo[];
}

// Generate random pairing code (6 chars alphanumeric)
function generatePairingCode(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return code;
}

// Generate device token (base64 encoded random bytes)
function generateDeviceToken(): string {
  const bytes = new Uint8Array(32); // 256 bits
  crypto.getRandomValues(bytes);
  return btoa(String.fromCharCode(...bytes));
}

// Generate UUID v4
function generateUUID(): string {
  return crypto.randomUUID();
}

// Get available iOS streams (matching Rust implementation exactly)
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

// Convert StreamDescriptor to StreamInfo for verify response
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

// Handle CORS
function corsHeaders(origin?: string): HeadersInit {
  return {
    'Access-Control-Allow-Origin': origin || '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Access-Control-Max-Age': '86400',
  };
}

// In-memory store for test pairing codes (cleared on restart)
const pairingCodes = new Map<string, { source_id: string, expires_at: Date }>();

const server = Bun.serve({
  port: process.env.PORT || 3000,

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const origin = req.headers.get('origin');

    // Handle CORS preflight
    if (req.method === 'OPTIONS') {
      return new Response(null, {
        status: 204,
        headers: corsHeaders(origin || undefined)
      });
    }

    // Root endpoint
    if (url.pathname === '/' && req.method === 'GET') {
      return new Response(
        JSON.stringify({
          service: 'ariata-ios-test',
          version: '1.0.0',
          endpoints: [
            'POST /api/devices/pairing/initiate - Initiate device pairing (web/CLI)',
            'POST /api/devices/pairing/complete - Complete device pairing (iOS app)',
            'POST /api/devices/verify - Verify device token',
            'GET /health - Health check'
          ]
        }),
        {
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders(origin || undefined)
          }
        }
      );
    }

    // Health check
    if (url.pathname === '/health' && req.method === 'GET') {
      return new Response(
        JSON.stringify({ status: 'ok', timestamp: new Date().toISOString() }),
        {
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders(origin || undefined)
          }
        }
      );
    }

    // Initiate pairing endpoint (web/CLI uses this to generate code)
    if (url.pathname === '/api/devices/pairing/initiate' && req.method === 'POST') {
      try {
        const body = await req.json() as InitiatePairingRequest;

        const sourceId = generateUUID();
        const code = generatePairingCode();
        const expiresAt = new Date(Date.now() + 10 * 60 * 1000); // 10 minutes

        // Store the code for later validation
        pairingCodes.set(code, { source_id: sourceId, expires_at: expiresAt });

        const response: InitiatePairingResponse = {
          source_id: sourceId,
          code: code,
          expires_at: expiresAt.toISOString()
        };

        console.log(`[INITIATE] Generated pairing code: ${code} for ${body.device_type}`);

        return new Response(JSON.stringify(response), {
          status: 200,
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders(origin || undefined)
          }
        });
      } catch (error) {
        return new Response(
          JSON.stringify({
            error: 'Invalid request body',
            message: error instanceof Error ? error.message : 'Unknown error'
          }),
          {
            status: 400,
            headers: {
              'Content-Type': 'application/json',
              ...corsHeaders(origin || undefined)
            }
          }
        );
      }
    }

    // Complete pairing endpoint (iOS app uses this with code + device_info)
    if (url.pathname === '/api/devices/pairing/complete' && req.method === 'POST') {
      try {
        const body = await req.json() as CompletePairingRequest;

        // Validate code exists and hasn't expired
        const pairing = pairingCodes.get(body.code);
        if (!pairing) {
          return new Response(
            JSON.stringify({ error: 'Invalid or expired pairing code' }),
            {
              status: 400,
              headers: {
                'Content-Type': 'application/json',
                ...corsHeaders(origin || undefined)
              }
            }
          );
        }

        if (pairing.expires_at < new Date()) {
          pairingCodes.delete(body.code);
          return new Response(
            JSON.stringify({ error: 'Pairing code expired' }),
            {
              status: 400,
              headers: {
                'Content-Type': 'application/json',
                ...corsHeaders(origin || undefined)
              }
            }
          );
        }

        // Generate device token
        const token = generateDeviceToken();

        const response: CompletePairingResponse = {
          source_id: pairing.source_id,
          device_token: token,
          available_streams: getAvailableStreams()
        };

        console.log(`[COMPLETE] Device paired: ${body.device_info.device_name} (${body.device_info.device_id})`);

        // Clean up used code
        pairingCodes.delete(body.code);

        return new Response(JSON.stringify(response), {
          status: 200,
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders(origin || undefined)
          }
        });
      } catch (error) {
        return new Response(
          JSON.stringify({
            error: 'Invalid request body',
            message: error instanceof Error ? error.message : 'Unknown error'
          }),
          {
            status: 400,
            headers: {
              'Content-Type': 'application/json',
              ...corsHeaders(origin || undefined)
            }
          }
        );
      }
    }

    // Verify endpoint (iOS app uses this to verify token and get enabled streams)
    if (url.pathname === '/api/devices/verify' && req.method === 'POST') {
      try {
        const authHeader = req.headers.get('Authorization');

        if (!authHeader || !authHeader.startsWith('Bearer ')) {
          return new Response(
            JSON.stringify({ error: 'Missing or invalid Authorization header' }),
            {
              status: 401,
              headers: {
                'Content-Type': 'application/json',
                ...corsHeaders(origin || undefined)
              }
            }
          );
        }

        const token = authHeader.substring(7); // Remove 'Bearer '

        // In test mode, accept any token and return all streams as enabled
        const enabledStreams = getAvailableStreams().map(stream => toStreamInfo(stream, true));

        const response: VerifyResponse = {
          source_id: generateUUID(),
          configuration_complete: true,
          enabled_streams: enabledStreams
        };

        console.log(`[VERIFY] Token verified: ${token.substring(0, 10)}...`);

        return new Response(JSON.stringify(response), {
          status: 200,
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders(origin || undefined)
          }
        });
      } catch (error) {
        return new Response(
          JSON.stringify({
            error: 'Verification failed',
            message: error instanceof Error ? error.message : 'Unknown error'
          }),
          {
            status: 500,
            headers: {
              'Content-Type': 'application/json',
              ...corsHeaders(origin || undefined)
            }
          }
        );
      }
    }

    // 404 for unknown routes
    return new Response(
      JSON.stringify({ error: 'Not found' }),
      {
        status: 404,
        headers: {
          'Content-Type': 'application/json',
          ...corsHeaders(origin || undefined)
        }
      }
    );
  },
});

console.log(`🚀 iOS Test Server running at http://localhost:${server.port}`);
console.log(`📱 Endpoints (matching Rust backend):`);
console.log(`   POST http://localhost:${server.port}/api/devices/pairing/initiate`);
console.log(`   POST http://localhost:${server.port}/api/devices/pairing/complete`);
console.log(`   POST http://localhost:${server.port}/api/devices/verify`);
console.log(`   GET  http://localhost:${server.port}/health`);
