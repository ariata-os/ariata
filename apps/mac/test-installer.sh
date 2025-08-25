#!/bin/bash
# Test installer for local development of Ariata Mac Monitor with iMessage support
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Use local build path
LOCAL_BUILD="$(cd "$(dirname "$0")/.." && pwd)/.build/release/ariata-mac"

# Default to localhost for testing
DEFAULT_ENDPOINT="http://localhost:3000"
DEFAULT_TOKEN="${ARIATA_TOKEN:-}"

echo -e "${GREEN}🧪 Ariata Mac Monitor LOCAL TEST Installer${NC}"
echo "This will install the locally built version with iMessage support"
echo ""

# Check if local build exists
if [ ! -f "$LOCAL_BUILD" ]; then
    echo -e "${RED}Error: Local build not found at $LOCAL_BUILD${NC}"
    echo "Please run: swift build -c release"
    exit 1
fi

# Show welcome dialog
if ! osascript -e 'display dialog "Welcome to Ariata Mac Monitor LOCAL TEST Setup!

This test installer will:
• Install the locally built version
• Configure with your device token
• Set up message monitoring (iMessage)
• Request Full Disk Access permission

⚠️ NEW: iMessage monitoring requires Full Disk Access

You will need:
• Your device token from the web UI
• Administrator password
• Full Disk Access permission" buttons {"Cancel", "Continue"} default button "Continue" with title "Ariata Mac TEST Setup" with icon note' >/dev/null 2>&1; then
    echo "Installation cancelled by user"
    exit 0
fi

# Get endpoint
ENDPOINT=$(osascript -e "text returned of (display dialog \"Enter Ariata server URL for testing:\" default answer \"$DEFAULT_ENDPOINT\" with title \"Test Setup - Step 1\" with icon note)" 2>/dev/null || echo "")

if [ -z "$ENDPOINT" ]; then
    echo -e "${RED}No endpoint provided${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Endpoint: $ENDPOINT"

# Get token
TOKEN=$(osascript -e "text returned of (display dialog \"Enter your device token:

For testing, you can get this from:
1. Web UI at $ENDPOINT
2. Or check existing config: ariata-mac status\" default answer \"$DEFAULT_TOKEN\" with title \"Test Setup - Step 2\" with icon note)" 2>/dev/null || echo "")

if [ -z "$TOKEN" ]; then
    echo -e "${RED}No token provided${NC}"
    exit 1
fi

TOKEN=$(echo "$TOKEN" | tr '[:lower:]' '[:upper:]')
echo -e "${GREEN}✓${NC} Token: ${TOKEN:0:4}****"

# Stop existing instances
echo "Stopping any existing instances..."
launchctl unload ~/Library/LaunchAgents/com.ariata.mac.plist 2>/dev/null || true
pkill -f "ariata-mac" 2>/dev/null || true
sleep 2

# Install to /usr/local/bin
echo "Installing local build (requires admin password)..."
sudo mkdir -p /usr/local/bin
sudo cp "$LOCAL_BUILD" /usr/local/bin/ariata-mac
sudo chmod +x /usr/local/bin/ariata-mac

echo -e "${GREEN}✓${NC} Installed local build to /usr/local/bin/ariata-mac"

# Configure
echo "Configuring..."
export ARIATA_API_URL="$ENDPOINT"

if ! /usr/local/bin/ariata-mac init "$TOKEN" 2>&1; then
    echo -e "${RED}Configuration failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Configuration complete"

# Check Full Disk Access for iMessage
echo "Checking Full Disk Access permission..."

# Test if we can read the Messages database
if [ -r ~/Library/Messages/chat.db ]; then
    echo -e "${GREEN}✓${NC} Full Disk Access granted - iMessage monitoring enabled"
    MESSAGES_ENABLED="YES"
else
    echo -e "${YELLOW}⚠️${NC} Full Disk Access not granted - iMessage monitoring disabled"
    MESSAGES_ENABLED="NO"
    
    # Show permission instructions
    osascript -e 'display dialog "⚠️ Full Disk Access Required for iMessage Monitoring

To enable iMessage sync, you need to:

1. Open System Settings
2. Go to Privacy & Security → Full Disk Access
3. Click the + button
4. Add /usr/local/bin/ariata-mac
   OR add Terminal (if running from Terminal)
5. Restart ariata-mac after granting permission

Without this, only app monitoring will work.
iMessage sync will be skipped." buttons {"I'\''ll Do This Later", "Open System Settings"} default button "Open System Settings" with title "Permission Required" with icon caution' | grep -q "Open System Settings" && {
        echo "Opening System Settings..."
        open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
    }
fi

# Start daemon
echo "Starting daemon..."
if ! /usr/local/bin/ariata-mac daemon 2>&1; then
    echo -e "${YELLOW}Warning: Could not start daemon${NC}"
fi

# Show final status
STATUS=$(/usr/local/bin/ariata-mac status 2>/dev/null || echo "Status check failed")

# Final dialog
if [ "$MESSAGES_ENABLED" = "YES" ]; then
    MESSAGE_STATUS="✅ iMessage monitoring ENABLED"
else
    MESSAGE_STATUS="⚠️ iMessage monitoring DISABLED (needs Full Disk Access)"
fi

osascript -e "display dialog \"✅ Test Installation Complete!

Configuration:
• Endpoint: $ENDPOINT
• Token: ${TOKEN:0:4}****
• App Monitoring: ✅ ENABLED
• $MESSAGE_STATUS

Monitor Status:
$STATUS

To check logs:
  tail -f ~/.ariata/ariata-mac.log

To verify messages in database:
  docker compose exec postgres psql -U ariata_user -d ariata -c 'SELECT COUNT(*) FROM stream_mac_messages;'\" buttons {\"Done\"} default button \"Done\" with title \"Test Setup Complete\" with icon note"

echo -e "${GREEN}✅ Test installation complete!${NC}"
echo ""
echo "Status:"
echo "  App Monitoring: ENABLED"
if [ "$MESSAGES_ENABLED" = "YES" ]; then
    echo "  iMessage Monitoring: ENABLED"
else
    echo "  iMessage Monitoring: DISABLED (grant Full Disk Access to enable)"
fi
echo ""
echo "Check logs: tail -f ~/.ariata/ariata-mac.log"
echo "Check messages: docker compose exec postgres psql -U ariata_user -d ariata -c 'SELECT COUNT(*) FROM stream_mac_messages;'"