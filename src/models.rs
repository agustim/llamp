use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Backend {
    pub id: i64,
    pub provider_type: String,
    pub display_name: String,
    pub model_alias: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub api_key: Option<String>,
    pub additional_config: Option<String>,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub max_request_timeout_s: Option<i32>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewBackend {
    pub provider_type: String,
    pub display_name: String,
    pub model_alias: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub api_key: Option<String>,
    pub additional_config: Option<String>,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub max_request_timeout_s: Option<i32>,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub proxy_key: String,
    pub enabled: bool,
    pub allowed_backends: Option<String>,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub monthly_token_budget: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewUser {
    pub username: String,
    pub proxy_key: String,
    pub enabled: bool,
    pub allowed_backends: Option<String>,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub monthly_token_budget: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub model_alias: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: Option<i64>,
    pub cost: Option<f64>,
    pub status: String,
    pub error_message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewUsageLog {
    pub user_id: Option<i64>,
    pub model_alias: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: Option<i64>,
    pub cost: Option<f64>,
    pub status: String,
    pub error_message: Option<String>,
}

// OpenAI API compatible structures
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    // Add other OpenAI API fields as needed
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}