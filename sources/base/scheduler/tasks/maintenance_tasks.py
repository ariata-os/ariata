"""Maintenance tasks for the scheduler."""

import json
import logging
from datetime import datetime, timedelta
from uuid import uuid4

from sqlalchemy import text
from croniter import croniter

from sources.base.scheduler.celery_app import app
from sources.base.storage.database import SyncSessionLocal as Session

logger = logging.getLogger(__name__)


@app.task(name="check_scheduled_syncs")
def check_scheduled_syncs():
    """Check for streams that need to be synced based on their schedule."""
    
    db = Session()
    activity_id = None
    try:
        # Create pipeline activity record
        activity_id = str(uuid4())
        db.execute(
            text("""
                INSERT INTO pipeline_activities 
                (id, activity_type, activity_name, source_name, status, started_at, created_at, updated_at)
                VALUES (:id, 'scheduled_check', 'check_scheduled_syncs', 'system', 'running', :started_at, :created_at, :updated_at)
            """),
            {
                "id": activity_id,
                "started_at": datetime.utcnow(),
                "created_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        # Get all active stream instances with cron schedules from cloud sources only
        # Device sources (mac, ios) push their own data
        # We need to query stream instances (streams table), not configs, to get the correct IDs
        result = db.execute(
            text("""
                SELECT s.id, stc.stream_name, stc.source_name, stc.cron_schedule, 
                       stc.last_ingestion_at, src.platform, src.auth_type
                FROM streams s
                JOIN stream_configs stc ON s.stream_config_id = stc.id
                JOIN sources source ON s.source_id = source.id
                JOIN source_configs src ON stc.source_name = src.name
                WHERE source.status = 'active'
                AND stc.status = 'active'
                AND stc.ingestion_type = 'pull'
                AND stc.cron_schedule IS NOT NULL
                AND src.platform = 'cloud'
            """)
        ).fetchall()

        triggered = []
        now = datetime.utcnow()

        for row in result:
            stream = dict(row._mapping)

            # Since we're now querying stream instances directly with active sources joined,
            # we don't need to check for source existence again
            # The sync_stream task will handle OAuth authentication checks

            # Check if sync is due based on cron schedule
            if should_sync(stream, now):
                # Import sync_stream from sync_sources to avoid circular import
                from .sync_sources import sync_stream
                # Trigger sync for this stream
                sync_stream.delay(str(stream['id']))
                triggered.append({
                    "stream_id": str(stream['id']),
                    "stream_name": stream['stream_name'],
                    "source": stream['source_name']
                })

        # Update pipeline activity with results
        db.execute(
            text("""
                UPDATE pipeline_activities 
                SET status = 'completed',
                    completed_at = :completed_at,
                    records_processed = :records_processed,
                    activity_metadata = :metadata,
                    updated_at = :updated_at
                WHERE id = :id
            """),
            {
                "id": activity_id,
                "completed_at": datetime.utcnow(),
                "records_processed": len(triggered),
                "metadata": json.dumps({
                    "streams_checked": len(result),
                    "streams_triggered": len(triggered),
                    "triggered": triggered
                }),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        return {
            "checked": len(result),
            "triggered": len(triggered),
            "streams": triggered
        }
    except Exception as e:
        # Log failure if activity was created
        if activity_id:
            db.execute(
                text("""
                    UPDATE pipeline_activities 
                    SET status = 'failed',
                        completed_at = :completed_at,
                        error_message = :error_message,
                        updated_at = :updated_at
                    WHERE id = :id
                """),
                {
                    "id": activity_id,
                    "completed_at": datetime.utcnow(),
                    "error_message": str(e)[:1000],
                    "updated_at": datetime.utcnow()
                }
            )
            db.commit()
        raise
    finally:
        db.close()


def should_sync(stream: dict, now: datetime) -> bool:
    """Check if a stream should sync based on its schedule."""
    cron_schedule = stream.get('cron_schedule')
    if not cron_schedule:
        return False

    # Parse cron expression
    try:
        # Use last_ingestion_at for streams
        last_sync = stream.get('last_ingestion_at')
        if last_sync is None:
            # If never synced, sync now
            return True

        # Ensure both datetimes are timezone-aware or both are naive
        if last_sync.tzinfo is None:
            # If last_sync is naive, assume it's UTC
            from datetime import timezone
            last_sync = last_sync.replace(tzinfo=timezone.utc)
        if now.tzinfo is None:
            # If now is naive, assume it's UTC
            from datetime import timezone
            now = now.replace(tzinfo=timezone.utc)

        cron = croniter(cron_schedule, last_sync)
        next_run = cron.get_next(datetime)
        return next_run <= now
    except Exception as e:
        logger.error(
            f"Invalid cron expression '{cron_schedule}' for stream {stream.get('stream_name')}: {e}")
        return False


@app.task(name="cleanup_old_runs")
def cleanup_old_runs(days_to_keep: int = 30):
    """Clean up old ingestion runs to prevent table bloat."""

    db = Session()
    try:
        cutoff_date = datetime.utcnow() - timedelta(days=days_to_keep)

        # Delete old runs
        result = db.execute(
            text("""
                DELETE FROM pipeline_activities 
                WHERE activity_type = 'ingestion' 
                AND started_at < :cutoff_date
                RETURNING id
            """),
            {"cutoff_date": cutoff_date}
        )

        deleted_count = result.rowcount
        db.commit()

        return {
            "deleted": deleted_count,
            "cutoff_date": cutoff_date.isoformat()
        }
    finally:
        db.close()