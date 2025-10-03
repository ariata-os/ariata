"""Device token authentication handler for iOS and Mac sources."""

from typing import Dict, Any, Optional
from datetime import datetime, timedelta
import secrets
import hashlib
import hmac


class DeviceTokenHandler:
    """Handles device token authentication for native apps."""
    
    def __init__(self, secret_key: Optional[str] = None):
        """
        Initialize device token handler.
        
        Args:
            secret_key: Secret key for token signing (uses env var if not provided)
        """
        import os
        self.secret_key = secret_key or os.getenv('DEVICE_TOKEN_SECRET', 'default-secret-key')
    
    def generate_pairing_code(self, length: int = 6) -> str:
        """
        Generate a pairing code for device authentication.
        
        Args:
            length: Length of pairing code
            
        Returns:
            Numeric pairing code string
        """
        # Generate numeric code for easy entry
        code = ''.join(secrets.choice('0123456789') for _ in range(length))
        return code
    
    def generate_device_token(
        self,
        device_id: str,
        device_type: str,
        user_id: str
    ) -> Dict[str, Any]:
        """
        Generate a device authentication token.
        
        Args:
            device_id: Unique device identifier
            device_type: Type of device (ios, mac)
            user_id: User identifier
            
        Returns:
            Token data dictionary
        """
        # Create token payload
        payload = f"{device_id}:{device_type}:{user_id}:{datetime.utcnow().isoformat()}"
        
        # Generate token
        token = self._sign_payload(payload)
        
        # Generate refresh token
        refresh_payload = f"refresh:{payload}:{secrets.token_hex(16)}"
        refresh_token = self._sign_payload(refresh_payload)
        
        return {
            'device_id': device_id,
            'device_type': device_type,
            'user_id': user_id,
            'token': token,
            'refresh_token': refresh_token,
            'created_at': datetime.utcnow().isoformat(),
            'expires_at': (datetime.utcnow() + timedelta(days=30)).isoformat()
        }
    
    def validate_device_token(
        self,
        token: str,
        device_id: str
    ) -> bool:
        """
        Validate a device token.
        
        Args:
            token: Token to validate
            device_id: Expected device ID
            
        Returns:
            True if token is valid
        """
        try:
            # Verify signature
            parts = token.split('.')
            if len(parts) != 2:
                return False
            
            payload, signature = parts
            expected_signature = self._create_signature(payload)
            
            if not hmac.compare_digest(signature, expected_signature):
                return False
            
            # Decode and validate payload
            import base64
            decoded_payload = base64.urlsafe_b64decode(
                payload + '=' * (4 - len(payload) % 4)
            ).decode('utf-8')
            
            # Check device ID matches
            payload_parts = decoded_payload.split(':')
            if payload_parts[0] != device_id:
                return False
            
            return True
        except Exception:
            return False
    
    def refresh_device_token(
        self,
        refresh_token: str,
        device_id: str
    ) -> Optional[Dict[str, Any]]:
        """
        Refresh a device token using refresh token.
        
        Args:
            refresh_token: Refresh token
            device_id: Device identifier
            
        Returns:
            New token data or None if invalid
        """
        # Validate refresh token
        if not self.validate_device_token(refresh_token, device_id):
            return None
        
        # Extract user info from refresh token
        try:
            parts = refresh_token.split('.')
            payload = parts[0]
            
            import base64
            decoded_payload = base64.urlsafe_b64decode(
                payload + '=' * (4 - len(payload) % 4)
            ).decode('utf-8')
            
            payload_parts = decoded_payload.split(':')
            if len(payload_parts) < 3:
                return None
            
            # Generate new token
            return self.generate_device_token(
                device_id=payload_parts[1],
                device_type=payload_parts[2],
                user_id=payload_parts[3]
            )
        except Exception:
            return None
    
    def _sign_payload(self, payload: str) -> str:
        """
        Sign a payload to create a token.
        
        Args:
            payload: Payload to sign
            
        Returns:
            Signed token string
        """
        import base64
        
        # Encode payload
        encoded_payload = base64.urlsafe_b64encode(
            payload.encode('utf-8')
        ).decode('utf-8').rstrip('=')
        
        # Create signature
        signature = self._create_signature(encoded_payload)
        
        return f"{encoded_payload}.{signature}"
    
    def _create_signature(self, data: str) -> str:
        """
        Create HMAC signature for data.
        
        Args:
            data: Data to sign
            
        Returns:
            Signature string
        """
        signature = hmac.new(
            self.secret_key.encode('utf-8'),
            data.encode('utf-8'),
            hashlib.sha256
        ).hexdigest()
        
        return signature
    
    def create_pairing_session(
        self,
        pairing_code: str,
        device_info: Dict[str, Any],
        ttl_seconds: int = 300
    ) -> Dict[str, Any]:
        """
        Create a pairing session for device authentication.
        
        Args:
            pairing_code: The pairing code
            device_info: Information about the device
            ttl_seconds: Time to live for pairing session
            
        Returns:
            Pairing session data
        """
        session_id = secrets.token_urlsafe(32)
        
        return {
            'session_id': session_id,
            'pairing_code': pairing_code,
            'device_info': device_info,
            'created_at': datetime.utcnow().isoformat(),
            'expires_at': (datetime.utcnow() + timedelta(seconds=ttl_seconds)).isoformat(),
            'status': 'pending'
        }
    
    def complete_pairing(
        self,
        session_id: str,
        user_id: str,
        device_info: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        Complete device pairing and generate tokens.
        
        Args:
            session_id: Pairing session ID
            user_id: User completing the pairing
            device_info: Device information
            
        Returns:
            Device authentication data
        """
        # Generate device token
        token_data = self.generate_device_token(
            device_id=device_info.get('device_id'),
            device_type=device_info.get('device_type'),
            user_id=user_id
        )
        
        # Add pairing completion info
        token_data['paired_at'] = datetime.utcnow().isoformat()
        token_data['session_id'] = session_id
        
        return token_data