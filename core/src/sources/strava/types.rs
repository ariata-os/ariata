//! Strava API type definitions

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Strava Activity
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    pub id: i64,
    pub name: String,
    pub distance: f64,
    pub moving_time: i64,
    pub elapsed_time: i64,
    pub total_elevation_gain: f64,
    pub sport_type: String,
    pub start_date: DateTime<Utc>,
    pub start_date_local: DateTime<Utc>,
    pub timezone: String,
    pub achievement_count: i32,
    pub kudos_count: i32,
    pub comment_count: i32,
    pub athlete_count: i32,
    pub photo_count: i32,
    pub map: Option<PolylineMap>,
    pub trainer: bool,
    pub commute: bool,
    pub manual: bool,
    pub private: bool,
    pub flagged: bool,
    pub gear_id: Option<String>,
    pub average_speed: f64,
    pub max_speed: f64,
    pub average_cadence: Option<f64>,
    pub average_watts: Option<f64>,
    pub weighted_average_watts: Option<i32>,
    pub kilojoules: Option<f64>,
    pub device_watts: Option<bool>,
    pub has_heartrate: bool,
    pub average_heartrate: Option<f64>,
    pub max_heartrate: Option<i32>,
    pub pr_count: i32,
}

/// Activity map data
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PolylineMap {
    pub id: String,
    pub summary_polyline: Option<String>,
    pub resource_state: i32,
}

/// Athlete profile
#[derive(Debug, Deserialize, Serialize)]
pub struct Athlete {
    pub id: i64,
    pub username: Option<String>,
    pub firstname: String,
    pub lastname: String,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub sex: Option<String>,
    pub premium: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}