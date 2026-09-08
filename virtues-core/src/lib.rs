//! Virtues - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod applet_git_import;
pub mod applet_runner;
pub mod applet_templates;
pub mod agent;
pub mod api;
pub mod cli;
pub mod client;
pub mod codename;
pub mod credentials;
pub mod crypto;
pub mod data_disk;
pub mod database;
pub mod dayline;
pub mod entity_resolution;
pub mod error;
pub mod extraction;
pub mod fetch;
pub mod geo;
pub mod http_client;
pub mod ids;
pub mod inference_report;
pub mod install_manifest;
pub mod magnet;
pub mod maintenance;
pub mod middleware;
pub mod bookmark_enrichment;
pub mod box_secrets;
pub mod net_check;
pub mod peer_addr;
pub mod relay;
pub mod scheduler;
pub mod search;
pub mod seeding;
pub mod sessionize;
pub mod server;
pub mod setup;
pub mod storage;
pub mod timezone;
pub mod virtues_api;
pub mod tools;
pub mod types;

// Re-export main types
pub use client::{Virtues, VirtuesBuilder};
pub use error::{Error, Result};
pub use types::Timestamp;

// Re-export Scheduler
pub use scheduler::Scheduler;

// Re-export tools
pub use tools::{get_tool_definitions_for_llm, ToolContext, ToolError, ToolExecutor, ToolResult};

// Re-export library API functions
pub use api::{
    // Credential management (post-Phase-6: pair flows live in source_auth.rs)
    check_pairing_status,
    list_pending_pairings,
    DeviceInfo,
    PairingStatus,
    PendingPairing,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    /// VERSION is well-formed semver.
    ///
    /// This used to be `assert_eq!(VERSION, "0.1.0")` — a test that asserts the
    /// version is a particular old number, so it fails on every release and
    /// teaches whoever hits it to edit the literal without thinking. It sat at
    /// 0.1.0 through the whole v0.2.0 release for exactly that reason, and then
    /// went red the moment the crate was versioned for v0.3.0.
    ///
    /// What is actually worth protecting is the shape, because
    /// `GET /api/system/update` compares this string against the newest release
    /// tag to decide whether a box is behind. A malformed or empty version
    /// makes that comparison meaningless rather than merely wrong.
    #[test]
    fn version_is_semver() {
        assert!(!VERSION.is_empty(), "VERSION must not be empty");

        let parts: Vec<&str> = VERSION.split('-').next().unwrap().split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "VERSION must be major.minor.patch, got {VERSION:?}"
        );
        for p in parts {
            assert!(
                !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()),
                "VERSION component {p:?} is not numeric in {VERSION:?}"
            );
        }
    }
}
