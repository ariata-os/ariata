"""Startup initialization tasks for the scheduler service."""

import asyncio
import os
from botocore.exceptions import ClientError


async def ensure_minio_bucket():
    """Ensure the MinIO bucket exists for storing raw data."""
    try:
        # Import here to avoid circular dependencies and handle failures gracefully
        from sources.base.storage.minio import MinIOClient
        storage = MinIOClient()
    except ImportError as e:
        print(f"Warning: Could not import MinIOClient: {e}")
        print("Skipping MinIO bucket initialization")
        return
    except Exception as e:
        print(f"Warning: Could not initialize MinIOClient: {e}")
        print("Skipping MinIO bucket initialization")
        return
        
    bucket_name = "ariata"
    
    try:
        # Use context manager to create client
        async with storage.session.client(
            "s3",
            endpoint_url=storage.endpoint_url,
            aws_access_key_id=storage.access_key,
            aws_secret_access_key=storage.secret_key,
            use_ssl=os.getenv("MINIO_USE_SSL", "false").lower() == "true"
        ) as s3:
            try:
                # Check if bucket exists
                await s3.head_bucket(Bucket=bucket_name)
                print(f"✓ MinIO bucket '{bucket_name}' already exists")
            except ClientError as e:
                error_code = e.response.get("Error", {}).get("Code")
                if error_code == "404":
                    # Bucket doesn't exist, create it
                    try:
                        await s3.create_bucket(Bucket=bucket_name)
                        print(f"✓ Created MinIO bucket '{bucket_name}'")
                    except Exception as create_error:
                        print(f"✗ Failed to create bucket '{bucket_name}': {create_error}")
                        # Don't raise, just log the error
                        return
                else:
                    print(f"✗ Error checking bucket '{bucket_name}': {e}")
                    # Don't raise, just log the error
                    return
    except Exception as e:
        print(f"✗ Failed to initialize MinIO connection: {e}")
        print("This is not critical for service startup")
        return


async def initialize():
    """Run all startup initialization tasks."""
    print("Initializing scheduler service...")
    
    # Ensure MinIO is ready and bucket exists
    await ensure_minio_bucket()
    
    print("✓ Scheduler initialization complete")


def run_startup_tasks():
    """Run async startup tasks synchronously."""
    try:
        asyncio.run(initialize())
    except Exception as e:
        print(f"✗ Startup initialization failed: {e}")
        print("Service will continue to start despite initialization errors")
        # Don't crash the service, but log the error
        # In production, this should be sent to a monitoring service