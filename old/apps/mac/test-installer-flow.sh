#!/bin/bash
# Test the updated installer flow without sudo

echo "🧪 Testing Updated Installer Flow (Simulation)"
echo "=============================================="
echo ""

# Simulate the iMessage check from installer
echo "Checking iMessage monitoring capability..."

if sqlite3 ~/Library/Messages/chat.db "SELECT 1 LIMIT 1" 2>/dev/null >/dev/null; then
    echo "✅ Full Disk Access granted - iMessage monitoring enabled"
    IMESSAGE_STATUS="✅ iMessage monitoring: ENABLED"
else
    echo "⚠️ Full Disk Access needed for iMessage monitoring"
    
    # Show what the dialog would look like
    echo ""
    echo "📱 Dialog Preview:"
    echo "┌─────────────────────────────────────────┐"
    echo "│ Enable iMessage Monitoring? (Optional)  │"
    echo "│                                          │"
    echo "│ Ariata can sync your iMessage history   │"
    echo "│ for backup and search.                  │"
    echo "│                                          │"
    echo "│ This requires Full Disk Access:         │"
    echo "│ • Messages stay private and local       │"
    echo "│ • Only syncs to YOUR server             │"
    echo "│ • Can be disabled anytime               │"
    echo "│                                          │"
    echo "│ [Skip This]  [Enable iMessage Sync]     │"
    echo "└─────────────────────────────────────────┘"
    echo ""
    
    # Test the actual dialog
    USER_CHOICE=$(osascript -e 'button returned of (display dialog "📱 Enable iMessage Monitoring? (Optional)

Ariata can sync your iMessage history for backup and search.

This requires Full Disk Access permission:
• Your messages stay private and local
• Only syncs to YOUR Ariata server  
• Can be disabled anytime

Without this, Ariata will only monitor app usage.

Would you like to enable iMessage sync?" buttons {"Skip This", "Enable iMessage Sync"} default button "Enable iMessage Sync" with title "Optional: iMessage Monitoring" with icon note)' 2>/dev/null || echo "Skip This")
    
    echo "User selected: $USER_CHOICE"
    
    if [[ "$USER_CHOICE" == "Enable iMessage Sync" ]]; then
        echo ""
        echo "Would open System Settings to Full Disk Access panel"
        echo "Command: open 'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles'"
        
        # Show instruction dialog
        osascript -e 'display dialog "📝 To Enable iMessage Monitoring:

1. System Settings would open here
2. Find '\''Full Disk Access'\'' 
3. Click the + button
4. Navigate to /usr/local/bin/
5. Select '\''ariata-mac'\''
6. Click Open

After granting access, iMessage sync will start automatically.

Note: You may need to restart ariata-mac after granting permission:
  ariata-mac stop
  ariata-mac start" buttons {"OK"} default button "OK" with title "Grant Permission (Demo)" with icon note'
        
        IMESSAGE_STATUS="⏳ iMessage monitoring: PENDING (grant Full Disk Access)"
    else
        IMESSAGE_STATUS="⏭️ iMessage monitoring: SKIPPED (optional)"
    fi
fi

echo ""
echo "Final Status:"
echo "  App Monitoring: ✅ ENABLED"
echo "  $IMESSAGE_STATUS"
echo ""
echo "✨ Test complete!"