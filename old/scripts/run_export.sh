#!/bin/bash
# Quick script to run the iOS test data export with proper environment

set -e

echo "🚀 Starting iOS test data export for September 15, 2025"
echo ""

# Check if running in Docker environment or local
if [ -f /.dockerenv ]; then
    echo "📦 Running in Docker environment"
    DB_HOST=postgres
    MINIO_ENDPOINT=http://minio:9000
else
    echo "💻 Running in local environment"
    DB_HOST=localhost
    MINIO_ENDPOINT=http://localhost:9000
fi

# Export environment variables
export DB_HOST=${DB_HOST}
export DB_PORT=5432
export DB_NAME=ariata
export DB_USER=ariata_user
export DB_PASSWORD=ariata_password

export MINIO_ENDPOINT=${MINIO_ENDPOINT}
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=ariata
export MINIO_REGION=us-east-1

# Check Python dependencies
echo "📋 Checking dependencies..."
python3 -c "import psycopg2" 2>/dev/null || {
    echo "❌ psycopg2 not installed. Installing..."
    pip3 install psycopg2-binary
}

python3 -c "import boto3" 2>/dev/null || {
    echo "❌ boto3 not installed. Installing..."
    pip3 install boto3
}

# Check for archive flag
ARCHIVE_FLAG=""
if [[ "$1" == "--archive" ]]; then
    ARCHIVE_FLAG="--archive"
    echo "📦 Archive mode enabled"
fi

# Run the export script
echo ""
echo "🏃 Running export script..."
python3 scripts/export_ios_test_data.py $ARCHIVE_FLAG

echo ""
echo "✅ Export complete!"
echo ""
echo "💡 Tip: Use './scripts/run_export.sh --archive' to automatically create a .tar.gz archive"