//! LLM Client Module
//!
//! Provides integration with LLM providers via Vercel AI Gateway for context curation
//! and other AI-powered features.
//!
//! Uses OpenAI-compatible API to support multiple providers:
//! - Anthropic (anthropic/claude-sonnet-4)
//! - OpenAI (openai/gpt-4)
//! - And other supported providers

pub mod client;

pub use client::{AIGatewayClient, LLMClient, LLMRequest, LLMResponse};
