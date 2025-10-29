//! iOS source registration for the catalog

use crate::registry::{AuthType, SourceDescriptor, SourceRegistry, StreamDescriptor};
use serde_json::json;

/// iOS source registration
pub struct IosSource;

impl SourceRegistry for IosSource {
    fn descriptor() -> SourceDescriptor {
        SourceDescriptor {
            name: "ios",
            display_name: "iOS",
            description: "Personal data from iOS devices (HealthKit, Location, Microphone)",
            auth_type: AuthType::Device,
            oauth_config: None,
            icon: Some("ri:apple-fill"),
            streams: vec![
                // HealthKit stream
                StreamDescriptor::new("healthkit")
                    .display_name("HealthKit")
                    .description("Health and fitness metrics including heart rate, steps, sleep, and workouts")
                    .table_name("stream_ios_healthkit")
                    .config_schema(healthkit_config_schema())
                    .config_example(healthkit_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based, not pull
                    .default_cron_schedule("*/5 * * * *")  // Every 5 minutes (device batches all streams)
                    .build(),

                // Location stream
                StreamDescriptor::new("location")
                    .display_name("Location")
                    .description("GPS coordinates, speed, altitude, and activity type")
                    .table_name("stream_ios_location")
                    .config_schema(location_config_schema())
                    .config_example(location_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based
                    .default_cron_schedule("*/5 * * * *")  // Every 5 minutes (device batches all streams)
                    .build(),

                // Microphone stream
                StreamDescriptor::new("microphone")
                    .display_name("Microphone")
                    .description("Audio levels, transcriptions, and recordings")
                    .table_name("stream_ios_microphone")
                    .config_schema(microphone_config_schema())
                    .config_example(microphone_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based
                    .default_cron_schedule("*/5 * * * *")  // Every 5 minutes (device batches all streams)
                    .build(),
            ],
        }
    }
}

/// JSON schema for HealthKit configuration
fn healthkit_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "enabled_metrics": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of metrics to collect: heart_rate, steps, sleep, workouts, etc."
            },
            "sampling_interval_seconds": {
                "type": "integer",
                "default": 60,
                "minimum": 10,
                "description": "How often to sample health metrics (in seconds)"
            }
        }
    })
}

fn healthkit_config_example() -> serde_json::Value {
    json!({
        "enabled_metrics": ["heart_rate", "steps", "sleep"],
        "sampling_interval_seconds": 60
    })
}

/// JSON schema for Location configuration
fn location_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "accuracy": {
                "type": "string",
                "enum": ["best", "high", "medium", "low"],
                "default": "high",
                "description": "GPS accuracy level"
            },
            "update_interval_seconds": {
                "type": "integer",
                "default": 30,
                "minimum": 5,
                "description": "Location update frequency (in seconds)"
            },
            "enable_background": {
                "type": "boolean",
                "default": false,
                "description": "Track location in background"
            }
        }
    })
}

fn location_config_example() -> serde_json::Value {
    json!({
        "accuracy": "high",
        "update_interval_seconds": 30,
        "enable_background": false
    })
}

/// JSON schema for Microphone configuration
fn microphone_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "enable_transcription": {
                "type": "boolean",
                "default": false,
                "description": "Enable speech-to-text transcription"
            },
            "store_audio": {
                "type": "boolean",
                "default": false,
                "description": "Store raw audio files in MinIO"
            },
            "sample_duration_seconds": {
                "type": "integer",
                "default": 5,
                "minimum": 1,
                "maximum": 60,
                "description": "Duration of each audio sample"
            }
        }
    })
}

fn microphone_config_example() -> serde_json::Value {
    json!({
        "enable_transcription": true,
        "store_audio": false,
        "sample_duration_seconds": 5
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_descriptor() {
        let desc = IosSource::descriptor();
        assert_eq!(desc.name, "ios");
        assert_eq!(desc.auth_type, AuthType::Device);
        assert_eq!(desc.streams.len(), 3);
    }

    #[test]
    fn test_healthkit_stream() {
        let desc = IosSource::descriptor();
        let healthkit = desc.streams.iter()
            .find(|s| s.name == "healthkit")
            .expect("HealthKit stream not found");

        assert_eq!(healthkit.display_name, "HealthKit");
        assert_eq!(healthkit.table_name, "stream_ios_healthkit");
        assert!(!healthkit.supports_incremental);
    }

    #[test]
    fn test_location_stream() {
        let desc = IosSource::descriptor();
        let location = desc.streams.iter()
            .find(|s| s.name == "location")
            .expect("Location stream not found");

        assert_eq!(location.display_name, "Location");
        assert_eq!(location.table_name, "stream_ios_location");
    }

    #[test]
    fn test_microphone_stream() {
        let desc = IosSource::descriptor();
        let mic = desc.streams.iter()
            .find(|s| s.name == "microphone")
            .expect("Microphone stream not found");

        assert_eq!(mic.display_name, "Microphone");
        assert_eq!(mic.table_name, "stream_ios_microphone");
    }
}
