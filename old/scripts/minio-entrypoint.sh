#!/bin/sh
# Custom entrypoint for MinIO that creates buckets after server starts

# Start MinIO server in the background
minio server /data --console-address ":9001" &
MINIO_PID=$!

# Wait for MinIO to be ready
echo "Waiting for MinIO to start..."
until mc alias set local http://localhost:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD" > /dev/null 2>&1; do
  sleep 1
done

echo "MinIO is up - creating buckets"

# Create the ariata bucket if it doesn't exist
mc mb local/ariata --ignore-existing > /dev/null 2>&1
echo "Bucket 'ariata' ready"

# Bring MinIO server to foreground
wait $MINIO_PID