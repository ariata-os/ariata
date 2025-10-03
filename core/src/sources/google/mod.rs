//! Google services integration

pub mod client;
pub mod config;
pub mod types;
pub mod calendar;
pub mod error_handler;

pub use calendar::{GoogleCalendarSync, SyncStats};
pub use config::GoogleCalendarConfig;
pub use error_handler::GoogleErrorHandler;