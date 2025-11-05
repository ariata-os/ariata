//! Ariata App Source
//!
//! Internal source for exporting operational data from the web application
//! to the ELT pipeline. Unlike external sources (Google, Notion), this source
//! reads from the app schema instead of calling external APIs.

pub mod export;
pub mod registry;

pub use export::AppChatExportStream;
pub use registry::AriataAppSource;
