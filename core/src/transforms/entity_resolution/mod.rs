//! Entity Resolution Transforms: Semantic Primitive → Entity Link
//!
//! These transforms link semantic primitives to canonical entity records:
//! - Location visits → Places (reverse geocoding)
//! - Social messages → People (contact resolution)
//! - Transactions → Merchants (merchant identification)
//!
//! All transforms in this module are **cross-cutting** - they bridge primitives
//! with entity tables to create the knowledge graph.

pub mod location;

pub use location::LocationPlaceResolutionTransform;
