//! LLM Client Implementation
//!
//! Provides HTTP client for Vercel AI Gateway (OpenAI-compatible API)
//! Supports multiple providers including Anthropic, OpenAI, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;

/// LLM request structure
#[derive(Debug, Clone, Serialize)]
pub struct LLMRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// LLM response structure
#[derive(Debug, Clone, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// LLM client trait for abstraction
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, String>;
}

/// Vercel AI Gateway client (OpenAI-compatible API)
/// Supports routing to multiple providers: Anthropic, OpenAI, etc.
#[derive(Clone)]
pub struct AIGatewayClient {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
}

impl AIGatewayClient {
    /// Create a new AI Gateway client from environment
    /// Expects AI_GATEWAY_API_KEY environment variable
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("AI_GATEWAY_API_KEY")
            .map_err(|_| "AI_GATEWAY_API_KEY not set in environment".to_string())?;

        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://ai-gateway.vercel.sh/v1".to_string(),
        })
    }

    /// Create a new AI Gateway client with explicit API key
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://ai-gateway.vercel.sh/v1".to_string(),
        }
    }

    /// Create with custom base URL (for testing or self-hosted)
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url,
        }
    }
}

// OpenAI-compatible API structures
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    object: String,
    #[allow(dead_code)]
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: OpenAIUsage,
}

#[derive(Deserialize)]
struct Choice {
    #[allow(dead_code)]
    index: u32,
    message: Message,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[allow(dead_code)]
    total_tokens: u32,
}

#[async_trait]
impl LLMClient for AIGatewayClient {
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, String> {
        // Build messages array
        let mut messages = Vec::new();

        // Add system message if provided
        if let Some(system) = request.system {
            messages.push(Message {
                role: "system".to_string(),
                content: system,
            });
        }

        // Add user message
        messages.push(Message {
            role: "user".to_string(),
            content: request.prompt,
        });

        let chat_request = ChatCompletionRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            return Err(format!("AI Gateway API error ({}): {}", status, error_text));
        }

        let chat_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract content from first choice
        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| "No choices in response".to_string())?
            .message
            .content
            .clone();

        Ok(LLMResponse {
            content,
            model: chat_response.model,
            usage: Usage {
                input_tokens: chat_response.usage.prompt_tokens,
                output_tokens: chat_response.usage.completion_tokens,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_gateway_client_creation() {
        env::set_var("AI_GATEWAY_API_KEY", "test-key");
        let client = AIGatewayClient::from_env();
        assert!(client.is_ok());
    }

    #[test]
    fn test_missing_api_key() {
        env::remove_var("AI_GATEWAY_API_KEY");
        let client = AIGatewayClient::from_env();
        assert!(client.is_err());
    }

    #[test]
    fn test_custom_base_url() {
        let client = AIGatewayClient::with_base_url(
            "test-key".to_string(),
            "https://custom-gateway.example.com/v1".to_string(),
        );
        assert_eq!(client.base_url, "https://custom-gateway.example.com/v1");
    }
}
