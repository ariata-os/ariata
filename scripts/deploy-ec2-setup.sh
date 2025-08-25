#!/bin/bash
# EC2 Setup Script for Ariata
# This script sets up a fresh Ubuntu EC2 instance with Docker and the Ariata stack

set -e  # Exit on error

echo "🚀 Starting EC2 setup for Ariata..."

# Update system
echo "📦 Updating system packages..."
sudo apt-get update -y
sudo apt-get upgrade -y

# Install Docker
echo "🐳 Installing Docker..."
sudo apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release

# Add Docker's official GPG key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

# Set up the Docker repository
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker Engine
sudo apt-get update -y
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Install Docker Compose standalone
echo "🔧 Installing Docker Compose..."
sudo curl -L "https://github.com/docker/compose/releases/download/v2.24.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Add current user to docker group
sudo usermod -aG docker $USER

# Install other dependencies
echo "📦 Installing additional tools..."
sudo apt-get install -y git make

# Create app directory
echo "📁 Creating application directory..."
mkdir -p ~/ariata
cd ~/ariata

# Get public IP
PUBLIC_IP=$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4)
echo "🌐 Detected public IP: $PUBLIC_IP"

# Create initial .env file
if [ ! -f .env ]; then
    echo "📝 Creating .env file..."
    cat > .env << EOF
# Environment Configuration
ENVIRONMENT=production

# Server Configuration
PUBLIC_IP=$PUBLIC_IP
FRONTEND_URL=http://$PUBLIC_IP:3000
PROCESSING_SERVICE_URL=http://processing:8001

# Database Configuration
DB_USER=ariata_user
DB_PASSWORD=ariata_password
DB_NAME=ariata
DB_HOST=postgres
DB_PORT=5432

# Redis Configuration
REDIS_HOST=redis
REDIS_PORT=6379

# MinIO Configuration
MINIO_ENDPOINT=http://minio:9000
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
MINIO_USE_SSL=false

# Google OAuth Configuration
GOOGLE_CLIENT_ID=your-google-client-id-here
GOOGLE_CLIENT_SECRET=your-google-client-secret-here

# Security
ENCRYPTION_KEY=your-32-character-encryption-key-here!

# Service Ports
WEB_PORT=3000
MINIO_PORT=9000
MINIO_CONSOLE_PORT=9001
REDIS_PORT=6379
PROCESSING_PORT=8001
POSTGRES_PORT=5432
EOF
    echo "⚠️  Please edit .env with your actual Google OAuth credentials and encryption key"
fi

# Setup systemd service for auto-start
echo "🔧 Setting up systemd service..."
sudo tee /etc/systemd/system/jaces.service > /dev/null << EOF
[Unit]
Description=Jaces Docker Compose Application
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/$USER/jaces
ExecStart=/usr/local/bin/docker-compose up -d
ExecStop=/usr/local/bin/docker-compose down
User=$USER

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable jaces.service

echo "✅ EC2 setup complete!"
echo ""
echo "📋 Next steps:"
echo "1. Log out and back in for Docker permissions: 'exit' then ssh back in"
echo "2. Extract your deployment package: 'tar -xzf deploy.tar.gz'"
echo "3. Edit .env with your actual credentials"
echo "4. Start the services: 'make prod'"
echo ""
echo "🔒 Security Group Requirements:"
echo "   - Port 22 (SSH)"
echo "   - Port 3000 (Web/API)"
echo "   - Port 9001 (MinIO Console)"
echo ""
echo "📱 iPhone App Configuration:"
echo "   - API URL: http://$PUBLIC_IP:3000/api/ingest"
echo "   - Use the same device token as local development"