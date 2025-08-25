"""
Base API Client for Jaces
==========================

This module provides the abstract base class for all API clients.
Handles common functionality like authentication, rate limiting, 
token refresh, pagination, and error handling.
"""

from abc import ABC, abstractmethod
from typing import Any, Dict, Optional, Callable, AsyncIterator, List
from datetime import datetime, timedelta
import asyncio
import httpx
import time
import logging

logger = logging.getLogger(__name__)


class RateLimiter:
    """
    Token bucket rate limiter for API requests.
    
    Ensures we don't exceed API rate limits by controlling request timing.
    """
    
    def __init__(self, requests_per_second: float):
        """
        Initialize rate limiter.
        
        Args:
            requests_per_second: Maximum requests allowed per second
        """
        self.requests_per_second = requests_per_second
        self.min_interval = 1.0 / requests_per_second if requests_per_second > 0 else 0
        self.last_request_time = 0
        self._lock = asyncio.Lock()
    
    async def acquire(self):
        """Wait if necessary to respect rate limit."""
        async with self._lock:
            current_time = time.time()
            time_since_last = current_time - self.last_request_time
            
            if time_since_last < self.min_interval:
                sleep_time = self.min_interval - time_since_last
                await asyncio.sleep(sleep_time)
            
            self.last_request_time = time.time()


