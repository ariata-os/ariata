"""Database connection and session management."""

import os
from typing import AsyncGenerator, Generator
from sqlalchemy import create_engine
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession
from sqlalchemy.orm import sessionmaker, Session


class DatabaseManager:
    """Manages database connections and sessions."""
    
    def __init__(self):
        """Initialize database manager with environment configuration."""
        # Get database URL from environment
        self.database_url = os.getenv(
            "DATABASE_URL", 
            "postgresql+asyncpg://postgres:postgres@localhost:5432/ariata"
        )
        
        # Ensure we're using asyncpg driver for async operations
        if self.database_url.startswith("postgresql://"):
            self.async_url = self.database_url.replace(
                "postgresql://", "postgresql+asyncpg://", 1
            )
        else:
            self.async_url = self.database_url
        
        # Create sync URL for Celery tasks
        self.sync_url = self.async_url.replace(
            "postgresql+asyncpg://", "postgresql://", 1
        )
        
        # Create engines
        self._async_engine = None
        self._sync_engine = None
        
        # Create session factories
        self._async_session_factory = None
        self._sync_session_factory = None
    
    @property
    def async_engine(self):
        """Get or create async engine."""
        if self._async_engine is None:
            self._async_engine = create_async_engine(
                self.async_url, 
                echo=False,
                pool_pre_ping=True
            )
        return self._async_engine
    
    @property
    def sync_engine(self):
        """Get or create sync engine."""
        if self._sync_engine is None:
            self._sync_engine = create_engine(
                self.sync_url,
                echo=False,
                pool_pre_ping=True
            )
        return self._sync_engine
    
    @property
    def async_session_factory(self):
        """Get or create async session factory."""
        if self._async_session_factory is None:
            self._async_session_factory = sessionmaker(
                self.async_engine,
                class_=AsyncSession,
                expire_on_commit=False
            )
        return self._async_session_factory
    
    @property
    def sync_session_factory(self):
        """Get or create sync session factory."""
        if self._sync_session_factory is None:
            self._sync_session_factory = sessionmaker(
                self.sync_engine,
                class_=Session,
                expire_on_commit=False
            )
        return self._sync_session_factory
    
    async def get_async_session(self) -> AsyncGenerator[AsyncSession, None]:
        """
        Create a new async database session.
        
        Yields:
            AsyncSession instance
        """
        async with self.async_session_factory() as session:
            try:
                yield session
                await session.commit()
            except Exception:
                await session.rollback()
                raise
            finally:
                await session.close()
    
    def get_sync_session(self) -> Generator[Session, None, None]:
        """
        Create a new sync database session for Celery tasks.
        
        Yields:
            Session instance
        """
        session = self.sync_session_factory()
        try:
            yield session
            session.commit()
        except Exception:
            session.rollback()
            raise
        finally:
            session.close()
    
    async def close_async(self):
        """Close async engine connections."""
        if self._async_engine:
            await self._async_engine.dispose()
            self._async_engine = None
            self._async_session_factory = None
    
    def close_sync(self):
        """Close sync engine connections."""
        if self._sync_engine:
            self._sync_engine.dispose()
            self._sync_engine = None
            self._sync_session_factory = None


# Create singleton instance
db_manager = DatabaseManager()

# Export convenience functions for backward compatibility
async_engine = db_manager.async_engine
sync_engine = db_manager.sync_engine
AsyncSessionLocal = db_manager.async_session_factory
SyncSessionLocal = db_manager.sync_session_factory
get_async_db = db_manager.get_async_session
get_sync_db = db_manager.get_sync_session