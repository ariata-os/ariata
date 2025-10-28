//! OAuth authentication and token management
//!
//! This module provides OAuth token management via TokenManager.
//! For new code, use TokenManager directly with the StreamFactory pattern.

pub mod encryption;
pub mod token_manager;

// Re-export the main types
pub use token_manager::{TokenManager, OAuthToken, OAuthProxyConfig};
pub use encryption::TokenEncryptor;