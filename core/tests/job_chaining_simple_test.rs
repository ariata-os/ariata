//! Simple unit tests for job chaining functionality
//!
//! Tests the data structures and basic logic without needing a database.

use ariata::sources::base::{ChainedTransform, TransformResult};
use uuid::Uuid;

#[test]
fn test_chained_transform_creation() {
    let source_record_id = Uuid::new_v4();

    let chained_transform = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec![
            "introspection_journal".to_string(),
            "social_interaction".to_string(),
        ],
        domain: "mixed".to_string(),
        source_record_id,
        transform_stage: "structuring".to_string(),
    };

    assert_eq!(chained_transform.source_table, "content_transcription");
    assert_eq!(chained_transform.target_tables.len(), 2);
    assert_eq!(chained_transform.target_tables[0], "introspection_journal");
    assert_eq!(chained_transform.target_tables[1], "social_interaction");
    assert_eq!(chained_transform.domain, "mixed");
    assert_eq!(chained_transform.source_record_id, source_record_id);
    assert_eq!(chained_transform.transform_stage, "structuring");
}

#[test]
fn test_transform_result_with_chaining() {
    let source_record_id = Uuid::new_v4();

    let chained_transform = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec!["introspection_journal".to_string()],
        domain: "introspection".to_string(),
        source_record_id,
        transform_stage: "journal_extraction".to_string(),
    };

    let result = TransformResult {
        records_read: 5,
        records_written: 5,
        records_failed: 0,
        last_processed_id: Some(source_record_id),
        chained_transforms: vec![chained_transform.clone()],
    };

    assert_eq!(result.records_read, 5);
    assert_eq!(result.records_written, 5);
    assert_eq!(result.records_failed, 0);
    assert_eq!(result.last_processed_id, Some(source_record_id));
    assert_eq!(result.chained_transforms.len(), 1);

    let chain = &result.chained_transforms[0];
    assert_eq!(chain.source_table, "content_transcription");
    assert_eq!(chain.transform_stage, "journal_extraction");
}

#[test]
fn test_single_stage_transform() {
    // Test a simple 1-stage transform (no chaining needed)
    let result = TransformResult {
        records_read: 10,
        records_written: 10,
        records_failed: 0,
        last_processed_id: Some(Uuid::new_v4()),
        chained_transforms: vec![], // No follow-up transforms
    };

    assert_eq!(result.chained_transforms.len(), 0);
    assert_eq!(result.records_written, 10);
    assert!(result.last_processed_id.is_some());
}

#[test]
fn test_two_stage_transform_pipeline() {
    // Test: Stage 1 (audio → transcription) spawns Stage 2 (transcription → journal)
    let audio_id = Uuid::new_v4();
    let transcript_id = Uuid::new_v4();

    // Stage 1 result: Audio transcription complete, spawn structuring job
    let stage1_result = TransformResult {
        records_read: 1,
        records_written: 1,
        records_failed: 0,
        last_processed_id: Some(audio_id),
        chained_transforms: vec![ChainedTransform {
            source_table: "content_transcription".to_string(),
            target_tables: vec!["introspection_journal".to_string()],
            domain: "introspection".to_string(),
            source_record_id: transcript_id,
            transform_stage: "structuring".to_string(),
        }],
    };

    assert_eq!(stage1_result.chained_transforms.len(), 1);
    assert_eq!(
        stage1_result.chained_transforms[0].transform_stage,
        "structuring"
    );

    // Stage 2 result: Structuring complete, no more chaining
    let stage2_result = TransformResult {
        records_read: 1,
        records_written: 1,
        records_failed: 0,
        last_processed_id: Some(transcript_id),
        chained_transforms: vec![], // Final stage
    };

    assert_eq!(stage2_result.chained_transforms.len(), 0);
}

