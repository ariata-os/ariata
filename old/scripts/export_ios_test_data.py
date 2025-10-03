#!/usr/bin/env python3
"""
Export iOS test data from MinIO and PostgreSQL for a specific date.
Exports all audio files and stream data from September 15, 2025.
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
import psycopg2
from psycopg2.extras import RealDictCursor
import boto3
from botocore.exceptions import ClientError
import logging

# Set up logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Configuration
TARGET_DATE = "2025-09-30"  # September 30, 2025
OUTPUT_DIR = f"test-data-{TARGET_DATE.replace('-', '')}"

# Database configuration
DB_CONFIG = {
    'host': os.getenv('DB_HOST', 'localhost'),
    'port': int(os.getenv('DB_PORT', 5432)),
    'database': os.getenv('DB_NAME', 'ariata'),
    'user': os.getenv('DB_USER', 'ariata_user'),
    'password': os.getenv('DB_PASSWORD', 'ariata_password')
}

# MinIO configuration
MINIO_CONFIG = {
    'endpoint_url': os.getenv('MINIO_ENDPOINT', 'http://localhost:9000'),
    'aws_access_key_id': os.getenv('MINIO_ACCESS_KEY', 'minioadmin'),
    'aws_secret_access_key': os.getenv('MINIO_SECRET_KEY', 'minioadmin'),
    'region_name': os.getenv('MINIO_REGION', 'us-east-1')
}
MINIO_BUCKET = os.getenv('MINIO_BUCKET', 'ariata')

def create_directory_structure():
    """Create the output directory structure."""
    dirs = [
        f"{OUTPUT_DIR}/audio",
        f"{OUTPUT_DIR}/streams",
        f"{OUTPUT_DIR}/metadata"
    ]
    for dir_path in dirs:
        Path(dir_path).mkdir(parents=True, exist_ok=True)
    logger.info(f"Created directory structure in {OUTPUT_DIR}")

def export_minio_audio():
    """Export all audio files from MinIO for the target date."""
    logger.info(f"Connecting to MinIO at {MINIO_CONFIG['endpoint_url']}")

    # Create S3 client for MinIO
    s3_client = boto3.client('s3', **MINIO_CONFIG)

    # Parse target date to construct the correct path
    # The actual storage pattern is: streams/ios_mic/{year}/{month}/{day}/{uuid}.m4a
    date_parts = TARGET_DATE.split('-')
    year, month, day = date_parts[0], date_parts[1], date_parts[2]

    # Use the correct prefix based on actual storage pattern
    prefix = f"streams/ios_mic/{year}/{month}/{day}/"

    logger.info(f"Searching for audio files with prefix: {prefix}")

    try:
        # First, list all buckets to verify connectivity
        try:
            buckets = s3_client.list_buckets()
            logger.info(f"Connected to MinIO. Available buckets: {[b['Name'] for b in buckets.get('Buckets', [])]}")
        except Exception as e:
            logger.warning(f"Could not list buckets: {e}")

        # List all objects with the prefix
        paginator = s3_client.get_paginator('list_objects_v2')
        page_iterator = paginator.paginate(
            Bucket=MINIO_BUCKET,
            Prefix=prefix
        )

        audio_count = 0
        all_objects_found = 0

        for page in page_iterator:
            if 'Contents' not in page:
                continue

            all_objects_found += len(page['Contents'])

            for obj in page['Contents']:
                key = obj['Key']

                # Since we're using date-based prefix, all files should be from target date
                # Just download any audio files (.m4a, .wav, .mp3)
                if key.endswith(('.m4a', '.wav', '.mp3', '.aac', '.mp4')):
                    # Download the file
                    filename = os.path.basename(key)
                    output_path = f"{OUTPUT_DIR}/audio/{filename}"

                    logger.info(f"Downloading {key} to {output_path}")
                    s3_client.download_file(MINIO_BUCKET, key, output_path)
                    audio_count += 1
                else:
                    logger.debug(f"Skipping non-audio file: {key}")

        logger.info(f"Found {all_objects_found} total objects with prefix {prefix}")
        logger.info(f"Downloaded {audio_count} audio files from MinIO")

        # If no audio files found, try to list what's actually in MinIO for debugging
        if audio_count == 0 and all_objects_found == 0:
            logger.warning("No files found with the expected prefix. Listing sample paths in MinIO for debugging...")

            # Try broader prefixes to see what's actually stored
            debug_prefixes = ["streams/", "ios/", ""]
            for debug_prefix in debug_prefixes:
                try:
                    response = s3_client.list_objects_v2(
                        Bucket=MINIO_BUCKET,
                        Prefix=debug_prefix,
                        MaxKeys=10
                    )
                    if 'Contents' in response:
                        logger.info(f"Sample paths with prefix '{debug_prefix}':")
                        for obj in response['Contents'][:5]:
                            logger.info(f"  - {obj['Key']}")
                        break
                except Exception as e:
                    logger.debug(f"Could not list with prefix '{debug_prefix}': {e}")

        return audio_count

    except ClientError as e:
        logger.error(f"Error accessing MinIO: {e}")
        return 0

def export_postgres_streams():
    """Export iOS stream data from PostgreSQL for the target date."""
    logger.info("Connecting to PostgreSQL")

    try:
        conn = psycopg2.connect(**DB_CONFIG)

        # iOS stream tables to export
        stream_tables = [
            'stream_ios_healthkit',
            'stream_ios_mic',
            'stream_ios_location'
        ]

        stream_counts = {}

        for table in stream_tables:
            cursor = conn.cursor(cursor_factory=RealDictCursor)

            # Query for data from target date
            query = f"""
            SELECT * FROM {table}
            WHERE DATE(timestamp) = %s
            ORDER BY timestamp
            """

            try:
                cursor.execute(query, (TARGET_DATE,))
                rows = cursor.fetchall()

                # Convert datetime objects to ISO format strings
                for row in rows:
                    for key, value in row.items():
                        if isinstance(value, datetime):
                            row[key] = value.isoformat()

                # Save to JSON file
                output_file = f"{OUTPUT_DIR}/streams/{table.replace('stream_', '')}.json"
                with open(output_file, 'w') as f:
                    json.dump(rows, f, indent=2, default=str)

                stream_counts[table] = len(rows)
                logger.info(f"Exported {len(rows)} records from {table}")

            except psycopg2.Error as e:
                logger.warning(f"Table {table} might not exist: {e}")
                stream_counts[table] = 0

            cursor.close()

        conn.close()
        return stream_counts

    except psycopg2.Error as e:
        logger.error(f"Database connection error: {e}")
        return {}

def export_metadata_tables():
    """Export metadata tables from PostgreSQL."""
    logger.info("Exporting metadata tables")

    try:
        conn = psycopg2.connect(**DB_CONFIG)

        # Metadata tables to export
        metadata_tables = [
            'sources',
            'streams',
            'source_configs',
            'stream_configs'
        ]

        metadata_counts = {}

        for table in metadata_tables:
            cursor = conn.cursor(cursor_factory=RealDictCursor)

            query = f"SELECT * FROM {table}"

            try:
                cursor.execute(query)
                rows = cursor.fetchall()

                # Convert datetime objects to ISO format strings
                for row in rows:
                    for key, value in row.items():
                        if isinstance(value, datetime):
                            row[key] = value.isoformat()

                # Save to JSON file
                output_file = f"{OUTPUT_DIR}/metadata/{table}.json"
                with open(output_file, 'w') as f:
                    json.dump(rows, f, indent=2, default=str)

                metadata_counts[table] = len(rows)
                logger.info(f"Exported {len(rows)} records from {table}")

            except psycopg2.Error as e:
                logger.warning(f"Table {table} might not exist: {e}")
                metadata_counts[table] = 0

            cursor.close()

        conn.close()
        return metadata_counts

    except psycopg2.Error as e:
        logger.error(f"Database connection error: {e}")
        return {}

def create_manifest(audio_count, stream_counts, metadata_counts):
    """Create a manifest file with export details."""
    manifest = {
        "export_date": datetime.now(timezone.utc).isoformat(),
        "target_date": TARGET_DATE,
        "data_summary": {
            "audio_files": audio_count,
            "streams": stream_counts,
            "metadata": metadata_counts
        },
        "environment": {
            "db_host": DB_CONFIG['host'],
            "db_name": DB_CONFIG['database'],
            "minio_endpoint": MINIO_CONFIG['endpoint_url'],
            "minio_bucket": MINIO_BUCKET
        }
    }

    manifest_path = f"{OUTPUT_DIR}/manifest.json"
    with open(manifest_path, 'w') as f:
        json.dump(manifest, f, indent=2)

    logger.info(f"Created manifest at {manifest_path}")
    return manifest

def main():
    """Main execution function."""
    logger.info(f"Starting iOS test data export for {TARGET_DATE}")

    # Create directory structure
    create_directory_structure()

    # Export MinIO audio files
    logger.info("Step 1: Exporting MinIO audio files...")
    audio_count = export_minio_audio()

    # Export PostgreSQL stream data
    logger.info("Step 2: Exporting PostgreSQL stream data...")
    stream_counts = export_postgres_streams()

    # Export metadata tables
    logger.info("Step 3: Exporting metadata tables...")
    metadata_counts = export_metadata_tables()

    # Create manifest
    logger.info("Step 4: Creating manifest...")
    manifest = create_manifest(audio_count, stream_counts, metadata_counts)

    # Print summary
    print("\n" + "="*50)
    print(f"✅ Export Complete!")
    print(f"📁 Output directory: {OUTPUT_DIR}")
    print(f"📅 Data from: {TARGET_DATE}")
    print("\n📊 Summary:")
    print(f"  - Audio files: {audio_count}")
    print(f"  - Stream records:")
    for table, count in stream_counts.items():
        print(f"    - {table}: {count} records")
    print(f"  - Metadata tables: {len(metadata_counts)} tables")
    print("="*50)

    # Optional: Create tarball (check for --archive flag or AUTO_ARCHIVE env var)
    create_archive = (
        '--archive' in sys.argv or
        os.getenv('AUTO_ARCHIVE', '').lower() in ['true', 'yes', '1']
    )

    if create_archive:
        import tarfile
        archive_name = f"{OUTPUT_DIR}.tar.gz"
        print(f"\n📦 Creating compressed archive: {archive_name}")
        with tarfile.open(archive_name, "w:gz") as tar:
            tar.add(OUTPUT_DIR, arcname=os.path.basename(OUTPUT_DIR))
        print(f"✅ Created archive: {archive_name}")
        print(f"📊 Archive size: {os.path.getsize(archive_name) / (1024*1024):.2f} MB")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        logger.info("Export interrupted by user")
        sys.exit(1)
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        sys.exit(1)