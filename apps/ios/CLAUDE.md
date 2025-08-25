# iOS App - Production Documentation

## Overview

The Ariata iOS app has a single purpose: **Reliable raw data collection to MinIO**.

### Architecture

- **Direct SQLite writes** - No in-memory buffers, SQLite is the single queue
- **Three data streams**: location, audio, healthkit
- **Single API endpoint**: `/api/ingest`
- **Batched uploads** - Groups SQLite entries by stream type (3 requests per sync)
- **5-minute sync intervals** with background support

### Core Principles

- Raw data only - no on-device processing
- Background resilience is paramount
- User privacy and control
- Simple, reliable architecture

## Background Resilience

The iOS app is designed to maintain continuous data collection even through system interruptions:

### Audio Interruption Handling

- **Phone Calls**: Automatically pauses and resumes recording after calls end
- **System Interruptions**: Handles Siri, alarms, and other audio interruptions
- **Recovery Mechanism**: Falls back to full restart if resume fails
- **Foreground Recovery**: Checks and restarts recording when app returns to foreground

### Background Execution Strategy

- **DispatchSourceTimer**: More reliable than Timer for background execution
- **Audio Session Configuration**: Uses `.mixWithOthers` for continuous background recording
- **Background Tasks**: Registered for `fetch` and `processing` modes
- **Keepalive Tasks**: Background task wrappers around critical operations

### Timer Synchronization

- **Aligned Intervals**: HealthKit (5 min) synced with upload timer (5 min)
- **Prevents Empty Uploads**: Ensures data is collected before sync attempts
- **Reliable Scheduling**: DispatchSourceTimer survives background transitions

## Data Collection Architecture

### Evolution and Design Decisions

The iOS app evolved from using in-memory buffers to a simpler, more reliable SQLite-based architecture:

1. **Original Design**: In-memory buffers → SQLite → Upload
2. **Current Design**: Direct SQLite writes → Batched Upload

### Key Architectural Changes

#### 1. Removed Signal Processing

- The iOS app collects **streams only** - no signal processing on device
- Signals are computed server-side in the data pipeline
- Removed all `signal_id`, `signalIds`, and `activatedSignals` fields from responses

#### 2. Direct SQLite Writes

- Each manager writes directly to SQLite after collecting data
- No intermediate in-memory buffers
- SQLite serves as the single, persistent buffer

#### 3. Batched Uploads by Stream Type

- Groups all pending SQLite entries by `stream_name`
- Combines data arrays before upload
- Reduces network requests by ~93% (45 → 3 requests per sync)

#### 4. Incremental HealthKit Sync

- Uses `HKAnchoredObjectQuery` instead of time-based queries
- Tracks what's been synced with persistent anchors
- Handles Apple Watch sync delays (10-15 minutes)
- No duplicate data - each sample synced exactly once

### Data Flow

```
1. Data Collection (every N seconds/minutes)
   ↓
2. Direct write to SQLite (no buffering)
   ↓
3. Every 5 minutes: Batch by stream type
   ↓
4. Upload batches (3 requests total)
   ↓
5. Mark SQLite entries as complete
```

### Benefits of This Architecture

1. **Simplicity**: Single buffer (SQLite), no complex state management
2. **Reliability**: Data persisted immediately, survives app crashes
3. **Efficiency**: Batching reduces network overhead significantly
4. **Accuracy**: Incremental sync prevents data loss or duplication
5. **Transparency**: UI shows actual queue counts, not buffer sizes

## Production Requirements

- **iOS Version**: 18.0+
- **Storage**: ~500MB available (handles 7-day buffer)
- **Network**: Handles offline/online transitions
- **Battery Impact**: ~10-15% additional drain per day
- **Data Usage**: ~50-100MB/day typical

## Onboarding Flow

The app blocks all data collection until onboarding completes.

### Step 1: Endpoint Configuration

1. Enter API endpoint URL
2. Enter API key (device token)
3. Verify connection to backend
4. **Blocks progression until verified**

### Step 2: Permissions

Request ALL permissions (none are optional):

- **Location Services** - Always (not "While Using App")  
- **Microphone** - For audio recording
- **HealthKit** - All types listed below

If any permission denied:

- Show explanation why it's required
- Button to open Settings app
- Re-check on app return

### Step 3: Initial Sync

1. Capture last 7 days of HealthKit data
   - **Note**: Due to iOS 30-second background limits, large syncs may require multiple attempts
   - Progress indicator shows upload status
   - App must remain in foreground for initial sync to complete reliably
2. Start all background services
3. Begin regular collection

## Data Streams

### 1. Location Stream (`apple_ios_core_location`)

**Collection**: Every 10 seconds (written directly to SQLite)

**Data captured**:

```json
{
  "timestamp": "2025-01-30T10:00:00.000Z",
  "latitude": 37.7749,
  "longitude": -122.4194,
  "altitude": 10.5,
  "speed": 1.2,
  "horizontal_accuracy": 5.0,
  "vertical_accuracy": 3.0
}
```

