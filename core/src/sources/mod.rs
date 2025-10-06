//! Source implementations for various data providers

pub mod base;
pub mod oauth_source;
pub mod google;
pub mod notion;
pub mod strava;

// Re-export the new DataSource trait from base for unified access
pub use base::DataSource;