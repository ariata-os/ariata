#!/bin/bash
# Quick Tailscale deployment helper

set -e

echo "🚀 Ariata Tailscale Deployment"
echo "=============================="

# Check if .env exists
if [ -f .env ]; then
    echo "⚠️  .env already exists. Backing up to .env.backup"
    cp .env .env.backup
fi

# Copy template
cp .env.example .env

# Get Tailscale IP
if command -v tailscale &> /dev/null; then
    TAILSCALE_IP=$(tailscale ip -4 2>/dev/null || echo "")
    if [ -n "$TAILSCALE_IP" ]; then
        echo "✅ Found Tailscale IP: $TAILSCALE_IP"

        # Update .env with Tailscale IP (replace localhost with Tailscale IP)
        sed -i "s/PUBLIC_IP=localhost/PUBLIC_IP=$TAILSCALE_IP/g" .env
        sed -i "s|FRONTEND_URL=http://localhost:3000|FRONTEND_URL=http://$TAILSCALE_IP:3000|g" .env
    else
        echo "❌ Tailscale not connected. Please run: sudo tailscale up"
        exit 1
    fi
else
    echo "❌ Tailscale not installed!"
    echo "Install with: curl -fsSL https://tailscale.com/install.sh | sh"
    exit 1
fi

# Generate secure passwords
echo "🔐 Generating secure passwords..."
DB_PASS=$(openssl rand -hex 16)
MINIO_PASS=$(openssl rand -hex 16)
ENCRYPTION_KEY=$(openssl rand -hex 16)

sed -i "s/ariata_password/$DB_PASS/g" .env
sed -i "s/minioadmin/$MINIO_PASS/g" .env
sed -i "s/your-32-character-encryption-key-here!/$ENCRYPTION_KEY/g" .env

echo ""
echo "📝 Next steps:"
echo "1. Add your OAuth credentials to .env (if using Google Calendar):"
echo "   nano .env"
echo ""
echo "2. Start services:"
echo "   docker compose up -d"
echo ""
echo "3. Check health (wait 30 seconds first):"
echo "   curl http://localhost:3000/api/health | jq"
echo ""
echo "4. Access from anywhere via Tailscale:"
echo "   Web UI: http://$TAILSCALE_IP:3000"
echo "   Health: http://$TAILSCALE_IP:3000/api/health"
echo ""
echo "5. Configure devices with server URL:"
echo "   http://$TAILSCALE_IP:3000"
