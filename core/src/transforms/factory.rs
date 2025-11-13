//! Transform factory for dynamic transform instantiation
//!
//! This module provides centralized transform construction logic, eliminating
//! the need for hardcoded match statements in multiple places.

use crate::error::{Error, Result};
use crate::jobs::transform_context::TransformContext;
use crate::sources::base::OntologyTransform;
use crate::sources::ariata::transform::ChatConversationTransform;
use crate::sources::google::calendar::transform::GoogleCalendarTransform;
use crate::sources::google::gmail::transform::GmailEmailTransform;
use crate::sources::ios::healthkit::transform::{
    HealthKitHeartRateTransform, HealthKitHRVTransform, HealthKitStepsTransform,
    HealthKitSleepTransform, HealthKitWorkoutTransform,
};
use crate::sources::ios::location::transform::IosLocationTransform;
use crate::sources::ios::microphone::MicrophoneTranscriptionTransform;
use crate::sources::notion::pages::transform::NotionPageTransform;

/// Transform factory for creating transform instances
pub struct TransformFactory;

impl TransformFactory {
    /// Create a transform instance for the given source and target tables
    ///
    /// This is the single source of truth for transform instantiation.
    /// All transform routing logic should go through this factory.
    ///
    /// # Arguments
    /// * `source_table` - Source stream table name (e.g., "stream_ios_location")
    /// * `target_table` - Target ontology table name (e.g., "location_point")
    /// * `context` - Transform context with dependencies (storage, API keys)
    ///
    /// # Returns
    /// A boxed transform instance ready to execute
    pub fn create(
        source_table: &str,
        target_table: &str,
        context: &TransformContext,
    ) -> Result<Box<dyn OntologyTransform>> {
        match (source_table, target_table) {
            // Ariata internal streams
            ("stream_ariata_ai_chat", "knowledge_ai_conversation") => {
                Ok(Box::new(ChatConversationTransform))
            }

            // Google streams
            ("stream_google_gmail", "social_email") => Ok(Box::new(GmailEmailTransform)),

            ("stream_google_calendar", "activity_calendar_entry") => {
                Ok(Box::new(GoogleCalendarTransform))
            }

            // Notion streams
            ("stream_notion_pages", "knowledge_document") => Ok(Box::new(NotionPageTransform)),

            // iOS microphone (requires API key + storage)
            ("stream_ios_microphone", "speech_transcription") => {
                let api_key = context.api_keys.assemblyai_required()?.to_string();
                let storage = (*context.storage).clone();
                Ok(Box::new(MicrophoneTranscriptionTransform::new(
                    api_key, storage,
                )))
            }

            // iOS location
            ("stream_ios_location", "location_point") => Ok(Box::new(IosLocationTransform)),

            // iOS HealthKit (one source, multiple targets)
            ("stream_ios_healthkit", "health_heart_rate") => {
                Ok(Box::new(HealthKitHeartRateTransform))
            }

            ("stream_ios_healthkit", "health_hrv") => Ok(Box::new(HealthKitHRVTransform)),

            ("stream_ios_healthkit", "health_steps") => Ok(Box::new(HealthKitStepsTransform)),

            ("stream_ios_healthkit", "health_sleep") => Ok(Box::new(HealthKitSleepTransform)),

            ("stream_ios_healthkit", "health_workout") => {
                Ok(Box::new(HealthKitWorkoutTransform))
            }

            // Unknown mapping
            _ => Err(Error::InvalidInput(format!(
                "Unknown transform mapping: {} -> {}. \
                 Check transforms/factory.rs for supported transforms.",
                source_table, target_table
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::ApiKeys;
    use crate::storage::Storage;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_context() -> TransformContext {
        let storage = Storage::local("/tmp/test").unwrap();
        let stream_writer = Arc::new(Mutex::new(crate::storage::stream_writer::StreamWriter::new()));
        let api_keys = ApiKeys::from_env();

        TransformContext::new(Arc::new(storage), stream_writer, api_keys)
    }

    #[test]
    fn test_create_gmail_transform() {
        let context = create_test_context();
        let result = TransformFactory::create("stream_google_gmail", "social_email", &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_location_transform() {
        let context = create_test_context();
        let result = TransformFactory::create("stream_ios_location", "location_point", &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_transform() {
        let context = create_test_context();
        let result = TransformFactory::create("invalid_source", "invalid_target", &context);
        assert!(result.is_err());
    }
}
