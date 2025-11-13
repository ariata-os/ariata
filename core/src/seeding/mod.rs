//! Seeding module for Monday in Rome reference dataset
//!
//! This module provides functionality to seed the database with the Monday in Rome
//! reference dataset, which contains real-world data collected from a full day in Rome, Italy.

pub mod monday_in_rome;
pub mod narratives;
pub mod ontologies;

use crate::database::Database;
use crate::storage::{Storage, stream_writer::StreamWriter};
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
