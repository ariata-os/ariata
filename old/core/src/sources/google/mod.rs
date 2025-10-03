//! Google services integration

pub mod auth;
pub mod client;
pub mod types;
pub mod calendar;

use std::sync::Arc;
use crate::oauth::OAuthManager;

pub use calendar::GoogleCalendarSource;

/// Create a new Google Calendar source
pub fn calendar_source(oauth: Arc<OAuthManager>) -> GoogleCalendarSource {
    GoogleCalendarSource::new(oauth)
}