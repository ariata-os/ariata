# Ariata iOS App

Native iOS application for continuous data collection including location, HealthKit metrics, and audio recording.

## Architecture Overview

The iOS app collects three primary data streams:
- **Location** (10-second GPS samples with course, speed, altitude, floor)
- **HealthKit** (heart rate, steps, activity, sleep, etc.)
- **Audio** (30-second chunks for transcription)

### Background Execution Strategy

The app uses iOS background modes to maintain continuous health tracking. Each stream operates independently with its own background mode permission.

#### Location Background Mode

Location tracking uses the standard iOS location background mode:
- **Background mode:** `UIBackgroundModes: ["location"]`
- **How it works:** Calls `startUpdatingLocation()` to receive continuous location updates
- **Sampling:** `DispatchSourceTimer` fires every 10 seconds for high-frequency sampling
- **Fallback:** `startMonitoringSignificantLocationChanges()` for additional reliability
- **Health checks:** 30-second intervals verify tracking remains active

#### Audio Background Mode

Audio recording operates independently using the audio background mode:
- **Background mode:** `UIBackgroundModes: ["audio"]`
- **How it works:** Active audio recording session keeps app alive
- **Sampling:** Continuous 30-second chunks for transcription
- **Health checks:** 30-second intervals detect and recover from interruptions
- **Interruptions:** Handles phone calls and audio session interruptions automatically

#### Independent Operation

Both streams are fully independent:
- ✅ Users can enable location without audio
- ✅ Users can enable audio without location
- ✅ Each has its own background mode and permissions
- ✅ Both include automatic recovery via health checks
- ✅ Configured independently from the web app

### Key Implementation Details

#### Audio Manager (apps/ios/Ariata/Managers/Tracking/AudioManager.swift)
- Uses `DispatchSourceTimer` for reliable 30-second chunks
- Health check every 30 seconds to detect and recover from interruptions
- Handles audio session interruptions (phone calls, etc.)
- Saves chunks to SQLite queue for batched upload

#### Location Manager (apps/ios/Ariata/Managers/Tracking/LocationManager.swift)
- Uses `DispatchSourceTimer` (not `Timer`) for background reliability
- Samples location every 10 seconds (configurable via `samplingIntervalSeconds`)
- Tracks: latitude, longitude, altitude, speed, course (direction), floor level
- Health check every 30 seconds verifies:
  - Timer is still running
  - Location data is fresh (< 60s old)
  - Tracking state matches configuration
- Implements `CLLocationManagerDelegate` pause/resume methods for iOS lifecycle events

#### Background Mode Design

**iOS Background Modes Used:**
- `location` - Continuous location updates for health context
- `audio` - Audio recording for environmental health analysis
- `processing` - Batch uploads via BGProcessingTask

**Why This Approach:**
- Standard iOS background modes approved for health/fitness apps
- Similar to apps like Strava (location) and voice memo apps (audio)
- Each stream operates independently with proper iOS APIs
- No workarounds or hacks - follows Apple's guidelines

### Network Changes (WiFi ↔ 5G)

Location and audio tracking work reliably across network transitions:
- `DispatchSourceTimer` runs on dedicated queue (not RunLoop-dependent)
- Health checks detect and recover from any issues
- Network changes have no impact on tracking
- Works equally well on WiFi, cellular, or offline

### Testing the Background Tracking

To verify continuous tracking:

1. Enable location and/or audio streams in the web app
2. Start the app and grant necessary permissions
3. Put app in background
4. Switch from WiFi to cellular (or vice versa)
5. Leave for 30+ minutes
6. Check database - should see continuous samples for enabled streams
7. Test interruptions (phone call) - both streams auto-resume afterward

Expected data collection rates:
- **Location:** Every 10 seconds (~360 samples/hour)
- **Audio:** 30-second chunks (120 chunks/hour, ~2.8 MB/hour at 16kbps)
- **HealthKit:** Event-driven (varies by metric type)

### Battery Impact

This is a high-fidelity health tracking app designed for comprehensive health insights:
- **Location only:** Moderate battery drain (~10-15% per hour)
- **Audio only:** Higher battery drain (~15-20% per hour)
- **Both streams:** Combined drain (~20-30% per hour)
- **Expected battery life:** 4-8 hours depending on streams enabled
- **Recommendation:** Use with power bank for all-day tracking

Users can disable streams they don't need to optimize battery life.

## Project Structure

```
Ariata/
├── Managers/
│   ├── DeviceManager.swift           # Device config and sync
│   ├── Data/
│   │   ├── NetworkManager.swift      # API communication
│   │   └── SQLiteManager.swift       # Local queue for offline support
│   ├── Integration/
│   │   └── HealthKitManager.swift    # HealthKit data collection
│   ├── Tracking/
│   │   ├── AudioManager.swift        # Audio recording
│   │   └── LocationManager.swift     # Location tracking
│   └── Sync/
│       └── BatchUploadCoordinator.swift  # 5-minute batch uploads
├── Models/
│   ├── AudioStreamData.swift         # Audio chunk format
│   ├── CoreLocationStreamData.swift  # Location sample format
│   ├── HealthKitStreamData.swift     # HealthKit metric format
│   └── DeviceConfiguration.swift     # Server-driven config
├── Views/
│   ├── MainView.swift                # Primary app interface
│   ├── SettingsView.swift            # Configuration
│   └── Onboarding/
│       └── OnboardingView.swift      # Initial setup flow
├── AriataApp.swift                   # App lifecycle and background tasks
└── Info.plist                        # Background modes configuration
```

## Key Features

- **Offline-first architecture** - All data queued in SQLite, uploaded every 5 minutes
- **Server-driven configuration** - Web app controls which streams are enabled
- **Automatic recovery** - Health checks detect and fix tracking issues
- **Battery-conscious** - Despite continuous tracking, optimized for efficiency
- **Privacy-focused** - All data stored locally until uploaded to your own server