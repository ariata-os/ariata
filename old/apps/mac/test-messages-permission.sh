#!/bin/bash

echo "🔍 Testing Messages Database Access"
echo "===================================="
echo ""

# Test 1: File visibility
echo "Test 1: Can we see the file?"
if [ -f ~/Library/Messages/chat.db ]; then
    echo "✅ File exists: ~/Library/Messages/chat.db"
    ls -lh ~/Library/Messages/chat.db
else
    echo "❌ File not found"
fi
echo ""

# Test 2: Read permission
echo "Test 2: Can we read the file?"
if [ -r ~/Library/Messages/chat.db ]; then
    echo "✅ Read permission granted"
else
    echo "❌ No read permission"
fi
echo ""

# Test 3: SQLite access
echo "Test 3: Can SQLite open the database?"
if sqlite3 ~/Library/Messages/chat.db "SELECT COUNT(*) FROM message;" 2>/dev/null >/dev/null; then
    MESSAGE_COUNT=$(sqlite3 ~/Library/Messages/chat.db "SELECT COUNT(*) FROM message;" 2>/dev/null)
    echo "✅ SQLite can open database"
    echo "   Found $MESSAGE_COUNT messages in database"
else
    echo "❌ SQLite cannot open database (Full Disk Access required)"
    echo ""
    echo "📝 To grant Full Disk Access:"
    echo "   1. Open System Settings"
    echo "   2. Go to Privacy & Security → Full Disk Access"
    echo "   3. Enable Terminal (or iTerm/your terminal app)"
    echo "   4. Restart your terminal"
    echo "   5. Run this test again"
fi
echo ""

# Show current terminal app
TERMINAL_APP=$(ps -p $PPID -o comm= | xargs basename)
echo "Current terminal: $TERMINAL_APP"
echo ""

# Check if we're in Terminal and offer to open settings
if [ "$TERMINAL_APP" = "Terminal" ] || [ "$TERMINAL_APP" = "iTerm2" ] || [ "$TERMINAL_APP" = "bash" ]; then
    echo "Would you like to open System Settings to grant Full Disk Access? (y/n)"
    read -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
        echo ""
        echo "👉 Please:"
        echo "   1. Find and enable '$TERMINAL_APP' in the list"
        echo "   2. Restart your terminal after granting access"
        echo "   3. Run this test again to verify"
    fi
fi