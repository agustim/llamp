use crate::models::{ChatCompletionRequest, Usage};
use crate::providers::{LLMProvider, Result, ProviderError, OpenAIStreamChunk};
use reqwest::{Client, Method};
use serde_json::Value;
use reqwest::header::HeaderValue;

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
        req: &ChatCompletionRequest,
        backend: &crate::models::Backend,
    ) -> Result<reqwest::Request> {
        let client = Client::new();

        // Log the backend information
        tracing::debug!(
            backend_model_name = backend.model_name,
            backend_endpoint = backend.endpoint_url,
            "Forwarding request to backend"
        );

        // Create the request body for the provider
        // Respect the stream parameter from the original request
        let stream = req.stream.unwrap_or(false);
        let mut body = serde_json::json!({
            "model": backend.model_name,
            "messages": req.messages,
            "stream": stream,
        });

        // Add optional parameters if they exist in the original request
        if let Some(temperature) = req.temperature {
            body["temperature"] = serde_json::json!(temperature);
        }
        if let Some(max_tokens) = req.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        // Log the body being sent (truncate if too large)
        if tracing::enabled!(tracing::Level::DEBUG) {
            let body_str = serde_json::to_string(&body).unwrap_or_default();
            let preview = if body_str.len() > 500 {
                format!("{}... ({} chars)", &body_str[..500], body_str.len())
            } else {
                body_str
            };
            tracing::debug!(body = preview, "Request body to backend");
        }

        // Determine the correct path based on the request type
        let endpoint = format!("{}/chat/completions", backend.endpoint_url);

        // Create the request to the provider
        let mut request = client
            .request(Method::POST, &endpoint)
            .header("Content-Type", "application/json")
            .json(&body)
            .build()?;

        // Add the API key header if available
        if let Some(api_key) = &backend.api_key {
            let auth_value = HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| ProviderError::Other(anyhow::anyhow!("Invalid header value: {}", e)))?;
            request.headers_mut().insert(
                "Authorization",
                auth_value,
            );
        }

        Ok(request)
    }

    /// Parse a streaming chunk from the provider and return it in OpenAI format
    fn parse_streaming_chunk(&self, raw: &[u8]) -> Result<Option<OpenAIStreamChunk>> {
        // SSE format: "data: {...}\n\n"
        let line = std::str::from_utf8(raw)
            .map_err(|e| ProviderError::Other(anyhow::anyhow!("Invalid UTF-8: {}", e)))?;

        if !line.starts_with("data: ") {
            return Ok(None);
        }

        let json_str = line.trim_start_matches("data: ").trim();

        // Skip empty lines and "[DONE]" marker
        if json_str.is_empty() || json_str == "[DONE]" {
            return Ok(None);
        }

        // Log the raw chunk for debugging
        tracing::debug!(chunk = json_str, "Parsing streaming chunk");

        let chunk: OpenAIStreamChunk = serde_json::from_str(json_str)
            .map_err(|e| ProviderError::JsonError(e))?;

        tracing::debug!(chunk = ?chunk, "Parsed streaming chunk");
        Ok(Some(chunk))
    }

    /// Extract usage from a non-streaming response
    fn parse_usage(&self, body: &Value) -> Option<Usage> {
        // Try to extract usage from the response
        if let Some(usage) = body.get("usage") {
            if let (Some(prompt_tokens), Some(completion_tokens), Some(total_tokens)) = (
                usage.get("prompt_tokens").and_then(|v| v.as_i64()),
                usage.get("completion_tokens").and_then(|v| v.as_i64()),
                usage.get("total_tokens").and_then(|v| v.as_i64()),
            ) {
                return Some(Usage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                });
            }
        }
        None
    }

    /// Return the content type expected by the provider
    fn content_type(&self) -> &str {
        "application/json"
    }
}
