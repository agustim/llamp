use axum::{body::Body, extract::Extension, http::Request, middleware::Next, response::Response};
use chrono::Utc;
use serde_json::json;
use std::time::Instant;

// Import the models
use crate::models::User;

/// Logging middleware that captures all HTTP requests and responses
pub async fn logging_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    // Extract user if authenticated
    let user_id = request.extensions().get::<User>()
        .map(|u| u.id)
        .unwrap_or(0);

    // Log request start
    tracing::info!(
        method = %method,
        uri = %uri,
        user_id = user_id,
        "Request started"
    );

    // Process the request
    let response = next.run(request).await;

    let elapsed = start_time.elapsed();
    let status = response.status();

    // Log response
    tracing::info!(
        method = %method,
        uri = %uri,
        user_id = user_id,
        status = %status.as_u16(),
        duration_ms = elapsed.as_millis(),
        "Request completed"
    );

    response
}

/// Struct to hold request/response logging data
#[derive(Debug, Clone)]
pub struct RequestLog {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub user_id: i64,
    pub status_code: u16,
    pub duration_ms: u128,
    pub model_alias: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub error: Option<String>,
}

impl RequestLog {
    pub fn new(
        method: String,
        path: String,
        user_id: i64,
    ) -> Self {
        RequestLog {
            timestamp: Utc::now().to_rfc3339(),
            method,
            path,
            user_id,
            status_code: 0,
            duration_ms: 0,
            model_alias: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            error: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model_alias = Some(model);
        self
    }

    pub fn with_tokens(mut self, prompt: i64, completion: i64, total: i64) -> Self {
        self.prompt_tokens = Some(prompt);
        self.completion_tokens = Some(completion);
        self.total_tokens = Some(total);
        self
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status_code = status;
        self
    }

    pub fn with_duration(mut self, duration_ms: u128) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "timestamp": self.timestamp,
            "method": self.method,
            "path": self.path,
            "user_id": self.user_id,
            "status_code": self.status_code,
            "duration_ms": self.duration_ms,
            "model_alias": self.model_alias,
            "prompt_tokens": self.prompt_tokens,
            "completion_tokens": self.completion_tokens,
            "total_tokens": self.total_tokens,
            "error": self.error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_log_creation() {
        let log = RequestLog::new("POST".to_string(), "/v1/chat/completions".to_string(), 123);
        assert_eq!(log.method, "POST");
        assert_eq!(log.path, "/v1/chat/completions");
        assert_eq!(log.user_id, 123);
    }

    #[test]
    fn test_request_log_with_tokens() {
        let log = RequestLog::new("POST".to_string(), "/v1/chat/completions".to_string(), 123)
            .with_tokens(100, 50, 150);
        
        assert_eq!(log.prompt_tokens, Some(100));
        assert_eq!(log.completion_tokens, Some(50));
        assert_eq!(log.total_tokens, Some(150));
    }

    #[test]
    fn test_request_log_to_json() {
        let log = RequestLog::new("POST".to_string(), "/v1/chat/completions".to_string(), 123)
            .with_model("gpt-4".to_string())
            .with_tokens(100, 50, 150)
            .with_status(200)
            .with_duration(1234);

        let json = log.to_json();
        assert_eq!(json["model_alias"], "gpt-4");
        assert_eq!(json["total_tokens"], 150);
        assert_eq!(json["duration_ms"], 1234);
    }
}