**Configuration**:

```swift
locationManager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
locationManager.allowsBackgroundLocationUpdates = true
locationManager.pausesLocationUpdatesAutomatically = false
```

### 2. Audio Stream (`apple_ios_mic_audio`)

**Collection**: 30-second chunks with 2-second overlap (written directly to SQLite)

**Data format**:

```json
{
  "id": "unique-chunk-id",
  "timestamp_start": "2025-01-30T10:00:00.000Z",
  "timestamp_end": "2025-01-30T10:00:30.000Z",
  "duration": 30.0,
  "audio_data": "base64-encoded-aac-audio",
  "overlap_duration": 2.0
}
```

**Configuration**:

- Sample rate: 16kHz
- Format: AAC compression (16kbps bitrate)
- Compression: ~120KB per 30-second chunk
- Background recording enabled
- Audio route: User selectable (iPhone mic, Bluetooth devices, wired headsets)
- Timer: DispatchSourceTimer for reliable background execution
- Session options: `.mixWithOthers` for continuous recording

**Audio Input Selection**:

The app allows users to select their preferred audio input device:

- **iPhone Microphone**: Uses built-in mic only, prevents Bluetooth device interference
- **Bluetooth Devices**: AirPods, headsets, car audio (when connected)
- **Wired Headsets**: Lightning/USB-C headphones with microphone

To change audio input:

1. Go to Settings > Audio Input
2. Select preferred microphone from the list
3. Selection persists across app restarts
4. Automatically falls back to iPhone mic if selected device disconnects

**Interruption Handling**:

The app automatically handles audio interruptions:

- **Phone Calls**: Recording pauses when call begins, resumes when call ends
- **Siri/Alarms**: Handles system audio interruptions gracefully
- **Recovery**: If resume fails, recording restarts completely
- **Foreground Check**: Recording state verified and restored on app foreground
- **Notification**: Logs interruption events for debugging

### 3. HealthKit Stream (`apple_ios_healthkit`)

**Collection**: Every 5 minutes using incremental sync (aligned with upload timer)

**Incremental Sync Details**:

- Uses `HKAnchoredObjectQuery` to track only NEW samples
- Queries samples by when they were ADDED to HealthKit, not measurement time
- Stores unique anchor for each health type
- Handles Apple Watch delayed syncs (10-15 minute delays)
- No duplicates - each sample synced exactly once

**HealthKit Types**:

- `HKQuantityTypeIdentifierHeartRate`
- `HKQuantityTypeIdentifierStepCount`
- `HKQuantityTypeIdentifierActiveEnergyBurned`
- `HKQuantityTypeIdentifierHeartRateVariabilitySDNN`
- `HKQuantityTypeIdentifierDistanceWalkingRunning`
- `HKQuantityTypeIdentifierRestingHeartRate`
- `HKCategoryTypeIdentifierSleepAnalysis`

**Data format**:

```json
{
  "timestamp": "2025-01-30T10:00:00.000Z",
  "sample_type": "HKQuantityTypeIdentifierHeartRate",
  "value": 72.0,
  "unit": "bpm"
}
```

**Value Normalization**:

Normalize values before upload to avoid excessive precision:

- **Heart Rate**: Round to whole number (72 bpm)
- **Steps**: Always whole number (1234 steps)
- **Distance**: 2 decimal places (1234.57 m)
- **Active Energy**: 1 decimal place (45.7 kcal)
- **HRV**: 1 decimal place (28.5 ms)
- **Sleep**: Raw category value (0, 1, 2)

## Upload & Sync

### Sync Strategy

- **Primary**: 5-minute timer (foreground and background)
- **Fallback**: iOS background tasks
- **Manual**: User-triggered sync button

### Payload Structure

All uploads to `/api/ingest` are batched by stream type:

```json
{
  "stream_name": "apple_ios_core_location",
  "device_id": "uuid",
  "data": [
    // 30 location samples from 5 minutes
    {"timestamp": "...", "latitude": 37.7749, "longitude": -122.4194, ...},
    {"timestamp": "...", "latitude": 37.7750, "longitude": -122.4195, ...},
    // ... 28 more samples
  ],
  "batch_metadata": {
    "total_records": 30,
    "app_version": "1.0"
  }
}
```

**Batching Strategy**:

- Groups all pending SQLite entries by stream type
- Combines data arrays before upload
- Results in 3 POST requests per sync (one per stream)
- ~93% reduction in network requests

### Network Resilience

- **Timeouts**: 30 seconds per request
- **Retries**: Exponential backoff: 30s → 60s → 120s → 240s → 300s
- **Batch size**: Unlimited (backend handles chunking)
- **Auth**: `X-Device-Token` header on all requests

## Sync Monitoring

The app tracks sync health to help diagnose issues:

### Success Tracking

