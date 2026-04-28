pub mod openai;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::models::{ChatCompletionRequest};
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIStreamChunk {
    pub id: String,
    pub object: String,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Transform an OpenAI request to the provider's format
    async fn prepare_request(&self, req: &ChatCompletionRequest, backend: &crate::models::Backend) -> Result<reqwest::Request>;

    /// Parse a streaming chunk from the provider and return it in OpenAI format
    fn parse_streaming_chunk(&self, raw: &[u8]) -> Result<Option<OpenAIStreamChunk>>;

    /// Extract usage from a non-streaming response
    fn parse_usage(&self, body: &Value) -> Option<crate::models::Usage>;

    /// Return the content type expected by the provider
    fn content_type(&self) -> &str;
}