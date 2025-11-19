use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

/// Source connection seed configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConnectionSeed {
    pub id: Uuid,
    pub source: String,
    pub name: String,
    pub auth_type: String,
    pub is_active: bool,
    pub is_internal: bool,
}

/// Source connections JSON structure
#[derive(Debug, Deserialize)]
struct SourceConnectionsJson {
    #[allow(dead_code)]
    version: String,
    connections: Vec<SourceConnectionSeed>,
}

/// Stream connection seed configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamConnectionSeed {
    pub id: Uuid,
    pub source_connection_id: Uuid,
    pub stream_name: String,
    pub table_name: String,
    pub is_enabled: bool,
    /// Optional cron schedule override (if None, will use registry default)
    pub cron_schedule: Option<String>,
}

/// Stream connections JSON structure
#[derive(Debug, Deserialize)]
struct StreamConnectionsJson {
    #[allow(dead_code)]
    version: String,
    connections: Vec<StreamConnectionSeed>,
}

/// Model configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub context_window: i32,
    pub max_output_tokens: i32,
    pub supports_tools: bool,
    #[serde(default)]
    pub is_default: bool,
}

/// Models JSON structure
#[derive(Debug, Deserialize)]
struct ModelsJson {
    #[allow(dead_code)]
    version: String,
    models: Vec<ModelConfig>,
}

/// Agent configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub default_model: String,
    pub max_steps: i32,
    pub enabled: bool,
    pub sort_order: i32,
}

/// Agents JSON structure
#[derive(Debug, Deserialize)]
struct AgentsJson {
    #[allow(dead_code)]
    version: String,
    agents: Vec<AgentConfig>,
}

/// Tool configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tool_type: String,
    pub category: String,
    pub icon: String,
    pub is_pinnable: bool,
    pub display_order: i32,
}

/// Tools JSON structure
#[derive(Debug, Deserialize)]
struct ToolsJson {
    #[allow(dead_code)]
    version: String,
    tools: Vec<ToolConfig>,
    default_enabled_tools: serde_json::Value,
}

/// Load model configurations from JSON
pub fn load_models() -> Result<Vec<ModelConfig>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/models.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read models.json from {}", path))?;
    let config: ModelsJson = serde_json::from_str(&content)
        .context("Failed to parse models.json")?;
    Ok(config.models)
}

/// Load agent configurations from JSON
pub fn load_agents() -> Result<Vec<AgentConfig>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/agents.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read agents.json from {}", path))?;
    let config: AgentsJson = serde_json::from_str(&content)
        .context("Failed to parse agents.json")?;
    Ok(config.agents)
}

/// Load tool configurations from JSON
pub fn load_tools() -> Result<(Vec<ToolConfig>, serde_json::Value)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/tools.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read tools.json from {}", path))?;
    let config: ToolsJson = serde_json::from_str(&content)
        .context("Failed to parse tools.json")?;
    Ok((config.tools, config.default_enabled_tools))
}

/// Assistant profile defaults from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistantProfileDefaults {
    pub assistant_name: Option<String>,
    pub default_agent_id: String,
    pub default_model_id: String,
    pub enabled_tools: serde_json::Value,
    pub ui_preferences: serde_json::Value,
}

/// Assistant profile JSON structure
#[derive(Debug, Deserialize)]
struct AssistantProfileJson {
    #[allow(dead_code)]
    version: String,
    defaults: AssistantProfileDefaults,
}

/// Load assistant profile defaults from JSON
pub fn load_assistant_profile_defaults() -> Result<AssistantProfileDefaults> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/assistant_profile.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read assistant_profile.json from {}", path))?;
    let config: AssistantProfileJson = serde_json::from_str(&content)
        .context("Failed to parse assistant_profile.json")?;
    Ok(config.defaults)
}

/// Load source connection configurations from JSON
pub fn load_source_connections() -> Result<Vec<SourceConnectionSeed>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/_generated_source_connections.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read _generated_source_connections.json from {}", path))?;
    let config: SourceConnectionsJson = serde_json::from_str(&content)
        .context("Failed to parse _generated_source_connections.json")?;
    Ok(config.connections)
}

