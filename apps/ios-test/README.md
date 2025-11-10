# iOS Test Endpoint

Simple Bun/TypeScript server for testing iOS device pairing with Ariata.

**IMPORTANT:** This test server matches the Rust backend API exactly. All routes, request/response structures, and field names are identical to production.

## Overview

This test endpoint simulates the device pairing flow for iOS devices, providing the same JSON response structure as the production Rust API but without database persistence. Perfect for iOS development and testing.

## Features

- **POST /api/devices/pairing/initiate** - Generate pairing code (web/CLI)
- **POST /api/devices/pairing/complete** - Complete pairing (iOS app)
- **POST /api/devices/verify** - Verify device tokens
- **GET /health** - Health check endpoint
- CORS enabled for web testing
- Matches production Rust API exactly

## Local Development

### Prerequisites

Install [Bun](https://bun.sh):

```bash
curl -fsSL https://bun.sh/install | bash
```

### Run Locally

```bash
cd apps/ios-test
bun install
bun run dev
```

Server runs on `http://localhost:3000`

## API Endpoints

### 1. Initiate Pairing (Web/CLI)

**Endpoint:** `POST /api/devices/pairing/initiate`

This endpoint is typically called by the web interface or CLI to generate a pairing code.

**Request:**
```json
{
  "device_type": "ios",
  "name": "Adam's iPhone"
}
```

**Response:**
```json
{
  "source_id": "uuid-v4",
  "code": "ABC123",
  "expires_at": "2025-11-10T12:10:00Z"
}
```

### 2. Complete Pairing (iOS App) ⭐ Main iOS Endpoint

**Endpoint:** `POST /api/devices/pairing/complete`

This is the primary endpoint the iOS app will call with the pairing code and device info.

**Request:**
```json
{
  "code": "ABC123",
  "device_info": {
    "device_id": "UUID-from-iOS",
    "device_name": "Adam's iPhone",
    "device_model": "iPhone16,1",
    "os_version": "iOS 17.2",
    "app_version": "1.0.0"
  }
}
```

**Response:**
```json
{
  "source_id": "uuid-v4",
  "device_token": "base64-encoded-256bit-token",
  "available_streams": [
    {
      "name": "healthkit",
      "display_name": "HealthKit",
      "description": "Health and fitness metrics including heart rate, steps, sleep, and workouts",
      "table_name": "stream_ios_healthkit",
      "config_schema": {
        "type": "object",
        "properties": {
          "enabled_metrics": {
            "type": "array",
            "items": { "type": "string" },
            "description": "List of metrics to collect: heart_rate, steps, sleep, workouts, etc."
          },
          "sampling_interval_seconds": {
            "type": "integer",
            "default": 60,
            "minimum": 10,
            "description": "How often to sample health metrics (in seconds)"
          }
        }
      },
      "config_example": {
        "enabled_metrics": ["heart_rate", "steps", "sleep"],
        "sampling_interval_seconds": 60
      },
      "supports_incremental": false,
      "supports_full_refresh": false,
      "default_cron_schedule": "*/5 * * * *"
    },
    {
      "name": "location",
      "display_name": "Location",
      "description": "GPS coordinates, speed, altitude, and activity type",
      "table_name": "stream_ios_location",
      "config_schema": {
        "type": "object",
        "properties": {
          "accuracy": {
            "type": "string",
            "enum": ["best", "high", "medium", "low"],
            "default": "high",
            "description": "GPS accuracy level"
          },
          "update_interval_seconds": {
            "type": "integer",
            "default": 30,
            "minimum": 5,
            "description": "Location update frequency (in seconds)"
          },
          "enable_background": {
            "type": "boolean",
            "default": false,
            "description": "Track location in background"
          }
        }
      },
      "config_example": {
        "accuracy": "high",
        "update_interval_seconds": 30,
        "enable_background": false
      },
      "supports_incremental": false,
      "supports_full_refresh": false,
      "default_cron_schedule": "*/5 * * * *"
    },
    {
      "name": "microphone",
      "display_name": "Microphone",
      "description": "Audio levels, transcriptions, and recordings",
      "table_name": "stream_ios_microphone",
      "config_schema": {
        "type": "object",
        "properties": {
          "enable_transcription": {
            "type": "boolean",
            "default": false,
            "description": "Enable speech-to-text transcription"
          },
          "store_audio": {
            "type": "boolean",
            "default": false,
            "description": "Store raw audio files in MinIO"
          },
          "sample_duration_seconds": {
            "type": "integer",
            "default": 5,
            "minimum": 1,
            "maximum": 60,
            "description": "Duration of each audio sample"
          }
        }
      },
      "config_example": {
        "enable_transcription": true,
        "store_audio": false,
        "sample_duration_seconds": 5
      },
      "supports_incremental": false,
      "supports_full_refresh": false,
      "default_cron_schedule": "*/5 * * * *"
    }
  ]
}
```

### 3. Verify Device Token

**Endpoint:** `POST /api/devices/verify`

The iOS app calls this to verify its token is valid and get the list of enabled streams.

**Headers:**
```
Authorization: Bearer <device_token>
```

**Response:**
```json
{
  "source_id": "uuid-v4",
  "configuration_complete": true,
  "enabled_streams": [
    {
      "stream_name": "healthkit",
      "display_name": "HealthKit",
      "description": "Health and fitness metrics including heart rate, steps, sleep, and workouts",
      "is_enabled": true,
      "config": {
        "enabled_metrics": ["heart_rate", "steps", "sleep"],
        "sampling_interval_seconds": 60
      },
      "supports_incremental": false,
      "default_cron_schedule": "*/5 * * * *",
      "table_name": "stream_ios_healthkit",
      "cron_schedule": "*/5 * * * *",
      "last_sync_at": null,
      "supports_full_refresh": false,
      "config_schema": { /* ... */ },
      "config_example": { /* ... */ }
    },
    {
      "stream_name": "location",
      "display_name": "Location",
      "description": "GPS coordinates, speed, altitude, and activity type",
      "is_enabled": true,
      "config": {
        "accuracy": "high",
        "update_interval_seconds": 30,
        "enable_background": false
      },
      "supports_incremental": false,
      "default_cron_schedule": "*/5 * * * *",
      "table_name": "stream_ios_location",
      "cron_schedule": "*/5 * * * *",
      "last_sync_at": null,
      "supports_full_refresh": false,
      "config_schema": { /* ... */ },
      "config_example": { /* ... */ }
    },
    {
      "stream_name": "microphone",
      "display_name": "Microphone",
      "description": "Audio levels, transcriptions, and recordings",
      "is_enabled": true,
      "config": {
        "enable_transcription": true,
        "store_audio": false,
        "sample_duration_seconds": 5
      },
      "supports_incremental": false,
      "default_cron_schedule": "*/5 * * * *",
      "table_name": "stream_ios_microphone",
      "cron_schedule": "*/5 * * * *",
      "last_sync_at": null,
      "supports_full_refresh": false,
      "config_schema": { /* ... */ },
      "config_example": { /* ... */ }
    }
  ]
}
```

### 4. Health Check

**Endpoint:** `GET /health`

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2025-11-10T12:00:00Z"
}
```

## Deployment Options

### Option 1: Fly.io (Recommended for Bun)

1. Install flyctl:
```bash
brew install flyctl
```

2. Login:
```bash
fly auth login
```

3. Launch app:
```bash
fly launch --name ariata-ios-test --region sjc
```

4. Deploy:
```bash
fly deploy
```

5. Set custom domain:
```bash
fly certs add test-ios.ariata.com
```

### Option 2: Railway

1. Install Railway CLI:
```bash
npm i -g @railway/cli
```

2. Login:
```bash
railway login
```

3. Deploy:
```bash
railway up
```

4. Add custom domain in Railway dashboard

### Option 3: Deno Deploy

Convert to Deno (change to Deno.serve):

```bash
deployctl deploy --project=ariata-ios-test index.ts
```

## Testing with curl

### Full pairing flow:

**Step 1: Initiate pairing (web/CLI would do this)**
```bash
curl -X POST http://localhost:3000/api/devices/pairing/initiate \
  -H "Content-Type: application/json" \
  -d '{
    "device_type": "ios",
    "name": "Test iPhone"
  }'
