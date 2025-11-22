use crate::database::Database;
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

/// Detect boundaries from continuous time-series data using changepoint detection
///
/// Algorithm: Simplified PELT (Pruned Exact Linear Time) for mean shift detection
/// Detects statistical changes in signal mean/variance to identify temporal segments
///
/// Use cases:
/// - Heart rate zones (resting → active → exercise)
/// - Audio volume levels (quiet → loud transitions)
/// - HRV transitions (calm → stressed states)
/// - Step count activity (sedentary → active periods)
pub async fn detect(
    db: &Database,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    table: &str,
    column: &str,
    penalty: f64,
    min_segment_minutes: i64,
) -> Result<Vec<BoundaryCandidate>> {
    // 1. Fetch time series data from database
    let query = format!(
        "SELECT timestamp, {} as value FROM data.{}
         WHERE timestamp >= $1 AND timestamp <= $2
         ORDER BY timestamp ASC",
        column, table
    );

    let rows = sqlx::query(&query)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(db.pool())
        .await?;

    if rows.len() < 10 {
        // Need minimum samples for statistical detection
        return Ok(Vec::new());
    }

    // 2. Extract (timestamp, value) pairs
    let mut timestamps = Vec::new();
    let mut values = Vec::new();

    for row in rows {
        let ts: DateTime<Utc> = row.try_get("timestamp")?;
        // Handle potential NULL values
        let val: Option<f64> = row.try_get("value")?;
        if let Some(v) = val {
            timestamps.push(ts);
            values.push(v);
        }
    }

    if values.len() < 10 {
        return Ok(Vec::new());
    }

    // 3. Run Bayesian Online Changepoint Detection (BOCPD)
    // Map penalty parameter to hazard lambda (λ):
    // - Higher penalty → higher λ → fewer changepoints (more conservative)
    // - penalty 3.0 → λ = 150 → P(changepoint) = 1/150 per timestep
    // - penalty 4.0 → λ = 200 → P(changepoint) = 1/200 per timestep
    let hazard_lambda = penalty * 50.0;

    let changepoint_indices = detect_changepoints_bocpd(&values, hazard_lambda);

    // 4. Convert changepoints to segments with begin/end boundaries
    // Each segment spans from one changepoint to the next
    let mut boundaries = Vec::new();

    // Build list of segment boundaries: [0, cp1, cp2, ..., len-1]
    let mut segment_points: Vec<usize> = vec![0];
    for &idx in &changepoint_indices {
        if idx > 0 && idx < timestamps.len() {
            segment_points.push(idx);
        }
    }
    segment_points.push(timestamps.len() - 1);

    // Create segments between consecutive points
    for window in segment_points.windows(2) {
        let start_idx = window[0];
        let end_idx = window[1];

        if end_idx <= start_idx {
            continue;
        }

        let segment_start = timestamps[start_idx];
        let segment_end = timestamps[end_idx];
        let duration = segment_end - segment_start;

        // Filter by minimum segment duration
        if duration < Duration::minutes(min_segment_minutes) {
            continue;
        }

        // Calculate segment statistics
        let segment_values = &values[start_idx..=end_idx.min(values.len() - 1)];
        let avg_value = mean(segment_values);

        // Begin boundary for this segment
        boundaries.push(BoundaryCandidate {
            timestamp: segment_start,
            boundary_type: BoundaryType::Begin,
            source_ontology: String::new(), // Set by caller
            fidelity: 0.0,                  // Set by caller
            weight: 0,                      // Set by caller
            metadata: serde_json::json!({
                "type": "continuous_segment",
                "detection_method": "bocpd_truncated",
                "hazard_lambda": hazard_lambda,
                "penalty_parameter": penalty,
                "avg_value": avg_value,
                "sample_count": segment_values.len(),
            }),
        });

        // End boundary for this segment
        boundaries.push(BoundaryCandidate {
            timestamp: segment_end,
            boundary_type: BoundaryType::End,
            source_ontology: String::new(),
            fidelity: 0.0,
            weight: 0,
            metadata: serde_json::json!({
                "type": "continuous_segment",
                "detection_method": "bocpd_truncated",
                "avg_value": avg_value,
                "duration_seconds": duration.num_seconds(),
            }),
        });
    }

    tracing::debug!(
        "Continuous detector ({}): Processed {} samples, found {} changepoints",
        table,
        values.len(),
        changepoint_indices.len()
    );

    Ok(boundaries)
}

