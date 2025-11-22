//! Seeding module for reference datasets and production defaults
//!
//! This module provides functionality to seed the database with:
//! - Monday in Rome reference dataset (real-world data from a full day in Rome, Italy)
//! - Production defaults (models, agents, sample axiology tags)

pub mod config_loader;
pub mod monday_in_rome;
// pub mod narratives;  // Commented out - narrative_chunks table removed, use narrative_primitive instead
pub mod ontologies;
pub mod prod_seed;

use crate::database::Database;
use crate::storage::{stream_writer::StreamWriter, Storage};
use crate::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Seeds the database with Monday in Rome reference dataset
///
/// This seeds a real-world reference dataset through the full pipeline:
/// - Archive jobs (S3/MinIO upload)
/// - Transform jobs (ontology table population)
///
/// Requires storage and stream_writer to be initialized.
pub async fn seed_monday_in_rome_dataset(
    db: &Database,
    storage: &Storage,
    stream_writer: Arc<Mutex<StreamWriter>>,
) -> Result<usize> {
    monday_in_rome::seed_monday_in_rome(db, storage, stream_writer).await
}