```

This returns a code like `ABC123`.

**Step 2: Complete pairing (iOS app does this)**
```bash
curl -X POST http://localhost:3000/api/devices/pairing/complete \
  -H "Content-Type: application/json" \
  -d '{
    "code": "ABC123",
    "device_info": {
      "device_id": "test-device-id",
      "device_name": "Test iPhone",
      "device_model": "iPhone16,1",
      "os_version": "iOS 17.2",
      "app_version": "1.0.0"
    }
  }'
```

This returns a `device_token`.

**Step 3: Verify token**
```bash
curl -X POST http://localhost:3000/api/devices/verify \
  -H "Authorization: Bearer <device_token_from_step_2>"
```

### Health check:
```bash
curl http://localhost:3000/health
```

## iOS App Integration

The iOS app should:

1. **Get pairing code** - User enters code from web/CLI
2. **Call complete endpoint** with code + device info
3. **Store device_token** securely in Keychain
4. **Call verify endpoint** on app launch to check if token is still valid
5. **Use available_streams** to show user what data can be collected

## Differences from Production

- ✅ All routes match exactly
- ✅ All request/response structures match exactly
- ✅ All field names and types match exactly
- ❌ Data is not persisted (stored in memory only)
- ❌ Tokens are not encrypted (test mode)
- ❌ All tokens are accepted as valid (no real validation)
- ❌ Pairing codes reset on server restart

## Architecture Alignment

This endpoint mirrors the Rust backend exactly:
- [core/src/api/device_pairing.rs](../../core/src/api/device_pairing.rs) - Pairing logic
- [core/src/sources/ios/](../../core/src/sources/ios/) - Stream implementations
- [core/src/server/api.rs](../../core/src/server/api.rs) - API routes (line 97: complete pairing, line 693: verify)

All stream descriptors, config schemas, and field names match the production implementation.