/// Bayesian Online Changepoint Detection using truncated run-length distribution
///
/// Algorithm: BocpdTruncated from `changepoint` crate
/// - Maintains posterior probability distribution over run lengths
/// - Truncates unlikely run lengths for memory efficiency (O(m) vs O(n))
/// - Detects mean/variance shifts in Gaussian-distributed data
///
/// Parameters:
/// - data: Time-series values
/// - hazard_lambda: Hazard function parameter (λ where P(changepoint) = 1/λ)
///
/// Returns indices where changepoints occur (high probability of run_length=0)
fn detect_changepoints_bocpd(
    data: &[f64],
    hazard_lambda: f64,
) -> Vec<usize> {
    use changepoint::{utils::infer_changepoints, BocpdLike, BocpdTruncated};
    use rand::{rngs::SmallRng, SeedableRng};
    use rv::prelude::*;

    if data.len() < 10 {
        return Vec::new();
    }

    // Configure NormalGamma prior (weakly informative)
    // Parameters: (mean, precision, shape, scale)
    // Using standard uninformative prior
    let prior = NormalGamma::new_unchecked(
        0.0, // Mean: uninformative (will learn from data)
        1.0, // Precision: weak prior
        1.0, // Shape: uninformative
        1.0, // Scale: uninformative
    );

    // Initialize detector
    // First parameter is hazard_lambda (λ where P(changepoint) = 1/λ)
    let mut detector = BocpdTruncated::new(hazard_lambda, prior);

    // Optional: Preload with first few samples for better initialization
    let preload_size = 10.min(data.len());
    detector.preload(&data[..preload_size].to_vec());

    // Process each data point and collect run-length distributions
    let mut run_lengths: Vec<Vec<f64>> = Vec::new();
    for &value in data {
        let rl = detector.step(&value).to_vec();
        run_lengths.push(rl);
    }

    // Use built-in changepoint inference with MCMC sampling
    let mut rng = SmallRng::seed_from_u64(0x1234);
    match infer_changepoints(&run_lengths, 1000, &mut rng) {
        Ok(change_point_probs) => {
            // Extract indices where probability > threshold
            extract_changepoints_from_probs(&change_point_probs, 0.3)
        }
        Err(_) => {
            // Fallback: simple peak detection in run-length distributions
            extract_changepoints_from_run_lengths(&run_lengths)
        }
    }
}

/// Extract changepoint indices from probability distribution
fn extract_changepoints_from_probs(probs: &[f64], threshold: f64) -> Vec<usize> {
    probs
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p > threshold { Some(i) } else { None })
        .collect()
}

/// Fallback: Extract changepoint indices from run-length distributions
///
/// A changepoint occurs when the run length drops to 0 with high probability,
/// indicating the start of a new regime
fn extract_changepoints_from_run_lengths(run_lengths: &[Vec<f64>]) -> Vec<usize> {
    let mut changepoints = Vec::new();

    // Changepoint threshold: probability of run_length=0
    const CHANGEPOINT_THRESHOLD: f64 = 0.3;

    for (t, rl_dist) in run_lengths.iter().enumerate().skip(1) {
        if let Some(&prob_at_zero) = rl_dist.first() {
            // High probability at run_length=0 indicates changepoint
            if prob_at_zero > CHANGEPOINT_THRESHOLD {
                changepoints.push(t);
            }
        }
    }

    changepoints
}

/// Calculate variance of data slice
#[allow(dead_code)]
#[inline]
fn variance(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }

    let m = mean(data);
    data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (data.len() - 1) as f64
}

/// Calculate mean of data slice
#[inline]
fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().sum::<f64>() / data.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_calculation() {
        assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(mean(&[42.0]), 42.0);
    }

    #[test]
    fn test_changepoint_detection() {
        // Create data with clear shift: [1,1,1,1,5,5,5,5]
        let data = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0,
            5.0, 5.0, 5.0,
        ];

        // Test with hazard_lambda = 100 (P(changepoint) = 1/100)
        let changepoints = detect_changepoints_bocpd(&data, 100.0);
        assert!(
            !changepoints.is_empty(),
            "Should detect at least one changepoint"
        );

        // Changepoint should be around index 10
        assert!(
            changepoints[0] >= 8 && changepoints[0] <= 12,
            "Changepoint should be near the transition"
        );
    }
}
