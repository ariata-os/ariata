//! iOS device data sources
//!
//! Processors for data pushed from iOS devices including HealthKit,
//! Location, and Microphone streams.

pub mod healthkit;
pub mod location;
pub mod microphone;
pub mod registry;

pub use healthkit::process as process_healthkit;
pub use location::process as process_location;
pub use microphone::process as process_microphone;
pub use registry::IosSource;