- **Last Upload Attempt**: Timestamp of most recent sync attempt
- **Last Successful Sync**: Only updated after confirmed uploads
- **Stream-Level Success**: Each stream upload tracked independently
- **Batch Return Values**: Upload functions return success boolean

### Monitoring Properties

- `lastUploadDate`: Most recent sync attempt (successful or not)
- `lastSuccessfulSyncDate`: Last time data was actually uploaded
- `uploadStats`: Pending, failed, and total counts
- `streamCounts`: Per-stream queue counts (healthkit, location, audio)

### Success Validation

1. Each stream batch upload returns true/false
2. Only marks `lastSuccessfulSyncDate` if any uploads succeeded
3. Failed uploads increment retry counters
4. Success logs include data size and stream key

## Error Handling

### Error Codes

- `E001` - Network timeout
- `E002` - Invalid API key
- `E003` - Server error (5xx)
- `E004` - Storage full
- `E005` - Permission denied

### SQLite Retry Logic

- Max attempts: 5 per record
- Tracks `upload_attempts` and `last_attempt_date`
- Failed records retained for 3 days

### Storage Management

- Auto-cleanup: Uploaded data after 3 days
- Critical threshold: < 100MB available
- Priority: Keep most recent data
- User notification when < 50MB

## Quick Reference

| Setting | Value | Purpose |
|---------|-------|---------|
| Location interval | 10 seconds | GPS sampling rate |
| Location accuracy | kCLLocationAccuracyNearestTenMeters | Battery vs accuracy balance |
| Audio chunk size | 30 seconds | Transcription segments |
| Audio overlap | 2 seconds | Prevent word cutoff |
| Audio sample rate | 16 kHz | Optimal for speech |
| Audio format | AAC 16kbps | ~120KB per chunk |
| HealthKit interval | 5 minutes | Incremental sync with anchors |
| Sync interval | 5 minutes | Upload frequency |
| Batch uploads | 3 requests | One per stream type |
| Network timeout | 30 seconds | Request timeout |
| Max retries | 5 | Upload attempts |
| Backoff max | 300 seconds | Network retry ceiling |
| Data retention | 3 days | All local data cleanup |
| Storage warning | 100 MB | Critical storage threshold |
| Background limit | 30 seconds | iOS background task limit |

## Troubleshooting

### HealthKit Not Syncing

1. Check Settings > Privacy > Health > Ariata
2. All categories must show checkmarks
3. Force quit and restart app
4. Check for iOS health database corruption
5. **Apple Watch sync delay**: Data syncs every 10-15 minutes
6. **Measurement vs sync time**: Data timestamps reflect when measured, not when synced
7. **Reset anchors**: Delete and reinstall app for fresh incremental sync

### Audio Not Recording

1. Check Settings > Privacy > Microphone > Ariata
2. Verify no other app is using microphone
3. Check audio input device in Settings > Audio Input
4. Select "iPhone Microphone" to prevent Bluetooth interference
5. Reset audio session by toggling recording off/on

### Location Gaps

1. Ensure "Always" permission (not "While Using")
2. Check Settings > General > Background App Refresh
3. Disable Low Power Mode
4. Check for location services system toggle

### Upload Failures

1. Verify network connectivity
2. Check API endpoint URL format
3. Validate API key in settings
4. Monitor SQLite retry count
5. Check available storage

### Battery Drain

1. Normal: 10-15% additional per day
2. High drain: Check location accuracy setting
3. Consider disabling audio if not needed
4. Reduce HealthKit collection frequency

### Audio Stops After Phone Call

1. Check logs for "Audio interruption began/ended" messages
2. Verify AVAudioSession interruption handling is working
3. Force restart recording: Settings > toggle Audio Recording off/on
4. Check if other apps are claiming exclusive audio
5. Restart the app to reset audio session

### Long Gaps in Data

1. Compare `lastSuccessfulSyncDate` to current time
2. Check if gap is exactly 2+ hours (background execution limit)
3. Verify background modes are enabled in iOS Settings
4. Check for iOS Low Power Mode (disables background refresh)
5. Look for timer cancellation in logs
6. Ensure app wasn't force-quit (prevents background execution)

## iOS Background Limits

### The 30-Second Constraint

iOS allows background tasks only 30 seconds to complete before forcefully terminating them. This affects:

- **Initial sync**: 7 days of HealthKit data may contain thousands of records
- **Large uploads**: Audio chunks or accumulated offline data
- **Poor network**: Slow connections may not complete within time limit

### How It's Handled

- **Chunked uploads**: Large datasets automatically split into smaller batches
- **Resume capability**: Failed uploads retry on next sync cycle
- **Progress tracking**: SQLite tracks partial upload progress
- **Foreground priority**: Initial sync recommends keeping app open

## Background Modes

Required in Info.plist:

- `location` - Location updates
- `audio` - Audio recording  
- `fetch` - Background fetch
- `processing` - Background tasks

Background task identifiers:

- `com.ariata.ios.refresh`
- `com.ariata.ios.processing`
- `com.ariata.ios.sync`
