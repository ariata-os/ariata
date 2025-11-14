//! Google services integration

pub mod calendar;
pub mod client;
pub mod config;
pub mod error_handler;
pub mod gmail;
pub mod registry;
pub mod types;

pub use calendar::GoogleCalendarStream;
pub use config::{GoogleCalendarConfig, GoogleGmailConfig};
pub use error_handler::GoogleErrorHandler;
pub use gmail::GoogleGmailStream;
