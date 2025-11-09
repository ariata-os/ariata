//! iOS HealthKit data processor and transforms

pub mod transform;

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validate_heart_rate, validate_positive, validate_percentage, validate_timestamp_reasonable},
    storage::Storage,
};

pub use transform::{
    HealthKitHeartRateTransform,
    HealthKitHRVTransform,
    HealthKitStepsTransform,
    HealthKitSleepTransform,
    HealthKitWorkoutTransform,
};

/// Process iOS HealthKit data
///
/// Parses and stores health metrics from iOS devices including heart rate, steps,
/// sleep data, workouts, and other HealthKit measurements.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for HealthKit, but kept for API consistency)
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, record), fields(source = "ios", stream = "healthkit"))]
pub async fn process(db: &Database, _storage: &Arc<Storage>, record: &Value) -> Result<()> {
    // Get source_id from device_id in the record
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "ios", device_id).await?;

    // Parse timestamp
    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    // Extract health metrics (all optional)
    let heart_rate = record.get("heart_rate").and_then(|v| v.as_f64());
    let hrv = record.get("hrv").and_then(|v| v.as_f64());
    let resting_heart_rate = record.get("resting_heart_rate").and_then(|v| v.as_f64());
    let steps = record.get("steps").and_then(|v| v.as_i64()).map(|v| v as i32);
    let distance = record.get("distance").and_then(|v| v.as_f64());
    let active_energy = record.get("active_energy").and_then(|v| v.as_f64());
    let basal_energy = record.get("basal_energy").and_then(|v| v.as_f64());
    let flights_climbed = record.get("flights_climbed").and_then(|v| v.as_i64()).map(|v| v as i32);

    let sleep_stage = record.get("sleep_stage").and_then(|v| v.as_str());
    let sleep_duration = record.get("sleep_duration").and_then(|v| v.as_i64()).map(|v| v as i32);

    let workout_type = record.get("workout_type").and_then(|v| v.as_str());
    let workout_duration = record.get("workout_duration").and_then(|v| v.as_i64()).map(|v| v as i32);

    let weight = record.get("weight").and_then(|v| v.as_f64());
    let body_fat_percentage = record.get("body_fat_percentage").and_then(|v| v.as_f64());
    let mindful_minutes = record.get("mindful_minutes").and_then(|v| v.as_i64()).map(|v| v as i32);

    let device_name = record.get("device_name").and_then(|v| v.as_str());
    let device_model = record.get("device_model").and_then(|v| v.as_str());

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    // Validate health metrics if present
    if let Some(hr) = heart_rate {
        validate_heart_rate(hr)?;
    }
    if let Some(rhr) = resting_heart_rate {
        validate_heart_rate(rhr)?;
    }
    if let Some(hrv_val) = hrv {
        validate_positive("HRV", hrv_val)?;
    }
    if let Some(s) = steps {
        if s < 0 {
            return Err(Error::InvalidInput("Steps cannot be negative".into()));
        }
    }
    if let Some(d) = distance {
        validate_positive("Distance", d)?;
    }
    if let Some(bf) = body_fat_percentage {
        validate_percentage("Body fat percentage", bf)?;
    }

    sqlx::query(
        "INSERT INTO stream_ios_healthkit
         (source_id, timestamp, heart_rate, hrv, resting_heart_rate, steps, distance,
          active_energy, basal_energy, flights_climbed, sleep_stage, sleep_duration,
          workout_type, workout_duration, weight, body_fat_percentage, mindful_minutes,
          device_name, device_model, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
         ON CONFLICT (source_id, timestamp)
         DO UPDATE SET
            heart_rate = EXCLUDED.heart_rate,
            hrv = EXCLUDED.hrv,
            resting_heart_rate = EXCLUDED.resting_heart_rate,
            steps = EXCLUDED.steps,
            distance = EXCLUDED.distance,
            active_energy = EXCLUDED.active_energy,
            basal_energy = EXCLUDED.basal_energy,
            flights_climbed = EXCLUDED.flights_climbed,
            sleep_stage = EXCLUDED.sleep_stage,
            sleep_duration = EXCLUDED.sleep_duration,
            workout_type = EXCLUDED.workout_type,
            workout_duration = EXCLUDED.workout_duration,
            weight = EXCLUDED.weight,
            body_fat_percentage = EXCLUDED.body_fat_percentage,
            mindful_minutes = EXCLUDED.mindful_minutes,
            device_name = EXCLUDED.device_name,
            device_model = EXCLUDED.device_model,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(heart_rate)
    .bind(hrv)
    .bind(resting_heart_rate)
    .bind(steps)
    .bind(distance)
    .bind(active_energy)
    .bind(basal_energy)
    .bind(flights_climbed)
    .bind(sleep_stage)
    .bind(sleep_duration)
    .bind(workout_type)
    .bind(workout_duration)
    .bind(weight)
    .bind(body_fat_percentage)
    .bind(mindful_minutes)
    .bind(device_name)
    .bind(device_model)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert healthkit data: {e}")))?;

    tracing::debug!("Inserted healthkit record for device {}", device_id);
    Ok(())
}
