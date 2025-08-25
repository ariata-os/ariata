#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Load environment variables from root .env if it exists
if [ -f "../../.env" ]; then
    # Source the file to handle quoted values properly
    set -a
    source ../../.env
    set +a
    echo "Loaded environment from root .env"
fi

echo -e "${GREEN}🔨 Building Ariata Mac CLI Release...${NC}"

# Get version from Version.swift
VERSION=$(grep 'static let current' Sources/Version.swift | cut -d'"' -f2)
echo -e "Version: ${YELLOW}$VERSION${NC}"

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf .build
rm -f ariata-mac ariata-mac-*.tar.gz ariata-mac-*.zip

# Build for x86_64 (Intel)
echo -e "\n${GREEN}Building for Intel (x86_64)...${NC}"
swift build -c release --arch x86_64

# Build for arm64 (Apple Silicon)
echo -e "\n${GREEN}Building for Apple Silicon (arm64)...${NC}"
swift build -c release --arch arm64

# Create universal binary
echo -e "\n${GREEN}Creating universal binary...${NC}"
lipo -create \
    .build/x86_64-apple-macosx/release/ariata-mac \
    .build/arm64-apple-macosx/release/ariata-mac \
    -output ariata-mac

# Make it executable
chmod +x ariata-mac

# Strip debug symbols to reduce size
echo "Stripping debug symbols..."
strip ariata-mac

# Code signing (optional - if identity is provided)
if [ -n "${CODESIGN_IDENTITY}" ]; then
    echo -e "\n${GREEN}Signing binary...${NC}"
    echo "Identity: ${CODESIGN_IDENTITY}"
    
    codesign --force --deep \
             --sign "${CODESIGN_IDENTITY}" \
             --options runtime \
             --timestamp \
             --entitlements ../Scripts/entitlements.plist \
             ariata-mac 2>/dev/null || {
        # If entitlements don't exist, sign without them
        codesign --force --deep \
                 --sign "${CODESIGN_IDENTITY}" \
                 --options runtime \
                 --timestamp \
                 ariata-mac
    }
    
    # Verify signature
    if codesign --verify --verbose ariata-mac 2>&1 | grep -q "satisfies its Designated Requirement"; then
        echo -e "${GREEN}✅ Binary signed successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  Signature verification failed${NC}"
    fi
else
    echo -e "\n${YELLOW}⚠️  No signing identity provided. Binary will be unsigned.${NC}"
    echo "To sign, set CODESIGN_IDENTITY environment variable"
fi

# Verify the binary
echo -e "\n${GREEN}Verifying binary...${NC}"
echo "File size: $(du -h ariata-mac | cut -f1)"
echo "Architectures:"
lipo -info ariata-mac
if [ -n "${CODESIGN_IDENTITY}" ]; then
    echo "Signature:"
    codesign -dv ariata-mac 2>&1 | grep "Authority" | head -1
fi

# Test the binary
echo -e "\n${GREEN}Testing binary...${NC}"
./ariata-mac --version

# Create archives
echo -e "\n${GREEN}Creating archives...${NC}"
tar -czf "ariata-mac-${VERSION}-universal.tar.gz" ariata-mac
zip -q "ariata-mac-${VERSION}-universal.zip" ariata-mac

# Also create unversioned archives for 'latest'
cp "ariata-mac-${VERSION}-universal.tar.gz" "ariata-mac-universal.tar.gz"
cp "ariata-mac-${VERSION}-universal.zip" "ariata-mac-universal.zip"

echo -e "\n${GREEN}✅ Build complete!${NC}"
echo -e "Files created:"
echo -e "  - ariata-mac (universal binary)"
echo -e "  - ariata-mac-${VERSION}-universal.tar.gz"
echo -e "  - ariata-mac-${VERSION}-universal.zip"
echo -e "\nBinary size: $(du -h ariata-mac | cut -f1)"
echo -e "\nTo install locally:"
echo -e "  ${YELLOW}sudo cp ariata-mac /usr/local/bin/${NC}"
echo -e "\nTo test:"
echo -e "  ${YELLOW}./ariata-mac --help${NC}"