"""Celery application configuration."""

import os
from celery import Celery
from celery.schedules import crontab
from celery.signals import worker_ready

# Redis connection from environment variable
REDIS_URL = os.getenv('REDIS_URL', 'redis://localhost:6379/0')

# Create Celery app
app = Celery(
    "ariata_scheduler",
    broker=REDIS_URL,
    backend=REDIS_URL,
    include=[
        "sources.base.scheduler.tasks.sync_sources",
        "sources.base.scheduler.tasks.token_refresh",
        "sources.base.scheduler.tasks.maintenance_tasks",
        "sources.base.scheduler.tasks.process_streams"
    ]
)

# Celery configuration with hardcoded values
app.conf.update(
    task_serializer="json",
    accept_content=["json"],
    result_serializer="json",
    timezone="UTC",
    enable_utc=True,
    task_track_started=True,
    task_time_limit=300,  # 5 minutes
    task_soft_time_limit=240,  # 4 minutes (warning before hard timeout)
    worker_prefetch_multiplier=1,
    worker_max_tasks_per_child=1000,
    task_routes={
        'process_stream_batch': {'queue': 'priority'},
        'check_scheduled_syncs': {'queue': 'celery'},
        'sync_source': {'queue': 'celery'},
        'refresh_expiring_tokens': {'queue': 'celery'},
        'cleanup_old_runs': {'queue': 'celery'}
    }
)

# Beat schedule for periodic tasks
app.conf.beat_schedule = {
    # Run source syncs every minute to check for scheduled syncs
    "check-scheduled-syncs": {
        "task": "check_scheduled_syncs",
        "schedule": crontab(minute="*"),
    },

    # Daily cleanup of old ingestion runs
    "cleanup-old-runs": {
        "task": "cleanup_old_runs",
        "schedule": crontab(hour=2, minute=0),  # 2 AM UTC
    },

    # Refresh tokens proactively (every 30 minutes)
    "refresh-expiring-tokens": {
        "task": "refresh_expiring_tokens",
        "schedule": crontab(minute="*/30"),
    },
}


@worker_ready.connect
def initialize_worker(sender=None, **kwargs):
    """Initialize worker on startup."""
    from .startup import run_startup_tasks
    run_startup_tasks()


if __name__ == "__main__":
    app.start()
