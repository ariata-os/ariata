//! macOS device data sources
//!
//! Processors for data pushed from macOS devices including application usage,
//! browser history, iMessage streams, and screen time duration.

pub mod apps;
pub mod browser;
pub mod imessage;
pub mod registry;
pub mod transform;

// PushStream implementations
pub use apps::MacAppsStream;
pub use browser::MacBrowserStream;
pub use imessage::MacIMessageStream;

// Registry and transforms
pub use registry::MacSource;
pub use transform::{MacAppsTransform, MacBrowserTransform, MacIMessageTransform};
