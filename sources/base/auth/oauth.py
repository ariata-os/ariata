"""OAuth authentication handler for various services."""

from typing import Dict, Any, Optional, List
from datetime import datetime, timedelta
import aiohttp
import urllib.parse
import secrets
from abc import ABC, abstractmethod


class OAuthHandler(ABC):
    """Base OAuth 2.0 handler for source authentication."""
    
    def __init__(
        self,
        client_id: str,
        client_secret: str,
        redirect_uri: str,
        auth_url: str,
        token_url: str,
        scopes: List[str]
    ):
        """
        Initialize OAuth handler.
        
        Args:
            client_id: OAuth client ID
            client_secret: OAuth client secret
            redirect_uri: Redirect URI for OAuth flow
            auth_url: Authorization endpoint URL
            token_url: Token endpoint URL
            scopes: List of required scopes
        """
        self.client_id = client_id
        self.client_secret = client_secret
        self.redirect_uri = redirect_uri
        self.auth_url = auth_url
        self.token_url = token_url
        self.scopes = scopes
    
    def generate_state(self) -> str:
        """
        Generate secure state parameter for OAuth flow.
        
        Returns:
            Random state string
        """
        return secrets.token_urlsafe(32)
    
    def get_authorization_url(
        self,
        state: str,
        additional_params: Optional[Dict[str, str]] = None
    ) -> str:
        """
        Build OAuth authorization URL.
        
        Args:
            state: State parameter for security
            additional_params: Optional additional query parameters
            
        Returns:
            Full authorization URL
        """
        params = {
            'client_id': self.client_id,
            'redirect_uri': self.redirect_uri,
            'response_type': 'code',
            'scope': ' '.join(self.scopes),
            'state': state,
            'access_type': 'offline',  # Request refresh token
            'prompt': 'consent'  # Force consent to get refresh token
        }
        
        if additional_params:
            params.update(additional_params)
        
        query_string = urllib.parse.urlencode(params)
        return f"{self.auth_url}?{query_string}"
    
    async def exchange_code_for_token(
        self,
        code: str
    ) -> Dict[str, Any]:
        """
        Exchange authorization code for access token.
        
        Args:
            code: Authorization code from OAuth flow
            
        Returns:
            Token response dictionary
        """
        data = {
            'client_id': self.client_id,
            'client_secret': self.client_secret,
            'code': code,
            'redirect_uri': self.redirect_uri,
            'grant_type': 'authorization_code'
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(self.token_url, data=data) as response:
                if response.status != 200:
                    error_text = await response.text()
                    raise Exception(f"Token exchange failed: {error_text}")
                
                token_data = await response.json()
                
                # Calculate expiry time
                if 'expires_in' in token_data:
                    token_data['expires_at'] = (
                        datetime.utcnow() + 
                        timedelta(seconds=token_data['expires_in'])
                    ).isoformat()
                
                return token_data
    
    async def refresh_access_token(
        self,
        refresh_token: str
    ) -> Dict[str, Any]:
        """
        Refresh an expired access token.
        
        Args:
            refresh_token: Refresh token
            
        Returns:
            New token response dictionary
        """
        data = {
            'client_id': self.client_id,
            'client_secret': self.client_secret,
            'refresh_token': refresh_token,
            'grant_type': 'refresh_token'
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(self.token_url, data=data) as response:
                if response.status != 200:
                    error_text = await response.text()
                    raise Exception(f"Token refresh failed: {error_text}")
                
                token_data = await response.json()
                
                # Calculate expiry time
                if 'expires_in' in token_data:
                    token_data['expires_at'] = (
                        datetime.utcnow() + 
                        timedelta(seconds=token_data['expires_in'])
                    ).isoformat()
                
                # Preserve refresh token if not returned
                if 'refresh_token' not in token_data:
                    token_data['refresh_token'] = refresh_token
                
                return token_data
    
    def is_token_expired(
        self,
        expires_at: str,
        buffer_seconds: int = 300
    ) -> bool:
        """
        Check if token is expired or about to expire.
        
        Args:
            expires_at: Token expiry time as ISO string
            buffer_seconds: Buffer time before actual expiry
            
        Returns:
            True if token is expired or expiring soon
        """
        expiry = datetime.fromisoformat(expires_at)
        buffer_time = datetime.utcnow() + timedelta(seconds=buffer_seconds)
        return buffer_time >= expiry
    
    async def get_valid_token(
        self,
        token_data: Dict[str, Any]
    ) -> str:
        """
        Get valid access token, refreshing if necessary.
        
        Args:
            token_data: Current token data
            
        Returns:
            Valid access token
        """
        if self.is_token_expired(token_data.get('expires_at', '')):
            # Refresh the token
            new_token_data = await self.refresh_access_token(
                token_data['refresh_token']
            )
            # Update the token data (caller should persist this)
            token_data.update(new_token_data)
            return new_token_data['access_token']
        
        return token_data['access_token']
    
    async def revoke_token(
        self,
        token: str,
        token_type: str = 'access_token'
    ) -> bool:
        """
        Revoke an access or refresh token.
        
        Args:
            token: Token to revoke
            token_type: Type of token ('access_token' or 'refresh_token')
            
        Returns:
            True if revocation successful
        """
        # This is provider-specific, override in subclasses
        return True
    
    def validate_state(
        self,
        received_state: str,
        expected_state: str
    ) -> bool:
        """
        Validate OAuth state parameter.
        
        Args:
            received_state: State received in callback
            expected_state: Expected state value
            
        Returns:
            True if states match
        """
        return secrets.compare_digest(received_state, expected_state)


class GoogleOAuthHandler(OAuthHandler):
    """Google-specific OAuth handler."""
    
    def __init__(
        self,
        client_id: str,
        client_secret: str,
        redirect_uri: str,
        scopes: Optional[List[str]] = None
    ):
        """Initialize Google OAuth handler."""
        default_scopes = [
            'https://www.googleapis.com/auth/calendar.readonly',
            'https://www.googleapis.com/auth/calendar.events.readonly'
        ]
        super().__init__(
            client_id=client_id,
            client_secret=client_secret,
            redirect_uri=redirect_uri,
            auth_url='https://accounts.google.com/o/oauth2/v2/auth',
            token_url='https://oauth2.googleapis.com/token',
            scopes=scopes or default_scopes
        )
    
    async def revoke_token(
        self,
        token: str,
        token_type: str = 'access_token'
    ) -> bool:
        """Revoke a Google OAuth token."""
        revoke_url = 'https://oauth2.googleapis.com/revoke'
        
        async with aiohttp.ClientSession() as session:
            async with session.post(
                revoke_url,
                data={'token': token}
            ) as response:
                return response.status == 200


class NotionOAuthHandler(OAuthHandler):
    """Notion-specific OAuth handler."""
    
    def __init__(
        self,
        client_id: str,
        client_secret: str,
        redirect_uri: str
    ):
        """Initialize Notion OAuth handler."""
        super().__init__(
            client_id=client_id,
            client_secret=client_secret,
            redirect_uri=redirect_uri,
            auth_url='https://api.notion.com/v1/oauth/authorize',
            token_url='https://api.notion.com/v1/oauth/token',
            scopes=[]  # Notion doesn't use scopes in the traditional way
        )
    
    def get_authorization_url(
        self,
        state: str,
        additional_params: Optional[Dict[str, str]] = None
    ) -> str:
        """Build Notion OAuth authorization URL."""
        params = {
            'client_id': self.client_id,
            'redirect_uri': self.redirect_uri,
            'response_type': 'code',
            'state': state,
            'owner': 'user'  # Notion-specific parameter
        }
        
        if additional_params:
            params.update(additional_params)
        
        query_string = urllib.parse.urlencode(params)
        return f"{self.auth_url}?{query_string}"
    
    async def exchange_code_for_token(
        self,
        code: str
    ) -> Dict[str, Any]:
        """Exchange authorization code for Notion access token."""
        import base64
        
        # Notion uses Basic auth for token exchange
        auth_string = f"{self.client_id}:{self.client_secret}"
        auth_bytes = auth_string.encode('utf-8')
        auth_b64 = base64.b64encode(auth_bytes).decode('utf-8')
        
        headers = {
            'Authorization': f'Basic {auth_b64}',
            'Content-Type': 'application/json'
        }
        
        data = {
            'code': code,
            'grant_type': 'authorization_code',
            'redirect_uri': self.redirect_uri
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(
                self.token_url,
                json=data,
                headers=headers
            ) as response:
                if response.status != 200:
                    error_text = await response.text()
                    raise Exception(f"Token exchange failed: {error_text}")
                
                token_data = await response.json()
                
                # Notion tokens don't expire
                token_data['expires_at'] = None
                
                return token_data


class StravaOAuthHandler(OAuthHandler):
    """Strava-specific OAuth handler."""
    
    def __init__(
        self,
        client_id: str,
        client_secret: str,
        redirect_uri: str,
        scopes: Optional[List[str]] = None
    ):
        """Initialize Strava OAuth handler."""
        default_scopes = [
            'read',
            'activity:read',
            'activity:read_all'
        ]
        super().__init__(
            client_id=client_id,
            client_secret=client_secret,
            redirect_uri=redirect_uri,
            auth_url='https://www.strava.com/oauth/authorize',
            token_url='https://www.strava.com/oauth/token',
            scopes=scopes or default_scopes
        )
    
    def get_authorization_url(
        self,
        state: str,
        additional_params: Optional[Dict[str, str]] = None
    ) -> str:
        """Build Strava OAuth authorization URL."""
        params = {
            'client_id': self.client_id,
            'redirect_uri': self.redirect_uri,
            'response_type': 'code',
            'scope': ','.join(self.scopes),  # Strava uses comma-separated scopes
            'state': state,
            'approval_prompt': 'force'  # Strava-specific parameter
        }
        
        if additional_params:
            params.update(additional_params)
        
        query_string = urllib.parse.urlencode(params)
        return f"{self.auth_url}?{query_string}"