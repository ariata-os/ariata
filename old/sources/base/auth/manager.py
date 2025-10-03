"""Unified authentication manager for all source types."""

from typing import Dict, Any, Optional, Union
from datetime import datetime, timedelta
from enum import Enum
from .oauth import OAuthHandler, GoogleOAuthHandler, NotionOAuthHandler, StravaOAuthHandler
from .device_token import DeviceTokenHandler


class AuthType(Enum):
    """Authentication types supported by the platform."""
    OAUTH = "oauth"
    DEVICE_TOKEN = "device_token"
    API_KEY = "api_key"
    NONE = "none"


class AuthManager:
    """
    Unified authentication manager for all sources.
    
    This manager provides a single interface for handling authentication
    across all source types, whether they use OAuth, device tokens, or API keys.
    """
    
    # Source to auth type mapping
    SOURCE_AUTH_TYPES = {
        'google_calendar': AuthType.OAUTH,
        'notion_pages': AuthType.OAUTH,
        'strava_activities': AuthType.OAUTH,
        'ios_healthkit': AuthType.DEVICE_TOKEN,
        'ios_location': AuthType.DEVICE_TOKEN,
        'ios_mic': AuthType.DEVICE_TOKEN,
        'mac_apps': AuthType.DEVICE_TOKEN,
    }
    
    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        Initialize auth manager with configuration.
        
        Args:
            config: Optional configuration dictionary with OAuth credentials, etc.
        """
        self.config = config or {}
        self._init_handlers()
    
    def _init_handlers(self):
        """Initialize authentication handlers based on configuration."""
        # Initialize device token handler
        self.device_token_handler = DeviceTokenHandler(
            secret_key=self.config.get('device_token_secret')
        )
        
        # Initialize OAuth handlers
        self.oauth_handlers = {}
        
        # Google OAuth
        if 'google' in self.config:
            self.oauth_handlers['google'] = GoogleOAuthHandler(
                client_id=self.config['google']['client_id'],
                client_secret=self.config['google']['client_secret'],
                redirect_uri=self.config['google'].get('redirect_uri', 'http://localhost:8000/auth/callback')
            )
        
        # Notion OAuth
        if 'notion' in self.config:
            self.oauth_handlers['notion'] = NotionOAuthHandler(
                client_id=self.config['notion']['client_id'],
                client_secret=self.config['notion']['client_secret'],
                redirect_uri=self.config['notion'].get('redirect_uri', 'http://localhost:8000/auth/callback')
            )
        
        # Strava OAuth
        if 'strava' in self.config:
            self.oauth_handlers['strava'] = StravaOAuthHandler(
                client_id=self.config['strava']['client_id'],
                client_secret=self.config['strava']['client_secret'],
                redirect_uri=self.config['strava'].get('redirect_uri', 'http://localhost:8000/auth/callback')
            )
    
    def get_auth_type(self, source_name: str) -> AuthType:
        """
        Get the authentication type for a source.
        
        Args:
            source_name: Name of the source
            
        Returns:
            Authentication type
        """
        return self.SOURCE_AUTH_TYPES.get(source_name, AuthType.NONE)
    
    def get_oauth_handler(self, provider: str) -> Optional[OAuthHandler]:
        """
        Get OAuth handler for a provider.
        
        Args:
            provider: OAuth provider name (google, notion, strava)
            
        Returns:
            OAuth handler instance or None
        """
        return self.oauth_handlers.get(provider)
    
    async def authenticate_source(
        self,
        source_name: str,
        credentials: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        Authenticate a source with provided credentials.
        
        Args:
            source_name: Name of the source to authenticate
            credentials: Authentication credentials
            
        Returns:
            Authentication result with tokens/keys
        """
        auth_type = self.get_auth_type(source_name)
        
        if auth_type == AuthType.OAUTH:
            return await self._authenticate_oauth(source_name, credentials)
        elif auth_type == AuthType.DEVICE_TOKEN:
            return self._authenticate_device(source_name, credentials)
        elif auth_type == AuthType.API_KEY:
            return self._validate_api_key(source_name, credentials)
        else:
            return {'authenticated': True, 'type': 'none'}
    
    async def _authenticate_oauth(
        self,
        source_name: str,
        credentials: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Handle OAuth authentication."""
        # Map source to provider
        provider_map = {
            'google_calendar': 'google',
            'notion_pages': 'notion',
            'strava_activities': 'strava',
        }
        
        provider = provider_map.get(source_name)
        if not provider:
            raise ValueError(f"No OAuth provider for source: {source_name}")
        
        handler = self.get_oauth_handler(provider)
        if not handler:
            raise ValueError(f"OAuth handler not configured for: {provider}")
        
        # If we have an authorization code, exchange it
        if 'code' in credentials:
            token_data = await handler.exchange_code_for_token(credentials['code'])
            return {
                'authenticated': True,
                'type': 'oauth',
                'provider': provider,
                'access_token': token_data['access_token'],
                'refresh_token': token_data.get('refresh_token'),
                'expires_at': token_data.get('expires_at')
            }
        
        # If we have a refresh token, refresh the access token
        elif 'refresh_token' in credentials:
            token_data = await handler.refresh_access_token(credentials['refresh_token'])
            return {
                'authenticated': True,
                'type': 'oauth',
                'provider': provider,
                'access_token': token_data['access_token'],
                'refresh_token': token_data.get('refresh_token', credentials['refresh_token']),
                'expires_at': token_data.get('expires_at')
            }
        
        # If we just have an access token, validate it
        elif 'access_token' in credentials:
            return {
                'authenticated': True,
                'type': 'oauth',
                'provider': provider,
                'access_token': credentials['access_token'],
                'refresh_token': credentials.get('refresh_token'),
                'expires_at': credentials.get('expires_at')
            }
        
        else:
            raise ValueError("Invalid OAuth credentials")
    
    def _authenticate_device(
        self,
        source_name: str,
        credentials: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Handle device token authentication."""
        # Generate new device token
        if 'device_id' in credentials and 'user_id' in credentials:
            device_type = 'ios' if 'ios' in source_name else 'mac'
            token_data = self.device_token_handler.generate_device_token(
                device_id=credentials['device_id'],
                device_type=device_type,
                user_id=credentials['user_id']
            )
            return {
                'authenticated': True,
                'type': 'device_token',
                **token_data
            }
        
        # Validate existing token
        elif 'token' in credentials and 'device_id' in credentials:
            is_valid = self.device_token_handler.validate_device_token(
                token=credentials['token'],
                device_id=credentials['device_id']
            )
            return {
                'authenticated': is_valid,
                'type': 'device_token',
                'token': credentials['token'] if is_valid else None
            }
        
        # Refresh token
        elif 'refresh_token' in credentials and 'device_id' in credentials:
            new_token = self.device_token_handler.refresh_device_token(
                refresh_token=credentials['refresh_token'],
                device_id=credentials['device_id']
            )
            if new_token:
                return {
                    'authenticated': True,
                    'type': 'device_token',
                    **new_token
                }
            else:
                return {
                    'authenticated': False,
                    'type': 'device_token',
                    'error': 'Invalid refresh token'
                }
        
        else:
            raise ValueError("Invalid device token credentials")
    
    def _validate_api_key(
        self,
        source_name: str,
        credentials: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Validate API key authentication."""
        if 'api_key' not in credentials:
            raise ValueError("API key required")
        
        # For now, we just pass through the API key
        # In production, you might want to validate against a service
        return {
            'authenticated': True,
            'type': 'api_key',
            'api_key': credentials['api_key']
        }
    
    async def refresh_credentials(
        self,
        source_name: str,
        current_credentials: Dict[str, Any]
    ) -> Optional[Dict[str, Any]]:
        """
        Refresh credentials if they're expired or expiring soon.
        
        Args:
            source_name: Source name
            current_credentials: Current credentials
            
        Returns:
            Refreshed credentials or None if refresh not needed/possible
        """
        auth_type = self.get_auth_type(source_name)
        
        if auth_type == AuthType.OAUTH:
            # Check if token is expired or expiring soon
            expires_at = current_credentials.get('expires_at')
            if expires_at:
                expiry = datetime.fromisoformat(expires_at)
                if expiry <= datetime.utcnow() + timedelta(minutes=5):
                    # Token expired or expiring soon, refresh it
                    if 'refresh_token' in current_credentials:
                        return await self._authenticate_oauth(
                            source_name,
                            {'refresh_token': current_credentials['refresh_token']}
                        )
        
        elif auth_type == AuthType.DEVICE_TOKEN:
            # Device tokens have longer expiry, check if refresh needed
            expires_at = current_credentials.get('expires_at')
            if expires_at:
                expiry = datetime.fromisoformat(expires_at)
                if expiry <= datetime.utcnow() + timedelta(days=1):
                    # Token expiring soon, refresh it
                    if 'refresh_token' in current_credentials:
                        return self._authenticate_device(
                            source_name,
                            {
                                'refresh_token': current_credentials['refresh_token'],
                                'device_id': current_credentials.get('device_id')
                            }
                        )
        
        return None
    
    def get_authorization_url(
        self,
        source_name: str,
        state: Optional[str] = None
    ) -> Optional[str]:
        """
        Get OAuth authorization URL for a source.
        
        Args:
            source_name: Source name
            state: Optional state parameter
            
        Returns:
            Authorization URL or None if not OAuth
        """
        auth_type = self.get_auth_type(source_name)
        if auth_type != AuthType.OAUTH:
            return None
        
        provider_map = {
            'google_calendar': 'google',
            'notion_pages': 'notion',
            'strava_activities': 'strava',
        }
        
        provider = provider_map.get(source_name)
        if not provider:
            return None
        
        handler = self.get_oauth_handler(provider)
        if not handler:
            return None
        
        state = state or handler.generate_state()
        return handler.get_authorization_url(state)
    
    def create_pairing_session(
        self,
        source_name: str,
        device_info: Dict[str, Any]
    ) -> Optional[Dict[str, Any]]:
        """
        Create a device pairing session.
        
        Args:
            source_name: Source name
            device_info: Device information
            
        Returns:
            Pairing session data or None if not device auth
        """
        auth_type = self.get_auth_type(source_name)
        if auth_type != AuthType.DEVICE_TOKEN:
            return None
        
        pairing_code = self.device_token_handler.generate_pairing_code()
        return self.device_token_handler.create_pairing_session(
            pairing_code=pairing_code,
            device_info=device_info
        )
    
    def complete_pairing(
        self,
        session_id: str,
        user_id: str,
        device_info: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        Complete device pairing.
        
        Args:
            session_id: Pairing session ID
            user_id: User ID
            device_info: Device information
            
        Returns:
            Device authentication data
        """
        return self.device_token_handler.complete_pairing(
            session_id=session_id,
            user_id=user_id,
            device_info=device_info
        )