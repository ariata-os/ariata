//! Strava integration

pub mod auth;
pub mod client;
pub mod types;
pub mod activities;
pub mod registry;

use std::sync::Arc;
use crate::oauth::OAuthManager;

pub use activities::StravaActivitiesSource;

/// Create a new Strava activities source
pub fn activities_source(oauth: Arc<OAuthManager>) -> StravaActivitiesSource {
    StravaActivitiesSource::new(oauth)
}