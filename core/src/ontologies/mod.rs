//! Ontology definitions and registry
//!
//! This module provides a unified registry for all ontology tables in the system.
//! Each ontology defines:
//! - Table schema and metadata
//! - Boundary detection configuration (for narrative timeline)
//! - Narrative role (container/structure/substance)
//! - Substance extraction logic
//!
//! ## Architecture
//!
//! Ontologies are organized by domain:
//! - `health/` - Heart rate, HRV, sleep, steps, workouts
//! - `location/` - GPS points, clustered visits
//! - `social/` - Email, messages
//! - `praxis/` - Calendar events, tasks
//! - `activity/` - App usage, web browsing
//! - `speech/` - Transcriptions
//!
//! ## Adding a New Ontology
//!
//! 1. Create the table in a migration (e.g., `migrations/00X_*.sql`)
//! 2. Create the ontology module (e.g., `ontologies/health/sleep.rs`)
//! 3. Implement `OntologyDescriptor` trait
//! 4. Register in the domain's `registry.rs`
//! 5. Run `cargo run --bin generate-seeds` to update configs

pub mod descriptor;
pub mod detector;
pub mod registry;

// Domain modules
pub mod activity;
pub mod health;
pub mod knowledge;
pub mod location;
pub mod praxis;
pub mod social;
pub mod speech;

// Re-export main types
pub use descriptor::{
    BoundaryConfig, DetectionAlgorithm, NarrativeRole, Ontology, OntologyBuilder,
    OntologyDescriptor, SubstanceQuery,
};
pub use detector::BoundaryDetector;
pub use registry::{get_ontology, list_ontologies, ontology_registry, OntologyRegistry};
