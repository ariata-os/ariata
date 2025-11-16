//! Enrichment Transforms: Primitive → Semantic Primitive
//!
//! These transforms operate on ontology primitives to derive semantic meaning:
//! - Clustering/aggregation (location_point → location_visit)
//! - Entity resolution (social_* → entities_person)
//! - Semantic inference (speech_transcription → topics)
//!
//! All transforms in this module are **provider-agnostic** - they work with
//! normalized ontology primitives regardless of which source provider created them.

pub mod location;
