"""Google Calendar OAuth authentication utilities."""

import os
import httpx
from datetime import datetime, timedelta
from typing import Dict, Any


async def refresh_google_token(refresh_token: str) -> Dict[str, Any]:
    """
    Refresh Google OAuth token using the auth proxy.
    
    Args:
        refresh_token: The refresh token to use
        
    Returns:
        Dict containing new access_token, optional refresh_token, and expires_in
    """
    auth_proxy_url = os.getenv("AUTH_PROXY_URL", "https://auth.ariata.com")
    
    async with httpx.AsyncClient() as client:
        try:
            response = await client.post(
                f"{auth_proxy_url}/google/refresh",
                json={
                    "refresh_token": refresh_token
                },
                timeout=30.0
            )
            response.raise_for_status()
            
            return response.json()
            
        except httpx.HTTPStatusError as e:
            error_body = ""
            try:
                error_body = e.response.text
            except:
                pass
            
            if e.response.status_code == 401:
                raise Exception(f"Refresh token is invalid or expired. Response: {error_body}")
            elif e.response.status_code == 400:
                raise Exception(f"Bad refresh token request. Response: {error_body}")
            else:
                raise Exception(f"Failed to refresh token: {e.response.status_code}. Response: {error_body}")
        except Exception as e:
            raise Exception(f"Failed to refresh token: {str(e)}")