//! macOS device data sources
//!
//! Processors for data pushed from macOS devices including application usage,
//! browser history, iMessage streams, and screen time duration.

pub mod apps;
pub mod browser;
pub mod imessage;
pub mod registry;
pub mod screen_time;
pub mod transform;

pub use apps::process as process_apps;
pub use browser::process as process_browser;
pub use imessage::process as process_imessage;
pub use registry::MacSource;
pub use screen_time::process as process_screen_time;
pub use transform::{MacAppsTransform, MacBrowserTransform, MacIMessageTransform};