#[test]
fn test_three_stage_transform_pipeline() {
    // Test: Stage 1 → Stage 2 → Stage 3
    let audio_id = Uuid::new_v4();
    let transcript_id = Uuid::new_v4();
    let journal_id = Uuid::new_v4();

    // Stage 1: Audio → Transcription
    let stage1 = TransformResult {
        records_read: 1,
        records_written: 1,
        records_failed: 0,
        last_processed_id: Some(audio_id),
        chained_transforms: vec![ChainedTransform {
            source_table: "content_transcription".to_string(),
            target_tables: vec!["introspection_journal".to_string()],
            domain: "introspection".to_string(),
            source_record_id: transcript_id,
            transform_stage: "journal_extraction".to_string(),
        }],
    };

    assert_eq!(stage1.chained_transforms.len(), 1);
    assert_eq!(stage1.chained_transforms[0].transform_stage, "journal_extraction");

    // Stage 2: Transcription → Journal (spawns entity resolution)
    let stage2 = TransformResult {
        records_read: 1,
        records_written: 1,
        records_failed: 0,
        last_processed_id: Some(transcript_id),
        chained_transforms: vec![ChainedTransform {
            source_table: "introspection_journal".to_string(),
            target_tables: vec!["entities_topic".to_string()],
            domain: "entities".to_string(),
            source_record_id: journal_id,
            transform_stage: "topic_extraction".to_string(),
        }],
    };

    assert_eq!(stage2.chained_transforms.len(), 1);
    assert_eq!(stage2.chained_transforms[0].transform_stage, "topic_extraction");

    // Stage 3: Journal → Topic entities (final stage)
    let stage3 = TransformResult {
        records_read: 1,
        records_written: 2,
        records_failed: 0,
        last_processed_id: Some(journal_id),
        chained_transforms: vec![], // No more chaining
    };

    assert_eq!(stage3.chained_transforms.len(), 0);
}

#[test]
fn test_multiple_chained_transforms() {
    // Test that a single transform can spawn multiple child transforms
    let transcript_id = Uuid::new_v4();

    let chain1 = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec!["introspection_journal".to_string()],
        domain: "introspection".to_string(),
        source_record_id: transcript_id,
        transform_stage: "journal_extraction".to_string(),
    };

    let chain2 = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec!["social_interaction".to_string()],
        domain: "social".to_string(),
        source_record_id: transcript_id,
        transform_stage: "interaction_extraction".to_string(),
    };

    let chain3 = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec!["entities_person".to_string(), "entities_place".to_string()],
        domain: "entities".to_string(),
        source_record_id: transcript_id,
        transform_stage: "entity_resolution".to_string(),
    };

    let result = TransformResult {
        records_read: 1,
        records_written: 1,
        records_failed: 0,
        last_processed_id: Some(transcript_id),
        chained_transforms: vec![chain1, chain2, chain3],
    };

    assert_eq!(result.chained_transforms.len(), 3);

    // Verify stages are different
    assert_eq!(result.chained_transforms[0].transform_stage, "journal_extraction");
    assert_eq!(result.chained_transforms[1].transform_stage, "interaction_extraction");
    assert_eq!(result.chained_transforms[2].transform_stage, "entity_resolution");

    // Verify all reference the same source record
    assert_eq!(result.chained_transforms[0].source_record_id, transcript_id);
    assert_eq!(result.chained_transforms[1].source_record_id, transcript_id);
    assert_eq!(result.chained_transforms[2].source_record_id, transcript_id);
}

#[test]
fn test_chained_transform_target_tables() {
    // Test that a single chained transform can have multiple target tables
    let chained = ChainedTransform {
        source_table: "content_transcription".to_string(),
        target_tables: vec![
            "entities_person".to_string(),
            "entities_place".to_string(),
            "entities_topic".to_string(),
        ],
        domain: "entities".to_string(),
        source_record_id: Uuid::new_v4(),
        transform_stage: "entity_resolution".to_string(),
    };

    assert_eq!(chained.target_tables.len(), 3);
    assert!(chained.target_tables.contains(&"entities_person".to_string()));
    assert!(chained.target_tables.contains(&"entities_place".to_string()));
    assert!(chained.target_tables.contains(&"entities_topic".to_string()));
}
