//! Strava activities processing logic

use serde_json::json;

use crate::{
    error::Result,
    sources::SourceRecord,
};

use crate::sources::strava::types::Activity;

/// Activities processor
pub struct ActivitiesProcessor {
    source_name: String,
}

impl ActivitiesProcessor {
    /// Create a new activities processor
    pub fn new(source_name: &str) -> Self {
        Self {
            source_name: source_name.to_string(),
        }
    }

    /// Process activities into source records
    pub fn process_activities(&self, activities: Vec<Activity>) -> Result<Vec<SourceRecord>> {
        let mut records = Vec::with_capacity(activities.len());

        for activity in activities {
            records.push(self.activity_to_record(activity));
        }

        Ok(records)
    }

    /// Convert activity to source record
    fn activity_to_record(&self, activity: Activity) -> SourceRecord {
        SourceRecord {
            id: activity.id.to_string(),
            source: self.source_name.clone(),
            timestamp: activity.start_date,
            data: json!({
                "id": activity.id,
                "name": activity.name,
                "distance": activity.distance,
                "moving_time": activity.moving_time,
                "elapsed_time": activity.elapsed_time,
                "total_elevation_gain": activity.total_elevation_gain,
                "sport_type": activity.sport_type,
                "start_date": activity.start_date,
                "start_date_local": activity.start_date_local,
                "timezone": activity.timezone,
                "average_speed": activity.average_speed,
                "max_speed": activity.max_speed,
                "average_cadence": activity.average_cadence,
                "average_watts": activity.average_watts,
                "average_heartrate": activity.average_heartrate,
                "max_heartrate": activity.max_heartrate,
                "kilojoules": activity.kilojoules,
                "trainer": activity.trainer,
                "commute": activity.commute,
                "manual": activity.manual,
                "private": activity.private,
            }),
            metadata: Some(json!({
                "achievement_count": activity.achievement_count,
                "kudos_count": activity.kudos_count,
                "comment_count": activity.comment_count,
                "pr_count": activity.pr_count,
                "has_heartrate": activity.has_heartrate,
                "device_watts": activity.device_watts,
            })),
        }
    }
}