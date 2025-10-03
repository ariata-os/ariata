"""Notion API client for fetching pages and databases."""

from typing import Dict, Any, List, Optional
from datetime import datetime
from sources.base.clients.base import BaseAPIClient, RateLimiter


class NotionClient(BaseAPIClient):
    """Client for interacting with Notion API."""
    
    BASE_URL = "https://api.notion.com/v1"
    API_VERSION = "2022-06-28"
    RATE_LIMIT_DELAY = 0.34  # ~3 requests per second
    
    def __init__(self, access_token: str):
        """Initialize Notion client with access token."""
        super().__init__(access_token=access_token)
        # Add Notion-specific headers
        self.headers["Notion-Version"] = self.API_VERSION
        self.headers["Content-Type"] = "application/json"
    
    def _create_rate_limiter(self) -> RateLimiter:
        """
        Create Notion API rate limiter.
        
        Notion API has a rate limit of 3 requests per second.
        
        Returns:
            Configured RateLimiter instance
        """
        return RateLimiter(requests_per_second=3.0)
    
    async def search_pages(
        self,
        query: Optional[str] = None,
        filter_type: Optional[str] = None,  # 'page' or 'database'
        cursor: Optional[str] = None,
        page_size: int = 100,
        sort_by: str = "last_edited_time",
        sort_direction: str = "descending"
    ) -> Dict[str, Any]:
        """
        Search for pages and databases in the workspace.
        
        Args:
            query: Text query to search for
            filter_type: Filter by 'page' or 'database'
            cursor: Pagination cursor
            page_size: Number of results per page (max 100)
            sort_by: Sort field ('last_edited_time' or 'created_time')
            sort_direction: Sort direction ('ascending' or 'descending')
        
        Returns:
            Dict containing results and pagination info
        """
        payload = {
            "page_size": min(page_size, 100),
            "sort": {
                "timestamp": sort_by,
                "direction": sort_direction
            }
        }
        
        if query:
            payload["query"] = query
        
        if filter_type:
            payload["filter"] = {
                "property": "object",
                "value": filter_type
            }
        
        if cursor:
            payload["start_cursor"] = cursor
        
        response = await self._make_request("POST", "/search", json=payload)
        return response.json()
    
    async def get_page(self, page_id: str) -> Dict[str, Any]:
        """
        Get a specific page by ID.
        
        Args:
            page_id: The Notion page ID
        
        Returns:
            Page object with metadata
        """
        response = await self._make_request("GET", f"/pages/{page_id}")
        return response.json()
    
    async def get_page_content(self, page_id: str, cursor: Optional[str] = None) -> Dict[str, Any]:
        """
        Get the content blocks of a page.
        
        Args:
            page_id: The Notion page ID
            cursor: Pagination cursor for blocks
        
        Returns:
            Dict containing blocks and pagination info
        """
        params = {"page_size": 100}
        if cursor:
            params["start_cursor"] = cursor
        
        response = await self._make_request("GET", f"/blocks/{page_id}/children", params=params)
        return response.json()
    
    async def get_database(self, database_id: str) -> Dict[str, Any]:
        """
        Get a specific database by ID.
        
        Args:
            database_id: The Notion database ID
        
        Returns:
            Database object with schema
        """
        response = await self._make_request("GET", f"/databases/{database_id}")
        return response.json()
    
    async def query_database(
        self,
        database_id: str,
        filter_obj: Optional[Dict] = None,
        sorts: Optional[List[Dict]] = None,
        cursor: Optional[str] = None,
        page_size: int = 100
    ) -> Dict[str, Any]:
        """
        Query a database for pages.
        
        Args:
            database_id: The Notion database ID
            filter_obj: Filter conditions
            sorts: Sort conditions
            cursor: Pagination cursor
            page_size: Number of results per page
        
        Returns:
            Dict containing database pages and pagination info
        """
        payload = {"page_size": min(page_size, 100)}
        
        if filter_obj:
            payload["filter"] = filter_obj
        
        if sorts:
            payload["sorts"] = sorts
        
        if cursor:
            payload["start_cursor"] = cursor
        
        response = await self._make_request("POST", f"/databases/{database_id}/query", json=payload)
        return response.json()
    
    async def get_user(self, user_id: str) -> Dict[str, Any]:
        """
        Get user information.
        
        Args:
            user_id: The Notion user ID
        
        Returns:
            User object
        """
        response = await self._make_request("GET", f"/users/{user_id}")
        return response.json()
    
    async def get_all_pages(
        self,
        filter_type: Optional[str] = None,
        since: Optional[datetime] = None,
        batch_size: int = 100,
        progress_callback: Optional[callable] = None
    ) -> List[Dict[str, Any]]:
        """
        Get all pages from the workspace.
        
        Args:
            filter_type: Filter by 'page' or 'database'
            since: Only get pages modified after this time
            batch_size: Number of results per API call
        
        Returns:
            List of all pages matching criteria
        """
        all_pages = []
        cursor = None
        has_more = True
        
        # Build initial sort direction based on whether we have a since filter
        sort_by = "last_edited_time"
        sort_direction = "ascending" if since else "descending"
        
        while has_more:
            result = await self.search_pages(
                filter_type=filter_type,
                cursor=cursor,
                page_size=batch_size,
                sort_by=sort_by,
                sort_direction=sort_direction
            )
            
            pages = result.get("results", [])
            
            # Filter by since date if provided
            if since:
                pages = [
                    page for page in pages
                    if datetime.fromisoformat(
                        page.get("last_edited_time", "").replace("Z", "+00:00")
                    ) > since
                ]
            
            all_pages.extend(pages)
            
            # Call progress callback if provided
            if progress_callback:
                progress_callback(len(all_pages), len(pages))
            
            cursor = result.get("next_cursor")
            has_more = result.get("has_more", False) and cursor
            
            # If filtering by date and sort ascending, stop when we hit older pages
            if since and sort_direction == "ascending" and pages:
                last_page_time = datetime.fromisoformat(
                    pages[-1].get("last_edited_time", "").replace("Z", "+00:00")
                )
                if last_page_time <= since:
                    has_more = False
        
        return all_pages
    
    async def get_full_page_with_content(self, page_id: str) -> Dict[str, Any]:
        """
        Get a page with all its content blocks.
        
        Args:
            page_id: The Notion page ID
        
        Returns:
            Page object with all content blocks included
        """
        # Get page metadata
        page = await self.get_page(page_id)
        
        # Get all content blocks
        all_blocks = []
        cursor = None
        has_more = True
        
        while has_more:
            result = await self.get_page_content(page_id, cursor)
            blocks = result.get("results", [])
            all_blocks.extend(blocks)
            
            cursor = result.get("next_cursor")
            has_more = result.get("has_more", False) and cursor
        
        # Add blocks to page object
        page["blocks"] = all_blocks
        
        return page