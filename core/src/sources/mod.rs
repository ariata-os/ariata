//! Source implementations for various data providers

pub mod ariata;
pub mod auth;
pub mod base;
pub mod factory;
pub mod google;
pub mod ios;
pub mod mac;
pub mod notion;
pub mod pull_stream;
pub mod push_stream;
pub mod stream_type;

// Re-export commonly used types
pub use auth::SourceAuth;
pub use factory::StreamFactory;
pub use pull_stream::{PullStream, SyncMode, SyncResult};
pub use push_stream::{IngestPayload, PushResult, PushStream};
pub use stream_type::StreamType;
