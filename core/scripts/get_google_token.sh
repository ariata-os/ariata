#!/bin/bash
# Get Google OAuth Refresh Token for Testing
#
# This script helps you obtain a refresh token from your Google account
# that can be used for automated testing without browser interaction.
#
# Prerequisites:
# - GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET must be set in .env
# - auth.ariata.com must be running and accessible
#
# Usage:
#   ./scripts/get_google_token.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔐 Google OAuth Refresh Token Generator${NC}"
echo "========================================="
echo ""

# Load environment variables from .env
ENV_FILE=""
if [ -f "../.env" ]; then
    ENV_FILE="../.env"
elif [ -f ".env" ]; then
    ENV_FILE=".env"
else
    echo -e "${RED}❌ Error: .env file not found${NC}"
    echo "Please run this script from the core/ directory or ensure .env exists"
    exit 1
fi

# Parse .env file more carefully to avoid issues with special characters
while IFS='=' read -r key value; do
    # Skip comments and empty lines
    [[ "$key" =~ ^#.*  ]] && continue
    [[ -z "$key" ]] && continue
    # Remove inline comments from value
    value=$(echo "$value" | sed 's/#.*//' | xargs)
    # Export the variable
    export "$key=$value"
done < <(grep -v '^[[:space:]]*#' "$ENV_FILE" | grep -v '^[[:space:]]*$' | grep '=')

# Check required environment variables
if [ -z "$GOOGLE_CLIENT_ID" ]; then
    echo -e "${RED}❌ Error: GOOGLE_CLIENT_ID not set in .env${NC}"
    exit 1
fi

if [ -z "$GOOGLE_CLIENT_SECRET" ]; then
    echo -e "${RED}❌ Error: GOOGLE_CLIENT_SECRET not set in .env${NC}"
    exit 1
fi

echo -e "${BLUE}Using OAuth credentials:${NC}"
echo "  Client ID: ${GOOGLE_CLIENT_ID:0:20}..."
echo "  Redirect URI: https://auth.ariata.com/google/callback"
echo ""

# Generate state for CSRF protection
STATE=$(openssl rand -hex 16)
CALLBACK_URL="http://localhost:8888/callback"

# Build the OAuth URL
AUTH_URL="https://auth.ariata.com/google/auth?return_url=$(printf %s "$CALLBACK_URL" | jq -sRr @uri)&state=$STATE"

echo -e "${YELLOW}📝 Instructions:${NC}"
echo "1. A browser window will open with Google's consent screen"
echo "2. Log in with your TEST Google account (not your personal one!)"
echo "3. Click 'Allow' to grant permissions"
echo "4. You'll be redirected back to this script"
echo ""
echo -e "${YELLOW}⚠️  Important:${NC}"
echo "  • Use a dedicated test account (e.g., ariata-test@gmail.com)"
echo "  • The refresh token will be saved to use in automated tests"
echo "  • This account should have test calendar data"
echo ""

read -p "Press Enter to open the authorization URL in your browser..."

# Start a simple HTTP server to catch the callback
echo ""
echo -e "${BLUE}🚀 Starting local callback server on port 8888...${NC}"

# Create a temporary file for the token response
TEMP_FILE=$(mktemp)

# Start callback server in background
(
python3 - <<'PYEOF'
import http.server
import socketserver
import urllib.parse
import sys
import os

class CallbackHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # Suppress log messages

    def do_GET(self):
        # Parse the URL
        parsed = urllib.parse.urlparse(self.path)
        params = urllib.parse.parse_qs(parsed.query)

        if parsed.path == '/callback':
            # Extract tokens from URL parameters
            if 'error' in params:
                self.send_response(400)
                self.send_header('Content-type', 'text/html')
                self.end_headers()
                error_msg = params['error'][0]
                self.wfile.write(f"""
                    <html><body style="font-family: sans-serif; max-width: 600px; margin: 50px auto;">
                    <h1 style="color: #dc3545;">❌ Authorization Failed</h1>
                    <p>Error: {error_msg}</p>
                    <p>You can close this window and try again.</p>
                    </body></html>
                """.encode())
                return

            # Extract tokens
            access_token = params.get('access_token', [''])[0]
            refresh_token = params.get('refresh_token', [''])[0]
            expires_in = params.get('expires_in', [''])[0]

            # Save to temp file
            temp_file = os.environ.get('TEMP_FILE', '/tmp/oauth_tokens')
            with open(temp_file, 'w') as f:
                f.write(f"ACCESS_TOKEN={access_token}\n")
                f.write(f"REFRESH_TOKEN={refresh_token}\n")
                f.write(f"EXPIRES_IN={expires_in}\n")

            # Send success response
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()

            if refresh_token:
                self.wfile.write("""
                    <html><body style="font-family: sans-serif; max-width: 600px; margin: 50px auto;">
                    <h1 style="color: #28a745;">✅ Authorization Successful!</h1>
                    <p>Your refresh token has been received.</p>
                    <p><strong>You can close this window and return to the terminal.</strong></p>
                    </body></html>
                """.encode())
            else:
                self.wfile.write("""
                    <html><body style="font-family: sans-serif; max-width: 600px; margin: 50px auto;">
                    <h1 style="color: #ffc107;">⚠️ Warning</h1>
                    <p>No refresh token was returned. You may need to:</p>
                    <ol>
                        <li>Revoke access at <a href="https://myaccount.google.com/permissions">myaccount.google.com/permissions</a></li>
                        <li>Run this script again</li>
                    </ol>
                    </body></html>
                """.encode())
        else:
            self.send_response(404)
            self.end_headers()

PORT = 8888
TEMP_FILE = os.environ.get('TEMP_FILE', '/tmp/oauth_tokens')

with socketserver.TCPServer(("", PORT), CallbackHandler) as httpd:
    print(f"Callback server running on port {PORT}", file=sys.stderr)
    httpd.serve_forever()
PYEOF
) &

SERVER_PID=$!

# Give server time to start
sleep 1

# Open the authorization URL
echo -e "${BLUE}🌐 Opening browser...${NC}"
if command -v open &> /dev/null; then
    open "$AUTH_URL"
elif command -v xdg-open &> /dev/null; then
    xdg-open "$AUTH_URL"
else
    echo ""
    echo -e "${YELLOW}Could not open browser automatically.${NC}"
    echo "Please open this URL manually:"
    echo ""
    echo "$AUTH_URL"
    echo ""
fi

echo -e "${BLUE}⏳ Waiting for authorization...${NC}"
echo "(This window is waiting for you to complete the authorization in your browser)"
echo ""

# Wait for the callback (max 5 minutes)
TIMEOUT=300
ELAPSED=0
while [ ! -f "$TEMP_FILE" ] && [ $ELAPSED -lt $TIMEOUT ]; do
    sleep 1
    ELAPSED=$((ELAPSED + 1))

    # Show a dot every 5 seconds
    if [ $((ELAPSED % 5)) -eq 0 ]; then
        echo -n "."
    fi
done

echo ""

# Kill the server
kill $SERVER_PID 2>/dev/null || true
sleep 1

# Check if we got the token
if [ ! -f "$TEMP_FILE" ]; then
    echo -e "${RED}❌ Timeout waiting for authorization${NC}"
    echo "Please try again and make sure to complete the authorization in your browser."
    exit 1
fi

# Read the tokens
source "$TEMP_FILE"
rm "$TEMP_FILE"

if [ -z "$REFRESH_TOKEN" ]; then
    echo -e "${RED}❌ No refresh token received${NC}"
    echo ""
    echo "This usually happens if you've already authorized this app."
    echo "To get a refresh token, you need to:"
    echo "1. Go to https://myaccount.google.com/permissions"
    echo "2. Find 'Ariata' and remove access"
    echo "3. Run this script again"
    exit 1
fi

echo -e "${GREEN}✅ Success! Received OAuth tokens${NC}"
echo ""
echo -e "${BLUE}Refresh Token:${NC}"
echo "$REFRESH_TOKEN"
echo ""

# Ask if user wants to add to .env
echo -e "${YELLOW}Would you like to add this to your .env file?${NC}"
read -p "Add to .env? (y/n): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    ENV_FILE="../.env"
    if [ ! -f "$ENV_FILE" ]; then
        ENV_FILE=".env"
    fi

    # Check if the section already exists
    if grep -q "GOOGLE_TEST_REFRESH_TOKEN" "$ENV_FILE"; then
        # Update existing token
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS
            sed -i '' "s|GOOGLE_TEST_REFRESH_TOKEN=.*|GOOGLE_TEST_REFRESH_TOKEN=$REFRESH_TOKEN|" "$ENV_FILE"
        else
            # Linux
            sed -i "s|GOOGLE_TEST_REFRESH_TOKEN=.*|GOOGLE_TEST_REFRESH_TOKEN=$REFRESH_TOKEN|" "$ENV_FILE"
        fi
        echo -e "${GREEN}✅ Updated GOOGLE_TEST_REFRESH_TOKEN in $ENV_FILE${NC}"
    else
        # Add new section
        cat >> "$ENV_FILE" << EOF

################################################
#            Test OAuth Tokens                 #
################################################

# Google Test Account Refresh Token
# Generated with: ./core/scripts/get_google_token.sh
# This token is used for automated testing and should be from a dedicated test account
GOOGLE_TEST_REFRESH_TOKEN=$REFRESH_TOKEN

# For other providers (add as needed):
# NOTION_TEST_REFRESH_TOKEN=
# STRAVA_TEST_REFRESH_TOKEN=
# MICROSOFT_TEST_REFRESH_TOKEN=
EOF
        echo -e "${GREEN}✅ Added GOOGLE_TEST_REFRESH_TOKEN to $ENV_FILE${NC}"
    fi

    echo ""
    echo -e "${BLUE}📝 Next steps:${NC}"
    echo "1. This refresh token will work indefinitely (while your app is in Testing mode)"
    echo "2. You can now run: cargo test test_google_calendar_real_oauth_e2e -- --ignored --nocapture"
    echo "3. Consider adding test events to your test account's calendar for predictable test data"
else
    echo ""
    echo -e "${BLUE}📝 To use this token:${NC}"
    echo "Add this line to your .env file:"
    echo ""
    echo "GOOGLE_TEST_REFRESH_TOKEN=$REFRESH_TOKEN"
fi

echo ""
echo -e "${GREEN}✨ Done!${NC}"
