# Ariata Mac CLI

A lightweight command-line tool for monitoring macOS application activity and sending it to the Ariata platform.

## Features

- 🎯 **Simple & Focused**: Tracks only essential app usage data (app name, bundle ID, focus events)
- 🔒 **Privacy First**: No window titles, URLs, or document paths are collected
- 💾 **Reliable**: Local SQLite queue ensures no data loss
- 🚀 **Lightweight**: Minimal CPU and memory usage
- 🔄 **Auto-sync**: Uploads data every 5 minutes
- 🛡️ **Secure**: Device token authentication
- 🍎 **Universal**: Works on both Intel and Apple Silicon Macs

## Installation

### Quick Install (Recommended)

Get your device token from the Ariata web UI, then run:

```bash
curl -sSL https://your-ariata-instance.com/api/setup/mac/YOUR_TOKEN | bash
```

### From GitHub Release

```bash
# Download latest release
curl -L -o ariata-mac.tar.gz https://github.com/ariata/ariata/releases/latest/download/ariata-mac-universal.tar.gz

# Extract and install
tar -xzf ariata-mac.tar.gz
sudo mv ariata-mac /usr/local/bin/
sudo chmod +x /usr/local/bin/ariata-mac

# Initialize with your token
ariata-mac init YOUR_TOKEN
```

### Building from Source

```bash
cd apps/mac
swift build -c release
cp .build/release/ariata-mac /usr/local/bin/
```

### Quick Start

1. **Generate a device token** in the Ariata web UI
2. **Initialize the CLI** with your token:
   ```bash
   ariata-mac init YOUR_TOKEN
   ```
3. **Start monitoring**:
   ```bash
   ariata-mac start  # Run in foreground
   # OR
   ariata-mac daemon # Install as background service
   ```

## Commands

### `init <token>`
Configure the CLI with a device token from the web UI.

```bash
ariata-mac init ABCD1234
```

### `start`
Start monitoring in the foreground. Press Ctrl+C to stop.

```bash
ariata-mac start
ariata-mac start --verbose  # Show debug output
```

### `daemon`
Install/update the LaunchAgent for background monitoring.

```bash
ariata-mac daemon
```

### `stop`
Stop the background daemon.

```bash
ariata-mac stop
```

### `status`
Show current configuration and queue statistics.

```bash
ariata-mac status
```

### `reset`
Reset configuration and clear all data.

```bash
ariata-mac reset
ariata-mac reset --force  # Skip confirmation
```

## How It Works

1. **Monitors** app focus changes using NSWorkspace notifications
2. **Stores** events in a local SQLite database (~/.ariata/activity.db)
3. **Uploads** batched events every 5 minutes to the Ariata API
4. **Cleans up** uploaded events after 24 hours

## Data Collected

- Application name
- Bundle identifier
- Event type (focus_gained, focus_lost, launch, quit)
- Timestamp

**Privacy First**: No window titles, URLs, or document paths are collected.

## File Locations

- **Config**: `~/.ariata/config.json`
- **Database**: `~/.ariata/activity.db`
- **Logs** (daemon mode): `~/.ariata/ariata-mac.log`
- **LaunchAgent**: `~/Library/LaunchAgents/com.ariata.mac.plist`

## Troubleshooting

### Not uploading data?
```bash
ariata-mac status  # Check pending events
```

### Daemon not running?
```bash
ariata-mac stop
ariata-mac daemon
```

### Need to reconfigure?
```bash
ariata-mac reset
ariata-mac init NEW_TOKEN
```

## Requirements

- macOS 13.0+ (Ventura)
- Swift 5.9+
- Network access to Ariata API

## Development

```bash
# Build debug version
swift build

# Run directly
swift run ariata-mac status

# Run tests (if available)
swift test
```

## Architecture

This is a complete rewrite of the original Mac app, simplified to ~500 lines of Swift:

- **No GUI** - Pure CLI tool
- **SQLite queue** - Resilient local storage
- **Fixed intervals** - Uploads every 5 minutes
- **Token auth** - Simple device pairing
- **Single binary** - No dependencies

See [requirements.md](requirements.md) for detailed specifications.