/// Load stream connection configurations from JSON
pub fn load_stream_connections() -> Result<Vec<StreamConnectionSeed>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/_generated_stream_connections.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read _generated_stream_connections.json from {}", path))?;
    let config: StreamConnectionsJson = serde_json::from_str(&content)
        .context("Failed to parse _generated_stream_connections.json")?;
    Ok(config.connections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_models() {
        let models = load_models().expect("Failed to load models");
        assert!(!models.is_empty(), "Models should not be empty");

        // Verify all models have context windows
        for model in &models {
            assert!(model.context_window > 0, "Model {} should have context_window", model.model_id);
            assert!(model.max_output_tokens > 0, "Model {} should have max_output_tokens", model.model_id);
        }
    }

    #[test]
    fn test_load_agents() {
        let agents = load_agents().expect("Failed to load agents");
        assert!(!agents.is_empty(), "Agents should not be empty");

        // Verify all agents have required fields
        for agent in &agents {
            assert!(!agent.agent_id.is_empty());
            assert!(!agent.name.is_empty());
            assert!(!agent.default_model.is_empty());
        }
    }

    #[test]
    fn test_load_tools() {
        let (tools, defaults) = load_tools().expect("Failed to load tools");
        assert!(!tools.is_empty(), "Tools should not be empty");
        assert!(defaults.is_object(), "Default enabled tools should be an object");
    }

    #[test]
    fn test_load_assistant_profile_defaults() {
        let defaults = load_assistant_profile_defaults().expect("Failed to load assistant profile defaults");

        // Verify required fields
        assert!(!defaults.default_agent_id.is_empty(), "Default agent ID should not be empty");
        assert!(!defaults.default_model_id.is_empty(), "Default model ID should not be empty");
        assert!(defaults.enabled_tools.is_object(), "Enabled tools should be an object");
        assert!(defaults.ui_preferences.is_object(), "UI preferences should be an object");

        // Verify UI preferences structure
        let ui_prefs = &defaults.ui_preferences;
        assert!(ui_prefs.get("contextIndicator").is_some(), "Should have contextIndicator preferences");
    }

    /// CRITICAL VALIDATION: Verify seed configs match registry
    /// This catches mismatches between config/seeds/*.json and the Rust registry
    #[test]
    fn test_source_connections_match_registry() {
        let connections = load_source_connections().expect("Failed to load source connections");

        for conn in &connections {
            // Verify source exists in registry
            let registered_source = crate::registry::get_source(&conn.source)
                .unwrap_or_else(|| panic!(
                    "Source '{}' in _generated_source_connections.json not found in registry. \
                     Did you forget to register it in core/src/registry/mod.rs?",
                    conn.source
                ));

            // Verify auth_type matches
            let registry_auth_type = match registered_source.auth_type {
                crate::registry::AuthType::OAuth2 => "oauth2",
                crate::registry::AuthType::Device => "device",
                crate::registry::AuthType::ApiKey => "api_key",
                crate::registry::AuthType::None => "none",
            };

            assert_eq!(
                conn.auth_type, registry_auth_type,
                "Auth type mismatch for source '{}': config says '{}', registry says '{}'. \
                 Run 'make generate-seeds' to regenerate config from registry.",
                conn.source, conn.auth_type, registry_auth_type
            );
        }
    }

    /// CRITICAL VALIDATION: Verify stream configs match registry
    #[test]
    fn test_stream_connections_match_registry() {
        let source_connections = load_source_connections().expect("Failed to load source connections");
        let stream_connections = load_stream_connections().expect("Failed to load stream connections");

        for stream_conn in &stream_connections {
            // Find the parent source connection
            let source_conn = source_connections.iter()
                .find(|s| s.id == stream_conn.source_connection_id)
                .unwrap_or_else(|| panic!(
                    "Stream '{}' references unknown source_connection_id {}. \
                     Check _generated_stream_connections.json.",
                    stream_conn.stream_name, stream_conn.source_connection_id
                ));

            // Verify stream exists in registry
            let registered_stream = crate::registry::get_stream(&source_conn.source, &stream_conn.stream_name)
                .unwrap_or_else(|| panic!(
                    "Stream '{}' for source '{}' not found in registry. \
                     Available streams: {:?}. \
                     Run 'make generate-seeds' to regenerate config from registry.",
                    stream_conn.stream_name,
                    source_conn.source,
                    crate::registry::get_source(&source_conn.source)
                        .map(|s| s.streams.iter().map(|st| st.name).collect::<Vec<_>>())
                        .unwrap_or_default()
                ));

            // Verify table_name matches
            assert_eq!(
                stream_conn.table_name, registered_stream.table_name,
                "Table name mismatch for {}/{}: config says '{}', registry says '{}'. \
                 Run 'make generate-seeds' to regenerate config from registry.",
                source_conn.source, stream_conn.stream_name,
                stream_conn.table_name, registered_stream.table_name
            );
        }
    }

    /// Verify only internal sources are in seed config
    /// OAuth/Device sources should NOT be seeded (created via auth flows)
    #[test]
    fn test_only_internal_sources_in_seeds() {
        let connections = load_source_connections().expect("Failed to load source connections");

        for conn in &connections {
            let registry_source = crate::registry::get_source(&conn.source)
                .unwrap();

            if registry_source.auth_type != crate::registry::AuthType::None {
                panic!(
                    "Source '{}' has auth_type '{}' but is in seed config! \
                     Only internal sources (auth_type=None) should be seeded. \
                     OAuth/Device sources are created via auth flows. \
                     Run 'make generate-seeds' to regenerate config from registry.",
                    conn.source,
                    conn.auth_type
                );
            }

            assert!(
                conn.is_internal,
                "Source '{}' in seed config should have is_internal=true",
                conn.source
            );
        }
    }
}
