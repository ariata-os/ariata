//! Sync strategy abstractions for defining data window boundaries
//!
//! Defines strategies for determining what data to fetch during full refresh operations.
//! These strategies apply to initial syncs, full refreshes, and incremental fallbacks.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Strategy for defining data boundaries during full sync operations
///
/// Determines what time window or scope of data to fetch when performing
/// a full refresh (initial sync, forced resync, or incremental fallback).
///
/// # Examples
///
/// ```rust
/// use ariata_core::sources::base::SyncStrategy;
///
/// // Sync last year of data
/// let strategy = SyncStrategy::TimeWindow { days_back: 365 };
///
/// // Sync all available history
/// let strategy = SyncStrategy::FullHistory { max_records: None };
///
/// // Sync specific date range
/// let strategy = SyncStrategy::DateRange {
///     start_date: "2024-01-01T00:00:00Z".parse().unwrap(),
///     end_date: "2024-12-31T23:59:59Z".parse().unwrap(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncStrategy {
    /// Time window looking back from now
    ///
    /// Fetches data from N days ago until now. Most common strategy.
    ///
    /// # Example
    /// ```json
    /// {
    ///   "type": "time_window",
    ///   "days_back": 365
    /// }
    /// ```
    TimeWindow {
        /// Number of days to look back from current time
        days_back: u32,
    },

    /// Full historical sync (all available data)
    ///
    /// Fetches all data from the source with no time filtering.
    /// Optional max_records provides safety limit.
    ///
    /// # Example
    /// ```json
    /// {
    ///   "type": "full_history",
    ///   "max_records": 10000
    /// }
    /// ```
    FullHistory {
        /// Optional hard limit on number of records to prevent runaway syncs
        max_records: Option<u32>,
    },

    /// Absolute date range
    ///
    /// Fetches data between specific start and end timestamps.
    ///
    /// # Example
    /// ```json
    /// {
    ///   "type": "date_range",
    ///   "start_date": "2024-01-01T00:00:00Z",
    ///   "end_date": "2024-12-31T23:59:59Z"
    /// }
    /// ```
    DateRange {
        /// Start of date range (inclusive)
        start_date: DateTime<Utc>,
        /// End of date range (inclusive)
        end_date: DateTime<Utc>,
    },
}

impl Default for SyncStrategy {
    fn default() -> Self {
        // Default: sync last year of data
        Self::TimeWindow { days_back: 365 }
    }
}

impl SyncStrategy {
    /// Calculate time bounds for this strategy
    ///
    /// Returns (min_time, max_time) tuple where:
    /// - Some(time) = apply this boundary
    /// - None = no boundary (unbounded)
    ///
    /// # Returns
    /// - `TimeWindow`: (Some(now - N days), Some(now))
    /// - `FullHistory`: (None, None)
    /// - `DateRange`: (Some(start), Some(end))
    pub fn calculate_time_bounds(&self) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        match self {
            Self::TimeWindow { days_back } => {
                let now = Utc::now();
                let start = now - Duration::days(*days_back as i64);
                (Some(start), Some(now))
            }
            Self::FullHistory { .. } => {
                // No time bounds - fetch all available data
                (None, None)
            }
            Self::DateRange {
                start_date,
                end_date,
            } => (Some(*start_date), Some(*end_date)),
        }
    }

    /// Create a time window strategy
    pub fn time_window(days_back: u32) -> Self {
        Self::TimeWindow { days_back }
    }

    /// Create a full history strategy
    pub fn full_history() -> Self {
        Self::FullHistory { max_records: None }
    }

    /// Create a full history strategy with record limit
    pub fn full_history_with_limit(max_records: u32) -> Self {
        Self::FullHistory {
            max_records: Some(max_records),
        }
    }

    /// Create a date range strategy
    pub fn date_range(start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Self {
        Self::DateRange {
            start_date,
            end_date,
        }
    }

    /// Get the maximum records limit if applicable
    pub fn max_records(&self) -> Option<u32> {
        match self {
            Self::FullHistory { max_records } => *max_records,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_default_strategy() {
        let strategy = SyncStrategy::default();
        assert!(matches!(strategy, SyncStrategy::TimeWindow { days_back: 365 }));
    }

    #[test]
    fn test_time_window_bounds() {
        let strategy = SyncStrategy::TimeWindow { days_back: 30 };
        let (min, max) = strategy.calculate_time_bounds();

        assert!(min.is_some());
        assert!(max.is_some());

        let now = Utc::now();
        let min_time = min.unwrap();
        let max_time = max.unwrap();

        // Check approximately 30 days difference
        let diff = now - min_time;
        assert!(diff.num_days() >= 29 && diff.num_days() <= 31);

        // Max should be approximately now
        let diff = max_time - now;
        assert!(diff.num_seconds().abs() < 5);
    }

    #[test]
    fn test_full_history_bounds() {
        let strategy = SyncStrategy::FullHistory { max_records: None };
        let (min, max) = strategy.calculate_time_bounds();

        assert!(min.is_none());
        assert!(max.is_none());
    }

    #[test]
    fn test_date_range_bounds() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();

        let strategy = SyncStrategy::DateRange {
            start_date: start,
            end_date: end,
        };

        let (min, max) = strategy.calculate_time_bounds();

        assert_eq!(min, Some(start));
        assert_eq!(max, Some(end));
    }

    #[test]
    fn test_serialization_time_window() {
        let strategy = SyncStrategy::TimeWindow { days_back: 90 };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: SyncStrategy = serde_json::from_str(&json).unwrap();

        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_serialization_full_history() {
        let strategy = SyncStrategy::FullHistory {
            max_records: Some(10000),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: SyncStrategy = serde_json::from_str(&json).unwrap();

        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_serialization_date_range() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();

        let strategy = SyncStrategy::DateRange {
            start_date: start,
            end_date: end,
        };

        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: SyncStrategy = serde_json::from_str(&json).unwrap();

        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_helper_constructors() {
        let tw = SyncStrategy::time_window(30);
        assert!(matches!(tw, SyncStrategy::TimeWindow { days_back: 30 }));

        let fh = SyncStrategy::full_history();
        assert!(matches!(fh, SyncStrategy::FullHistory { max_records: None }));

        let fhl = SyncStrategy::full_history_with_limit(5000);
        assert!(matches!(
            fhl,
            SyncStrategy::FullHistory {
                max_records: Some(5000)
            }
        ));

        let start = Utc::now();
        let end = start + Duration::days(7);
        let dr = SyncStrategy::date_range(start, end);
        assert!(matches!(dr, SyncStrategy::DateRange { .. }));
    }

    #[test]
    fn test_max_records() {
        let tw = SyncStrategy::TimeWindow { days_back: 30 };
        assert_eq!(tw.max_records(), None);

        let fh = SyncStrategy::FullHistory { max_records: None };
        assert_eq!(fh.max_records(), None);

        let fhl = SyncStrategy::FullHistory {
            max_records: Some(1000),
        };
        assert_eq!(fhl.max_records(), Some(1000));
    }
}
