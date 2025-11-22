//! Timeline Module
//!
//! This module handles the complete narrative primitive pipeline from raw boundaries
//! to synthesized events.
//!
//! ## Architecture
//!
//! - `boundaries/`: Detects and aggregates event boundaries from ontology data
//! - `synthesis/`: Converts boundaries into narrative primitives (who/when/where/why/what/how)
//! - `events.rs`: EventBoundary struct representing temporal transition points
//!
//! ## Pipeline
//!
//! 1. **Entity Resolution** (inline): Cluster locations, resolve people
//! 2. **Boundary Detection**: Detect changepoints across ontologies
//! 3. **Boundary Aggregation**: Weight-based merging and filtering
//! 4. **Narrative Synthesis**: Extract substance and create primitives
//!
//! This module is deterministic and runs hourly with a 6-hour lookback window.

pub mod boundaries;
pub mod events;
pub mod synthesis;