class BaseAPIClient(ABC):
    """
    Abstract base class for all API clients.
    
    Provides common functionality for OAuth APIs including:
    - Bearer token authentication
    - Automatic token refresh on 401
    - Rate limiting
    - Retry logic with exponential backoff
    - Pagination helpers
    - Consistent error handling
    """
    
    # Override these in subclasses
    BASE_URL: str = None
    DEFAULT_TIMEOUT: int = 30
    MAX_RETRIES: int = 3
    RETRY_STATUS_CODES = {429, 500, 502, 503, 504}  # Retry on these status codes
    
    def __init__(
        self, 
        access_token: str, 
        token_refresher: Optional[Callable] = None,
        timeout: Optional[int] = None
    ):
        """
        Initialize API client.
        
        Args:
            access_token: OAuth access token
            token_refresher: Optional callback to refresh expired tokens
            timeout: Request timeout in seconds (uses DEFAULT_TIMEOUT if not specified)
        """
        if not self.BASE_URL:
            raise ValueError(f"{self.__class__.__name__} must define BASE_URL")
        
        self.access_token = access_token
        self.token_refresher = token_refresher
        self.timeout = timeout or self.DEFAULT_TIMEOUT
        self.rate_limiter = self._create_rate_limiter()
        self._session: Optional[httpx.AsyncClient] = None
        self._headers_cache = None
    
    @property
    def headers(self) -> Dict[str, str]:
        """Get current headers with caching."""
        if self._headers_cache is None:
            self._headers_cache = self._build_headers()
        return self._headers_cache
    
    def _build_headers(self) -> Dict[str, str]:
        """
        Build request headers.
        
        Override in subclasses to add API-specific headers.
        
        Returns:
            Dictionary of HTTP headers
        """
        return {
            "Authorization": f"Bearer {self.access_token}",
            "Accept": "application/json",
            "Content-Type": "application/json"
        }
    
    def _update_token(self, new_token: str):
        """Update access token and clear headers cache."""
        self.access_token = new_token
        self._headers_cache = None  # Force rebuild on next access
    
    @abstractmethod
    def _create_rate_limiter(self) -> RateLimiter:
        """
        Create API-specific rate limiter.
        
        Must be implemented by subclasses to define API rate limits.
        
        Returns:
            Configured RateLimiter instance
        """
        pass
    
    async def _get_session(self) -> httpx.AsyncClient:
        """Get or create HTTP session."""
        if self._session is None or self._session.is_closed:
            self._session = httpx.AsyncClient(
                timeout=httpx.Timeout(self.timeout),
                follow_redirects=True
            )
        return self._session
    
    async def close(self):
        """Close HTTP session."""
        if self._session:
            await self._session.aclose()
            self._session = None
    
    async def __aenter__(self):
        """Async context manager entry."""
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()
    
    async def _execute_request(
        self,
        method: str,
        url: str,
        retry_on_401: bool = True,
        retry_count: int = 0,
        **kwargs
    ) -> httpx.Response:
        """
        Execute HTTP request with retry logic.
        
        Args:
            method: HTTP method (GET, POST, etc.)
            url: Full URL to request
            retry_on_401: Whether to refresh token and retry on 401
            retry_count: Current retry attempt number
            **kwargs: Additional arguments for httpx request
            
        Returns:
            HTTP response
            
        Raises:
            httpx.HTTPStatusError: On non-retryable HTTP errors
        """
        session = await self._get_session()
        
        # Apply rate limiting
        await self.rate_limiter.acquire()
        
        # Make the request
        response = await session.request(
            method=method,
            url=url,
            headers=self.headers,
            **kwargs
        )
        
        # Handle 401 Unauthorized
        if response.status_code == 401 and retry_on_401 and self.token_refresher:
            try:
                logger.info("Token expired, attempting refresh...")
                new_token = await self.token_refresher()
                if new_token:
                    self._update_token(new_token)
                    # Retry with new token (but don't retry 401 again)
                    return await self._execute_request(
                        method, url, retry_on_401=False, retry_count=0, **kwargs
                    )
            except Exception as e:
                logger.error(f"Token refresh failed: {e}")
        
        # Handle retryable status codes
        if response.status_code in self.RETRY_STATUS_CODES and retry_count < self.MAX_RETRIES:
            # Exponential backoff: 1s, 2s, 4s
            wait_time = 2 ** retry_count
            logger.warning(f"Got {response.status_code}, retrying in {wait_time}s...")
            await asyncio.sleep(wait_time)
            return await self._execute_request(
                method, url, retry_on_401, retry_count + 1, **kwargs
            )
        
        return response
    
    async def _make_request(
        self,
        method: str,
        endpoint: str,
        params: Optional[Dict] = None,
        json: Optional[Dict] = None,
        data: Optional[Any] = None,
        **kwargs
    ) -> httpx.Response:
        """
        Make an API request.
        
        Args:
            method: HTTP method
            endpoint: API endpoint (appended to BASE_URL)
            params: Query parameters
            json: JSON body data
            data: Form data
            **kwargs: Additional httpx arguments
            
        Returns:
            HTTP response
            
        Raises:
            httpx.HTTPStatusError: On HTTP errors
        """
        # Build full URL
        url = f"{self.BASE_URL}{endpoint}"
        
        # Prepare request kwargs
        request_kwargs = {
            "params": params,
            "json": json,
            "data": data,
            **kwargs
        }
        
        # Remove None values
        request_kwargs = {k: v for k, v in request_kwargs.items() if v is not None}
        
        # Execute request with retries
        response = await self._execute_request(method, url, **request_kwargs)
        
        # Raise for non-successful status codes
        response.raise_for_status()
        
        return response
    
    async def get(self, endpoint: str, **kwargs) -> Dict[str, Any]:
        """
        Make a GET request.
        
        Args:
            endpoint: API endpoint
            **kwargs: Additional request parameters
            
        Returns:
            JSON response as dictionary
        """
        response = await self._make_request("GET", endpoint, **kwargs)
        return response.json()
    
    async def post(self, endpoint: str, **kwargs) -> Dict[str, Any]:
        """
        Make a POST request.
        
        Args:
            endpoint: API endpoint
            **kwargs: Additional request parameters
            
        Returns:
            JSON response as dictionary
        """
        response = await self._make_request("POST", endpoint, **kwargs)
        return response.json()
    
    async def put(self, endpoint: str, **kwargs) -> Dict[str, Any]:
        """
        Make a PUT request.
        
        Args:
            endpoint: API endpoint
            **kwargs: Additional request parameters
            
        Returns:
            JSON response as dictionary
        """
        response = await self._make_request("PUT", endpoint, **kwargs)
        return response.json()
    
    async def delete(self, endpoint: str, **kwargs) -> Dict[str, Any]:
        """
        Make a DELETE request.
        
        Args:
            endpoint: API endpoint
            **kwargs: Additional request parameters
            
        Returns:
            JSON response as dictionary
        """
        response = await self._make_request("DELETE", endpoint, **kwargs)
        if response.content:
            return response.json()
        return {"success": True}
    
    async def paginate(
        self,
        endpoint: str,
        method: str = "GET",
        params: Optional[Dict] = None,
        page_size: int = 100,
        max_pages: Optional[int] = None
    ) -> AsyncIterator[List[Dict]]:
        """
        Generic pagination helper.
        
        Override in subclasses for API-specific pagination.
        
        Args:
            endpoint: API endpoint
            method: HTTP method
            params: Initial query parameters
            page_size: Number of items per page
            max_pages: Maximum number of pages to fetch
            
        Yields:
            Pages of results as lists
        """
        params = params or {}
        params["limit"] = page_size
        page = 1
        
        while max_pages is None or page <= max_pages:
            params["page"] = page
            
            try:
                if method == "GET":
                    response = await self.get(endpoint, params=params)
                else:
                    response = await self.post(endpoint, json=params)
                
                # Assume response has 'data' field with results
                # Override for different response structures
                if isinstance(response, dict) and "data" in response:
                    data = response["data"]
                    if not data:
                        break
                    yield data
                else:
                    yield response
                    break
                
                page += 1
                
            except httpx.HTTPStatusError as e:
                if e.response.status_code == 404:
                    # No more pages
                    break
                raise