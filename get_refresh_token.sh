#!/bin/bash

# Google OAuth Refresh Token Helper Script
# This script helps you obtain a Google refresh token for testing

echo "=== Google OAuth Refresh Token Helper ==="
echo ""
echo "Prerequisites:"
echo "1. Enable Google Calendar API in Google Cloud Console"
echo "2. Create OAuth 2.0 credentials (Web application type)"
echo "3. Add http://localhost:8080 to authorized redirect URIs"
echo ""

# Check if client ID and secret are provided as arguments or prompt for them
if [ -z "$1" ] || [ -z "$2" ]; then
    read -p "Enter your Google Client ID: " CLIENT_ID
    read -sp "Enter your Google Client Secret: " CLIENT_SECRET
    echo ""
else
    CLIENT_ID=$1
    CLIENT_SECRET=$2
fi

# OAuth configuration
REDIRECT_URI="http://localhost:8080"
SCOPE="https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/calendar.events.readonly"

# Generate the authorization URL
AUTH_URL="https://accounts.google.com/o/oauth2/v2/auth?client_id=${CLIENT_ID}&redirect_uri=${REDIRECT_URI}&response_type=code&scope=${SCOPE// /%20}&access_type=offline&prompt=consent"

echo ""
echo "Step 1: Open this URL in your browser:"
echo ""
echo "$AUTH_URL"
echo ""
echo "Step 2: After authorizing, you'll see an error page (expected)."
echo "        Copy the 'code' parameter from the URL."
echo ""
read -p "Enter the authorization code: " AUTH_CODE

echo ""
echo "Exchanging code for tokens..."

# Exchange the authorization code for tokens
TOKEN_RESPONSE=$(curl -s -X POST "https://oauth2.googleapis.com/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "client_id=${CLIENT_ID}" \
    -d "client_secret=${CLIENT_SECRET}" \
    -d "code=${AUTH_CODE}" \
    -d "redirect_uri=${REDIRECT_URI}" \
    -d "grant_type=authorization_code")

# Extract the refresh token
REFRESH_TOKEN=$(echo "$TOKEN_RESPONSE" | grep -o '"refresh_token":"[^"]*' | cut -d'"' -f4)

if [ -n "$REFRESH_TOKEN" ]; then
    echo ""
    echo "✅ Success! Your refresh token is:"
    echo ""
    echo "$REFRESH_TOKEN"
    echo ""
    echo "Add this to your .env file:"
    echo "GOOGLE_REFRESH_TOKEN=$REFRESH_TOKEN"
    echo ""

    # Optionally append to .env
    read -p "Do you want to append this to core/.env? (y/n): " APPEND_ENV
    if [ "$APPEND_ENV" = "y" ]; then
        echo "" >> core/.env
        echo "# Google OAuth" >> core/.env
        echo "GOOGLE_REFRESH_TOKEN=$REFRESH_TOKEN" >> core/.env
        echo "✅ Added to core/.env"
    fi
else
    echo ""
    echo "❌ Error: No refresh token received"
    echo "Response: $TOKEN_RESPONSE"
    echo ""
    echo "Make sure:"
    echo "1. You included 'access_type=offline' and 'prompt=consent'"
    echo "2. This is the first time authorizing, or you revoked access first"
fi