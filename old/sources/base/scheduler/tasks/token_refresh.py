"""Token refresh tasks for OAuth sources."""

import json
import logging
import importlib
from datetime import datetime, timedelta
from typing import Optional
from uuid import uuid4

from sqlalchemy import text
from croniter import croniter

from sources.base.scheduler.celery_app import app
from sources.base.storage.database import SyncSessionLocal as Session

logger = logging.getLogger(__name__)


def create_token_refresher(source_name: str, oauth_credentials: dict, source_config: dict, db):
    """Create a token refresher function for OAuth sources."""
    
    # Check if source_config is None
    if source_config is None:
        logger.warning(f"No source config provided for {source_name}, skipping token refresh")
        return None
    
    # Check if this is an OAuth source based on source_config
    if source_config.get('auth_type') != 'oauth2':
        logger.warning(f"Source {source_name} is not OAuth2, skipping token refresh")
        return None
    
    logger.info(f"Found source config for {source_name}: auth type = oauth2")
    
    # Dynamically import the auth module for this source
    try:
        # Construct auth module path based on naming convention
        # e.g., google -> sources.google.auth
        auth_module_path = f"sources.{source_name}.auth"
        logger.info(f"Trying to import auth module: {auth_module_path}")
        
        try:
            auth_module = importlib.import_module(auth_module_path)
        except ImportError as e:
            logger.info(f"Failed to import {auth_module_path}, trying subdirectories")
            # Try looking in subdirectories (e.g., google/calendar/auth.py)
            # For google source, check in calendar subdirectory
            if source_name == 'google':
                auth_module_path = 'sources.google.calendar.auth'
                logger.info(f"Trying Google Calendar auth module: {auth_module_path}")
                auth_module = importlib.import_module(auth_module_path)
            else:
                # Some sources might have auth at a different level
                # Try parent directory
                parts = source_path.split('/')
                if len(parts) > 1:
                    parent_path = '/'.join(parts[:-1])
                    auth_module_path = f"sources.{parent_path.replace('/', '.')}.auth"
                    auth_module = importlib.import_module(auth_module_path)
                else:
                    raise e
        
        # Check for refresh_token function or refresh_google_token (backward compat)
        refresh_func = None
        if hasattr(auth_module, 'refresh_token'):
            refresh_func = auth_module.refresh_token
        elif hasattr(auth_module, 'refresh_google_token'):
            refresh_func = auth_module.refresh_google_token
        else:
            logger.warning(f"No token refresh function found in {auth_module_path}")
            return None
            
        async def token_refresher():
            """Generic token refresher."""
            try:
                # Call source-specific refresh logic
                new_tokens = await refresh_func(oauth_credentials['oauth_refresh_token'])
                
                # Update tokens in sources table
                db.execute(
                    text("""
                        UPDATE sources 
                        SET oauth_access_token = :access_token,
                            oauth_refresh_token = :refresh_token,
                            oauth_expires_at = :expires_at,
                            updated_at = :updated_at
                        WHERE id = :source_id
                    """),
                    {
                        "source_id": oauth_credentials.get('source_id'),
                        "access_token": new_tokens["access_token"],
                        "refresh_token": new_tokens.get("refresh_token", oauth_credentials['oauth_refresh_token']),
                        "expires_at": datetime.utcnow() + timedelta(seconds=new_tokens.get("expires_in", 3600)),
                        "updated_at": datetime.utcnow()
                    }
                )
                db.commit()
                
                return new_tokens["access_token"]
            except Exception as e:
                raise Exception(f"Failed to refresh {source_name} token: {str(e)}")
        
        return token_refresher
        
    except ImportError as e:
        # Source doesn't have an auth module - that's okay for non-OAuth sources
        logger.debug(f"No auth module found for {source_name}: {e}")
        return None
    except Exception as e:
        logger.error(f"Error creating token refresher for {source_name}: {e}")
        return None


@app.task(name="refresh_expiring_tokens")
def refresh_expiring_tokens():
    """Proactively refresh tokens that are about to expire."""

    db = Session()
    activity_id = None
    try:
        # Create pipeline activity record
        activity_id = str(uuid4())
        db.execute(
            text("""
                INSERT INTO pipeline_activities 
                (id, activity_type, activity_name, source_name, status, started_at, created_at, updated_at)
                VALUES (:id, 'token_refresh', 'refresh_expiring_tokens', 'system', 'running', :started_at, :created_at, :updated_at)
            """),
            {
                "id": activity_id,
                "started_at": datetime.utcnow(),
                "created_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        # Find credentials that expire within the next hour
        expiry_threshold = datetime.utcnow() + timedelta(hours=1)

        # Find sources with tokens that need refreshing
        result = db.execute(
            text("""
                SELECT s.id, s.source_name, s.instance_name, s.oauth_access_token, 
                       s.oauth_refresh_token, s.oauth_expires_at, s.scopes
                FROM sources s
                WHERE s.oauth_expires_at IS NOT NULL
                AND s.oauth_expires_at < :expiry_threshold
                AND s.oauth_refresh_token IS NOT NULL
                AND s.status IN ('authenticated', 'active')
            """),
            {"expiry_threshold": expiry_threshold}
        ).fetchall()

        refreshed = []
        failed = []
        sources_checked = len(result)

        for row in result:
            source_dict = dict(row._mapping)
            try:
                # Skip non-OAuth sources (e.g., device sources)
                if not source_dict.get('oauth_refresh_token'):
                    continue

                # Create a token refresher and call it
                source_dict['source_id'] = source_dict['id']  # Add source_id for the refresher
                token_refresher = create_token_refresher(
                    source_dict['source_name'], 
                    source_dict, 
                    source_dict,  # Pass source_dict as the config too
                    db
                )
                
                if token_refresher:
                    # Run the token refresher
                    import asyncio
                    loop = asyncio.new_event_loop()
                    asyncio.set_event_loop(loop)
                    try:
                        new_token = loop.run_until_complete(token_refresher())
                        refreshed.append({
                            "source": source_dict['source_name'],
                            "instance": source_dict.get('instance_name', 'unknown')
                        })
                        logger.info(f"Successfully refreshed token for {source_dict['source_name']}")
                    finally:
                        loop.close()
                else:
                    logger.warning(
                        f"No token refresher available for source {source_dict['source_name']}"
                    )

            except Exception as e:
                failed.append({
                    "source": source_dict['source_name'],
                    "instance": source_dict.get('instance_name', 'unknown'),
                    "error": str(e)
                })

        # Update pipeline activity with results
        db.execute(
            text("""
                UPDATE pipeline_activities 
                SET status = :status,
                    completed_at = :completed_at,
                    records_processed = :records_processed,
                    activity_metadata = :metadata,
                    updated_at = :updated_at
                WHERE id = :id
            """),
            {
                "id": activity_id,
                "status": "completed" if len(failed) == 0 else "completed",
                "completed_at": datetime.utcnow(),
                "records_processed": sources_checked,
                "metadata": json.dumps({
                    "sources_checked": sources_checked,
                    "refreshed": len(refreshed),
                    "failed": len(failed),
                    "expiry_threshold": expiry_threshold.isoformat()
                }),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        return {
            "refreshed": len(refreshed),
            "failed": len(failed),
            "refreshed_sources": refreshed,
            "failed_sources": failed
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