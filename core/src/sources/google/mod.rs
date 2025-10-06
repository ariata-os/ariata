//! Google services integration

pub mod client;
pub mod config;
pub mod types;
pub mod calendar;
pub mod error_handler;
pub mod gmail;
pub mod registry;

pub use calendar::GoogleCalendarSync;
pub use config::{GoogleCalendarConfig, GoogleGmailConfig};
pub use error_handler::GoogleErrorHandler;
pub use gmail::GoogleGmailSync;