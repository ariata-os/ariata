//! Strava source registration for the catalog

use crate::registry::{AuthType, OAuthConfig, SourceDescriptor, SourceRegistry, StreamDescriptor};
use serde_json::json;

/// Strava source registration
pub struct StravaSource;

impl SourceRegistry for StravaSource {
    fn descriptor() -> SourceDescriptor {
        SourceDescriptor {
            name: "strava",
            display_name: "Strava",
            description: "Sync athletic activities, workouts, and performance data from Strava",
            auth_type: AuthType::OAuth2,
            oauth_config: Some(OAuthConfig {
                scopes: vec![
                    "read",
                    "activity:read_all",
                ],
                auth_url: "https://www.strava.com/oauth/authorize",
                token_url: "https://www.strava.com/oauth/token",
            }),
            streams: vec![
                // Activities stream
                StreamDescriptor::new("activities")
                    .display_name("Strava Activities")
                    .description("Sync workout activities with routes, performance metrics, and heart rate data")
                    .table_name("stream_strava_activities")
                    .config_schema(activities_config_schema())
                    .config_example(activities_config_example())
                    .supports_incremental(true)
                    .supports_full_refresh(true)
                    .build(),
            ],
        }
    }
}

/// JSON schema for Strava activities configuration
fn activities_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "activity_types": {
                "type": "array",
                "items": {
                    "type": "string",
                    "enum": ["Run", "Ride", "Swim", "Hike", "Walk", "AlpineSki", "BackcountrySki", "Workout", "Yoga"]
                },
                "description": "Filter activities by type (leave empty for all types)"
            },
            "sync_window_days": {
                "type": "integer",
                "default": 90,
                "minimum": 1,
                "maximum": 365,
                "description": "Number of days of activity history to sync"
            },
            "include_private": {
                "type": "boolean",
                "default": true,
                "description": "Include private activities"
            },
            "fetch_detailed_metrics": {
                "type": "boolean",
                "default": true,
                "description": "Fetch detailed streams (heart rate, power, cadence, GPS)"
            },
            "max_activities_per_sync": {
                "type": "integer",
                "default": 200,
                "minimum": 1,
                "maximum": 1000,
                "description": "Maximum number of activities to fetch per sync"
            }
        }
    })
}

/// Example configuration for Strava activities
fn activities_config_example() -> serde_json::Value {
    json!({
        "activity_types": ["Run", "Ride", "Swim"],
        "sync_window_days": 90,
        "include_private": true,
        "fetch_detailed_metrics": true,
        "max_activities_per_sync": 200
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strava_descriptor() {
        let desc = StravaSource::descriptor();
        assert_eq!(desc.name, "strava");
        assert_eq!(desc.auth_type, AuthType::OAuth2);
        assert!(desc.oauth_config.is_some());
        assert_eq!(desc.streams.len(), 1);
    }

    #[test]
    fn test_activities_stream() {
        let desc = StravaSource::descriptor();
        let activities = desc.streams.iter().find(|s| s.name == "activities");
        assert!(activities.is_some());

        let act = activities.unwrap();
        assert_eq!(act.table_name, "stream_strava_activities");
        assert!(act.supports_incremental);
        assert!(act.supports_full_refresh);
    }
}
