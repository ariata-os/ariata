#!/bin/bash
# Ariata Mac Monitor Interactive Installer
# This script provides a user-friendly installation experience with native macOS dialogs
set -e

# Colors for terminal output (when run from command line)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
USE_LOCAL_BUILD="false"
while [[ $# -gt 0 ]]; do
    case $1 in
        --local)
            USE_LOCAL_BUILD="true"
            shift
            ;;
        --token)
            CLI_TOKEN="$2"
            shift 2
            ;;
        --endpoint)
            CLI_ENDPOINT="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--local] [--token TOKEN] [--endpoint URL]"
            echo ""
            echo "Options:"
            echo "  --local           Use local build instead of downloading"
            echo "  --token TOKEN     Device token from Ariata web UI"
            echo "  --endpoint URL    Ariata server URL (e.g., https://ariata.com)"
            echo "  --help            Show this help message"
            echo ""
            echo "If not provided, you will be prompted for these values."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Default values (can be overridden by environment variables or CLI args)
DEFAULT_ENDPOINT="${CLI_ENDPOINT:-${ARIATA_ENDPOINT:-http://localhost:3000}}"
DEFAULT_TOKEN="${CLI_TOKEN:-${ARIATA_TOKEN:-}}"

# Function to show error and exit
show_error() {
    local message="$1"
    osascript -e "display alert \"Installation Failed\" message \"$message\" as critical"
    echo -e "${RED}Error: $message${NC}"
    exit 1
}

# Function to check if running in Terminal
is_terminal() {
    [ -t 0 ] && [ -t 1 ]
}

# Show welcome dialog
echo "Starting Ariata Mac Monitor installation..."

if ! osascript -e 'display dialog "Welcome to Ariata Mac Setup

This will install and configure Ariata to monitor your Mac activity." buttons {"Cancel", "Continue"} default button "Continue" with title "Ariata Mac Setup" with icon note' >/dev/null 2>&1; then
    echo "Installation cancelled by user"
    exit 0
fi

# Step 1: Get the API endpoint
echo "Step 1: Getting API endpoint..."
ENDPOINT=$(osascript -e "text returned of (display dialog \"Enter your server URL:\" default answer \"$DEFAULT_ENDPOINT\" with title \"Ariata Setup - Step 1 of 4\" with icon note)" 2>/dev/null || echo "")

if [ -z "$ENDPOINT" ]; then
    show_error "No endpoint provided. Setup cancelled."
fi

