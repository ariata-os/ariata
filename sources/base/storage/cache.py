"""Redis cache client for temporary storage and task coordination."""

import os
import json
from typing import Any, Optional, Dict, List
import redis.asyncio as redis
from datetime import timedelta


class CacheClient:
    """Redis client for caching and temporary data storage."""
    
    def __init__(self, redis_url: Optional[str] = None):
        """
        Initialize Redis cache client.
        
        Args:
            redis_url: Redis connection URL (defaults to env var or localhost)
        """
        self.redis_url = redis_url or os.getenv(
            "REDIS_URL", 
            "redis://localhost:6379/0"
        )
        self._client = None
    
    async def connect(self):
        """Establish connection to Redis."""
        if not self._client:
            self._client = await redis.from_url(
                self.redis_url,
                encoding="utf-8",
                decode_responses=True
            )
        return self._client
    
    async def disconnect(self):
        """Close Redis connection."""
        if self._client:
            await self._client.close()
            self._client = None
    
    async def get(self, key: str) -> Optional[str]:
        """
        Get value from cache.
        
        Args:
            key: Cache key
            
        Returns:
            Cached value or None
        """
        client = await self.connect()
        return await client.get(key)
    
    async def get_json(self, key: str) -> Optional[Dict[str, Any]]:
        """
        Get JSON value from cache.
        
        Args:
            key: Cache key
            
        Returns:
            Parsed JSON object or None
        """
        value = await self.get(key)
        if value:
            try:
                return json.loads(value)
            except json.JSONDecodeError:
                return None
        return None
    
    async def set(
        self, 
        key: str, 
        value: str, 
        ttl_seconds: Optional[int] = None
    ) -> bool:
        """
        Set value in cache.
        
        Args:
            key: Cache key
            value: Value to cache
            ttl_seconds: Optional TTL in seconds
            
        Returns:
            True if successful
        """
        client = await self.connect()
        if ttl_seconds:
            return await client.setex(key, ttl_seconds, value)
        return await client.set(key, value)
    
    async def set_json(
        self,
        key: str,
        value: Dict[str, Any],
        ttl_seconds: Optional[int] = None
    ) -> bool:
        """
        Set JSON value in cache.
        
        Args:
            key: Cache key
            value: Dictionary to cache as JSON
            ttl_seconds: Optional TTL in seconds
            
        Returns:
            True if successful
        """
        json_str = json.dumps(value)
        return await self.set(key, json_str, ttl_seconds)
    
    async def delete(self, key: str) -> bool:
        """
        Delete key from cache.
        
        Args:
            key: Cache key to delete
            
        Returns:
            True if key was deleted
        """
        client = await self.connect()
        return await client.delete(key) > 0
    
    async def exists(self, key: str) -> bool:
        """
        Check if key exists in cache.
        
        Args:
            key: Cache key
            
        Returns:
            True if key exists
        """
        client = await self.connect()
        return await client.exists(key) > 0
    
    async def expire(self, key: str, ttl_seconds: int) -> bool:
        """
        Set expiration on existing key.
        
        Args:
            key: Cache key
            ttl_seconds: TTL in seconds
            
        Returns:
            True if expiration was set
        """
        client = await self.connect()
        return await client.expire(key, ttl_seconds)
    
    async def lpush(self, key: str, *values: str) -> int:
        """
        Push values to list.
        
        Args:
            key: List key
            values: Values to push
            
        Returns:
            Length of list after push
        """
        client = await self.connect()
        return await client.lpush(key, *values)
    
    async def lrange(self, key: str, start: int = 0, end: int = -1) -> List[str]:
        """
        Get range of values from list.
        
        Args:
            key: List key
            start: Start index
            end: End index (-1 for all)
            
        Returns:
            List of values
        """
        client = await self.connect()
        return await client.lrange(key, start, end)
    
    async def hset(self, key: str, field: str, value: str) -> int:
        """
        Set hash field value.
        
        Args:
            key: Hash key
            field: Field name
            value: Field value
            
        Returns:
            Number of fields added
        """
        client = await self.connect()
        return await client.hset(key, field, value)
    
    async def hget(self, key: str, field: str) -> Optional[str]:
        """
        Get hash field value.
        
        Args:
            key: Hash key
            field: Field name
            
        Returns:
            Field value or None
        """
        client = await self.connect()
        return await client.hget(key, field)
    
    async def hgetall(self, key: str) -> Dict[str, str]:
        """
        Get all hash fields and values.
        
        Args:
            key: Hash key
            
        Returns:
            Dictionary of field:value pairs
        """
        client = await self.connect()
        return await client.hgetall(key)
    
    async def publish(self, channel: str, message: str) -> int:
        """
        Publish message to channel.
        
        Args:
            channel: Channel name
            message: Message to publish
            
        Returns:
            Number of subscribers that received the message
        """
        client = await self.connect()
        return await client.publish(channel, message)
    
    async def lock(
        self,
        key: str,
        timeout: int = 10,
        blocking_timeout: Optional[int] = None
    ):
        """
        Acquire a distributed lock.
        
        Args:
            key: Lock key
            timeout: Lock timeout in seconds
            blocking_timeout: Max time to wait for lock
            
        Returns:
            Lock object (use as context manager)
        """
        client = await self.connect()
        return client.lock(
            key,
            timeout=timeout,
            blocking_timeout=blocking_timeout
        )


# Create singleton instance
cache = CacheClient()