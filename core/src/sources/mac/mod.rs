//! macOS device data sources
//!
//! Processors for data pushed from macOS devices including application usage,
//! browser history, and iMessage streams.

pub mod apps;
pub mod browser;
pub mod imessage;
pub mod registry;

pub use apps::process as process_apps;
pub use browser::process as process_browser;
pub use imessage::process as process_imessage;
pub use registry::MacSource;
