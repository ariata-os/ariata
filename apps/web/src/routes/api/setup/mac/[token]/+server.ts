import { json, text } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db';
import { sources } from '$lib/db/schema';
import { eq, and } from 'drizzle-orm';

export const GET: RequestHandler = async ({ params, url }) => {
	const { token } = params;
	
	// Validate token format (8 alphanumeric characters)
	if (!token || !/^[A-Z0-9]{8}$/.test(token)) {
		return text('Invalid token format', { status: 400 });
	}
	
	try {
		// Check if token exists and is valid
		const source = await db
			.select()
			.from(sources)
			.where(
				and(
					eq(sources.authToken, token),
					eq(sources.sourceName, 'mac'),
					eq(sources.status, 'active')
				)
			)
			.limit(1);
		
		if (source.length === 0) {
			return text('Invalid or expired token. Please generate a new token from the web UI.', { 
				status: 404 
			});
		}
		
		// Get the base URL for the API
		const baseUrl = url.origin || 'http://localhost:3000';
		
		// Generate the installer script
		const installerScript = `#!/bin/bash
set -e

# Ariata Mac CLI Installer
# Generated for token: ${token}

echo "======================================"
echo "      Ariata Mac CLI Installer       "
echo "======================================"
echo ""

# Configuration
TOKEN="${token}"
API_URL="${baseUrl}"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"

# Colors for output
RED='\\033[0;31m'
GREEN='\\033[0;32m'
YELLOW='\\033[1;33m'
NC='\\033[0m' # No Color

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo -e "\${RED}❌ This installer is for macOS only\${NC}"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
echo "Detected architecture: $ARCH"

# Download the latest release
echo -e "\\n\${GREEN}📦 Downloading Ariata Mac CLI...\${NC}"
DOWNLOAD_URL="https://github.com/ariata/ariata/releases/latest/download/ariata-mac-universal.tar.gz"

# Create temp directory
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Download
if command -v curl &> /dev/null; then
    curl -L -o ariata-mac.tar.gz "$DOWNLOAD_URL" || {
        echo -e "\${RED}❌ Failed to download\${NC}"
        exit 1
    }
else
    echo -e "\${RED}❌ curl is required but not installed\${NC}"
    exit 1
fi

# Extract
echo -e "\${GREEN}📦 Extracting...\${NC}"
tar -xzf ariata-mac.tar.gz

# Install binary
echo -e "\${GREEN}📦 Installing to $INSTALL_DIR...\${NC}"
if [ -w "$INSTALL_DIR" ]; then
    mv ariata-mac "$INSTALL_DIR/ariata-mac"
    chmod +x "$INSTALL_DIR/ariata-mac"
else
    echo "Administrator password required to install to $INSTALL_DIR"
    sudo mv ariata-mac "$INSTALL_DIR/ariata-mac"
    sudo chmod +x "$INSTALL_DIR/ariata-mac"
fi

# Clean up temp files
cd - > /dev/null
rm -rf "$TEMP_DIR"

# Verify installation
if ! command -v ariata-mac &> /dev/null; then
    echo -e "\${YELLOW}⚠️  ariata-mac installed but not in PATH\${NC}"
    echo "Add $INSTALL_DIR to your PATH or use: $INSTALL_DIR/ariata-mac"
    ARIATA_CMD="$INSTALL_DIR/ariata-mac"
else
    echo -e "\${GREEN}✅ ariata-mac installed successfully\${NC}"
    ARIATA_CMD="ariata-mac"
fi

# Configure with token
echo -e "\\n\${GREEN}🔧 Configuring with your device token...\${NC}"
$ARIATA_CMD init "$TOKEN" || {
    echo -e "\${RED}❌ Failed to configure. Please run manually: ariata-mac init $TOKEN\${NC}"
    exit 1
}

# Ask about auto-start
echo -e "\\n\${YELLOW}Would you like to enable auto-start on login? (recommended)\${NC}"
read -p "Enable auto-start? [Y/n]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
    echo -e "\${GREEN}🚀 Setting up auto-start...\${NC}"
    $ARIATA_CMD daemon
    echo -e "\${GREEN}✅ Auto-start enabled\${NC}"
else
    echo -e "\${YELLOW}ℹ️  You can enable auto-start later with: ariata-mac daemon\${NC}"
fi

# Final instructions
echo -e "\\n\${GREEN}======================================"
echo -e "     ✅ Installation Complete!        "
echo -e "======================================\${NC}"
echo ""
echo "Your Mac is now connected to Ariata!"
echo ""
echo -e "\${YELLOW}Useful commands:\${NC}"
echo "  ariata-mac status    - Check current status"
echo "  ariata-mac daemon    - Enable auto-start on login"
echo "  ariata-mac stop      - Stop monitoring"
echo "  ariata-mac reset     - Reset configuration"
echo ""
echo -e "\${GREEN}Data is being collected and will sync every 5 minutes.\${NC}"
echo -e "View your data at: \${YELLOW}${baseUrl}\${NC}"
`;

		// Return the script with appropriate headers
		return new Response(installerScript, {
			status: 200,
			headers: {
				'Content-Type': 'text/plain; charset=utf-8',
				'Content-Disposition': 'inline; filename="install-ariata-mac.sh"',
				'Cache-Control': 'no-cache, no-store, must-revalidate',
			},
		});
	} catch (error) {
		console.error('Error generating installer:', error);
		return text('Failed to generate installer script', { status: 500 });
	}
};

// Also support POST for token validation
export const POST: RequestHandler = async ({ params }) => {
	const { token } = params;
	
	// Validate token format
	if (!token || !/^[A-Z0-9]{8}$/.test(token)) {
		return json({ valid: false, error: 'Invalid token format' }, { status: 400 });
	}
	
	try {
		// Check if token exists
		const source = await db
			.select()
			.from(sources)
			.where(
				and(
					eq(sources.authToken, token),
					eq(sources.sourceName, 'mac'),
					eq(sources.status, 'active')
				)
			)
			.limit(1);
		
		if (source.length === 0) {
			return json({ valid: false, error: 'Token not found or inactive' }, { status: 404 });
		}
		
		return json({ 
			valid: true, 
			source_id: source[0].id,
			device_name: source[0].displayName || 'Mac'
		});
	} catch (error) {
		console.error('Error validating token:', error);
		return json({ valid: false, error: 'Server error' }, { status: 500 });
	}
};