use crate::models::{ChatCompletionRequest, Usage};
use crate::providers::{LLMProvider, Result};
use serde_json::Value;

// Placeholder struct for OpenAIProvider (not actually used yet)
pub struct OpenAIProvider {}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    /// Transform an OpenAI request to the provider's format
    async fn prepare_request(
        &self,
        _req: &ChatCompletionRequest,
        _backend: &crate::models::Backend,
    ) -> Result<reqwest::Request> {
        // This is a placeholder implementation
        todo!("Implement OpenAI provider")
    }

    /// Parse a streaming chunk from the provider and return it in OpenAI format
    fn parse_streaming_chunk(
        &self,
        _raw: &[u8],
    ) -> Result<Option<crate::providers::OpenAIStreamChunk>> {
        // This is placeholder implementation
        todo!("Implement streaming chunk parsing")
    }

    /// Extract usage from a non-streaming response
    fn parse_usage(&self, _body: &Value) -> Option<Usage> {
        // This is a placeholder implementation
        None
    }

    /// Return the content type expected by the provider
    fn content_type(&self) -> &str {
        "application/json"
    }
}
