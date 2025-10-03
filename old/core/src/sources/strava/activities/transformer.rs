//! Strava activities data transformation utilities

use serde_json::Value;
use crate::sources::strava::types::Activity;

/// Transform activity data for different use cases
pub struct ActivityTransformer;

impl ActivityTransformer {
    /// Calculate speed in km/h
    pub fn speed_kmh(activity: &Activity) -> f64 {
        activity.average_speed * 3.6
    }

    /// Calculate pace in minutes per kilometer
    pub fn pace_min_per_km(activity: &Activity) -> Option<f64> {
        if activity.average_speed > 0.0 {
            Some(16.666667 / activity.average_speed)
        } else {
            None
        }
    }

    /// Convert distance to kilometers
    pub fn distance_km(activity: &Activity) -> f64 {
        activity.distance / 1000.0
    }

    /// Convert moving time to hours
    pub fn moving_time_hours(activity: &Activity) -> f64 {
        activity.moving_time as f64 / 3600.0
    }

    /// Calculate efficiency (moving time / elapsed time)
    pub fn efficiency(activity: &Activity) -> f64 {
        if activity.elapsed_time > 0 {
            (activity.moving_time as f64 / activity.elapsed_time as f64) * 100.0
        } else {
            100.0
        }
    }

    /// Transform activity for analytics
    pub fn to_analytics_format(activity: &Activity) -> Value {
        serde_json::json!({
            "activity_id": activity.id,
            "name": activity.name,
            "sport_type": activity.sport_type,
            "start_time": activity.start_date,
            "distance_km": Self::distance_km(activity),
            "duration_hours": Self::moving_time_hours(activity),
            "speed_kmh": Self::speed_kmh(activity),
            "pace_min_per_km": Self::pace_min_per_km(activity),
            "elevation_gain_m": activity.total_elevation_gain,
            "efficiency_percent": Self::efficiency(activity),
            "avg_heartrate": activity.average_heartrate,
            "avg_power_watts": activity.average_watts,
            "energy_kj": activity.kilojoules,
            "is_trainer": activity.trainer,
            "is_commute": activity.commute,
            "is_manual": activity.manual,
        })
    }

    /// Categorize activity by intensity
    pub fn categorize_intensity(activity: &Activity) -> &'static str {
        match activity.average_heartrate {
            Some(hr) if hr < 120.0 => "easy",
            Some(hr) if hr < 140.0 => "moderate",
            Some(hr) if hr < 160.0 => "hard",
            Some(_) => "max",
            None => "unknown",
        }
    }
}