# Validate endpoint format
if ! [[ "$ENDPOINT" =~ ^https?:// ]]; then
    show_error "Invalid endpoint. Must start with http:// or https://"
fi

echo -e "${GREEN}✓${NC} Endpoint: $ENDPOINT"

# Step 2: Get the device token
echo "Step 2: Getting device token..."
TOKEN_DEFAULT="$DEFAULT_TOKEN"
TOKEN=$(osascript -e "text returned of (display dialog \"Enter your device token (8 characters):\" default answer \"$TOKEN_DEFAULT\" with title \"Ariata Setup - Step 2 of 4\" with icon note)" 2>/dev/null || echo "")

if [ -z "$TOKEN" ]; then
    show_error "No token provided. Setup cancelled."
fi

# Convert token to uppercase
TOKEN=$(echo "$TOKEN" | tr '[:lower:]' '[:upper:]')

echo -e "${GREEN}✓${NC} Token: ${TOKEN:0:4}****"

# Step 3: Check for existing installations
echo "Step 3: Checking for existing installations..."

# Check if ariata-mac is already running
if pgrep -f "ariata-mac" > /dev/null 2>&1; then
    echo "Found existing ariata-mac process(es)..."
    
    # Ask user if they want to stop existing processes
    if osascript -e 'button returned of (display dialog "Ariata Mac Monitor is already running.

This will stop the existing monitor and install a fresh version with your new configuration.

Continue?" buttons {"Cancel", "Continue"} default button "Continue" with title "Existing Installation Found" with icon caution)' 2>/dev/null | grep -q "Continue"; then
        
        echo "Stopping existing processes..."
        
        # Stop the LaunchAgent if it exists
        if launchctl list | grep -q "com.ariata.mac" 2>/dev/null; then
            echo "  Unloading LaunchAgent..."
            launchctl unload ~/Library/LaunchAgents/com.ariata.mac.plist 2>/dev/null || true
        fi
        
        # Kill any running processes (including development builds)
        echo "  Stopping all ariata-mac processes..."
        pkill -f "ariata-mac" 2>/dev/null || true
        
        # Small delay to ensure processes are stopped
        sleep 2
        
        echo -e "${GREEN}✓${NC} Existing installation stopped"
    else
        echo "Installation cancelled by user"
        exit 0
    fi
fi

# Step 4: Download and install
echo "Step 4: Preparing Ariata Mac Monitor..."

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

if [ "$USE_LOCAL_BUILD" = "true" ]; then
    echo "Using local build..."
    
    # Find the local build
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    LOCAL_BUILD="$SCRIPT_DIR/../.build/release/ariata-mac"
    
    if [ ! -f "$LOCAL_BUILD" ]; then
        show_error "Local build not found at: $LOCAL_BUILD
        
Please run 'make mac-build' or 'swift build -c release' first."
    fi
    
    # Copy to temp location (mimics download behavior)
    cp "$LOCAL_BUILD" "$TEMP_DIR/ariata-mac"
    chmod +x "$TEMP_DIR/ariata-mac"
    
    echo -e "${GREEN}✓${NC} Using local build from: $LOCAL_BUILD"
else
    osascript -e 'display notification "Downloading Ariata Mac Monitor..." with title "Installing" subtitle "This may take a moment"'
    
    # Download the binary from mac-latest release (has unversioned files)
    DOWNLOAD_URL="https://github.com/ariata-com/ariata/releases/download/mac-latest/ariata-mac-universal.tar.gz"
    echo "Downloading from: $DOWNLOAD_URL"
    
    if ! curl -L --progress-bar -o "$TEMP_DIR/ariata-mac.tar.gz" "$DOWNLOAD_URL"; then
        show_error "Failed to download Ariata Mac Monitor. Please check your internet connection."
    fi
    
    # Extract
    echo "Extracting..."
    if ! tar -xzf "$TEMP_DIR/ariata-mac.tar.gz" -C "$TEMP_DIR"; then
        show_error "Failed to extract the downloaded file."
    fi
    
    # Check if binary exists
    if [ ! -f "$TEMP_DIR/ariata-mac" ]; then
        show_error "Downloaded file does not contain ariata-mac binary."
    fi
    
    # Make it executable
    chmod +x "$TEMP_DIR/ariata-mac"
fi

# Request admin permissions for installation
osascript -e 'display dialog "Admin permission required to install" buttons {"Cancel", "Continue"} default button "Continue" with title "Permission Required" with icon caution' >/dev/null 2>&1 || show_error "Installation cancelled."

# Install to /usr/local/bin
echo "Installing to /usr/local/bin (requires admin password)..."
if ! sudo mkdir -p /usr/local/bin; then
    show_error "Failed to create /usr/local/bin directory"
fi

if ! sudo mv "$TEMP_DIR/ariata-mac" /usr/local/bin/ariata-mac; then
    show_error "Failed to install ariata-mac. Please check permissions."
fi

echo -e "${GREEN}✓${NC} Installed to /usr/local/bin/ariata-mac"

# Step 5: Configure
echo "Step 5: Configuring..."
osascript -e 'display notification "Configuring Ariata..." with title "Setup" subtitle "Validating token"'

# Initialize with token and endpoint
export ARIATA_API_URL="$ENDPOINT"

echo "Validating token with server..."
if ! /usr/local/bin/ariata-mac init "$TOKEN" 2>&1; then
    # Try to provide more specific error message
    if ! curl -s "$ENDPOINT/api/health" >/dev/null 2>&1; then
        show_error "Cannot connect to $ENDPOINT. Please check the URL and try again."
    else
        show_error "Invalid token or configuration failed. Please check your token: $TOKEN"
    fi
fi

echo -e "${GREEN}✓${NC} Configuration validated"

# Step 6: Set up daemon
echo "Step 6: Setting up background service..."
osascript -e 'display notification "Setting up background service..." with title "Setup" subtitle "Installing daemon"'

if ! /usr/local/bin/ariata-mac daemon 2>&1; then
    echo -e "${YELLOW}Warning: Could not install background service${NC}"
    # Don't fail here, user can set it up manually
fi

# Step 7: Setup Full Disk Access for all data streams
echo "Step 7: Setting up Full Disk Access..."

# Show dialog before opening settings
osascript -e 'display dialog "Grant Full Disk Access to enable all data streams.

System Settings will open to the Full Disk Access page." buttons {"Continue"} default button "Continue" with title "Setup: Full Disk Access" with icon note' 2>/dev/null

# Open System Settings
echo "Opening System Settings..."
open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"

# Wait a moment for settings to open
sleep 2

# Show instruction dialog after settings is open
osascript -e 'display dialog "In System Settings:

1. Click + button
2. Navigate to /usr/local/bin/
3. Select ariata-mac
4. Ensure toggle is ON" buttons {"Done"} default button "Done" with title "Grant Full Disk Access" with icon note'

echo -e "${GREEN}✓${NC} Full Disk Access configuration complete"
echo "   Run 'ariata-mac status' to verify access"

# Success!
osascript -e 'display notification "Setup complete! Ariata is now monitoring." with title "Success" subtitle "Installation finished" sound name "Glass"'

# Final success dialog with status
STATUS=$(/usr/local/bin/ariata-mac status 2>/dev/null || echo "Status check failed")

osascript -e "display dialog \"✅ Installation Complete!

Ariata is now monitoring your Mac.

Verify setup: ariata-mac status
Uninstall: ariata-mac reset\" buttons {\"Done\"} default button \"Done\" with title \"Ariata Setup Complete\" with icon note"

echo -e "${GREEN}✅ Installation complete!${NC}"
echo ""
echo "Run 'ariata-mac status' to verify setup"