use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

/// Source configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    pub id: Uuid,
    pub provider: String,
    pub name: String,
    pub auth_type: String,
    pub is_active: bool,
    pub is_internal: bool,
}

/// Sources JSON structure
#[derive(Debug, Deserialize)]
struct SourcesJson {
    #[allow(dead_code)]
    version: String,
    sources: Vec<SourceConfig>,
}

/// Stream configuration from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamConfig {
    pub id: Uuid,
    pub source_id: Uuid,
    pub stream_name: String,
    pub table_name: String,
    pub is_enabled: bool,
}

/// Streams JSON structure
#[derive(Debug, Deserialize)]
struct StreamsJson {
    #[allow(dead_code)]
    version: String,
    streams: Vec<StreamConfig>,
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
    pub pinned_tool_ids: Vec<String>,
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

/// Load source configurations from JSON
pub fn load_sources() -> Result<Vec<SourceConfig>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/sources.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read sources.json from {}", path))?;
    let config: SourcesJson = serde_json::from_str(&content)
        .context("Failed to parse sources.json")?;
    Ok(config.sources)
}

/// Load stream configurations from JSON
pub fn load_streams() -> Result<Vec<StreamConfig>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/seeds/streams.json");
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read streams.json from {}", path))?;
    let config: StreamsJson = serde_json::from_str(&content)
        .context("Failed to parse streams.json")?;
    Ok(config.streams)
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
}
