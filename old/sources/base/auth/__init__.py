"""Authentication handlers for OAuth and device tokens."""

from .oauth import (
    OAuthHandler,
    GoogleOAuthHandler,
    NotionOAuthHandler,
    StravaOAuthHandler
)
from .device_token import DeviceTokenHandler
from .manager import AuthManager, AuthType

__all__ = [
    'AuthManager',
    'AuthType',
    'OAuthHandler',
    'GoogleOAuthHandler',
    'NotionOAuthHandler',
    'StravaOAuthHandler',
    'DeviceTokenHandler'
]