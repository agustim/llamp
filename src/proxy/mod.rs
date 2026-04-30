use axum::{
    body::Body,
    extract::{Extension, Json},
    http::StatusCode,
    middleware,
    response::Response,
    routing::{get, post},
    Router,
};
use serde_json::json;

// Import the models
use crate::models::{User, ChatCompletionRequest, NewUsageLog};

// Import the database module
use crate::db;

// Import the auth module
use crate::auth;

// Import the providers
use crate::providers::openai::OpenAIProvider;
use crate::providers::LLMProvider;

pub async fn create_app(log_level: String) -> anyhow::Result<Router> {
    Ok(Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health_check))
        // Admin routes
        .route("/admin/backends", get(list_backends).post(create_backend))
        .route(
            "/admin/backends/:id",
            get(get_backend).put(update_backend).delete(delete_backend),
        )
        .route("/admin/backends/:id/test", post(test_backend))
        .route("/admin/users", get(list_users).post(create_user))
        .route(
            "/admin/users/:id",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route("/admin/users/:id/regenerate-key", post(regenerate_user_key))
        .route("/admin/stats/overview", get(stats_overview))
        .route("/admin/stats/by-user", get(stats_by_user))
        .route("/admin/stats/by-model", get(stats_by_model))
        .route("/admin/logs", get(get_logs))
        // Apply auth middleware to protected routes
        .layer(middleware::from_fn(auth::auth_middleware)))
}

async fn chat_completions(
    Extension(user): Extension<User>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, (StatusCode, String)> {
    // Log the incoming request for debugging
    tracing::debug!(
        user_id = user.id,
        model = request.model,
        stream = ?request.stream,
        messages_count = request.messages.len(),
        "Received chat completion request"
    );

    // Log request details if in debug mode
    if tracing::enabled!(tracing::Level::DEBUG) {
        for (i, msg) in request.messages.iter().enumerate() {
            tracing::debug!(
                user_id = user.id,
                message_index = i,
                role = msg.role,
                content_length = msg.content.len(),
                "Message content"
            );
        }
    }

    // Create a database pool
    let pool = db::init("sqlite://./llamp.db").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to connect to database: {}", e)))?;

    // Get the backend for the requested model
    let backend = db::get_backend_by_alias(&pool, &request.model).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch backend: {}", e)))?
        .ok_or((StatusCode::NOT_FOUND, "Model not found".to_string()))?;

    // Create the OpenAI provider
    let provider = OpenAIProvider::new();

    // Prepare the request to the backend
    let backend_request = provider.prepare_request(&request, &backend).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to prepare request: {}", e)))?;

    tracing::debug!(
        user_id = user.id,
        model = request.model,
        backend_model = backend.model_name,
        "Request forwarded to backend"
    );

    // Forward the request to the backend
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();

    let backend_response = client.execute(backend_request).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to connect to backend: {}", e)))?;

    let elapsed = start_time.elapsed();

    // Get the status code before consuming the response
    let status = backend_response.status().as_u16();

    // Get the response body
    let response_body = backend_response.text().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to read backend response: {}", e)))?;

    // Log the backend response for debugging
    if tracing::enabled!(tracing::Level::DEBUG) {
        let response_preview = if response_body.len() > 500 {
            format!("{}... ({} chars)", &response_body[..500], response_body.len())
        } else {
            response_body.clone()
        };
        tracing::debug!(
            user_id = user.id,
            backend_status = status,
            response_preview = response_preview,
            "Backend response received"
        );
    }

    // If backend returned an error, pass it through
    if status >= 400 {
        tracing::warn!(
            user_id = user.id,
            backend_status = status,
            response_body_preview = %response_body.chars().take(200).collect::<String>(),
            "Backend returned error, passing through"
        );
        return Err((
            match status {
                400 => StatusCode::BAD_REQUEST,
                401 => StatusCode::UNAUTHORIZED,
                403 => StatusCode::FORBIDDEN,
                404 => StatusCode::NOT_FOUND,
                429 => StatusCode::TOO_MANY_REQUESTS,
                500 => StatusCode::INTERNAL_SERVER_ERROR,
                502 => StatusCode::BAD_GATEWAY,
                503 => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_GATEWAY,
            },
            response_body,
        ));
    }

    // Process the backend response (handles both streaming and non-streaming)
    let processed_response = process_backend_response(&response_body, &provider)?;

    // Parse the processed response
    let response_json: serde_json::Value = serde_json::from_str(&processed_response)
        .map_err(|e| {
            tracing::error!(
                user_id = user.id,
                processed_response_preview = %processed_response.chars().take(200).collect::<String>(),
                error = %e,
                "Failed to parse processed response"
            );
            (StatusCode::BAD_GATEWAY, format!("Failed to parse processed response: {}", e))
        })?;

    // Extract usage if available
    let usage = provider.parse_usage(&response_json);
    let prompt_tokens = usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
    let completion_tokens = usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
    let total_tokens = usage.as_ref().map(|u| u.total_tokens).unwrap_or(0);

    // Create a usage log entry
    let usage_log = NewUsageLog {
        user_id: Some(user.id),
        model_alias: Some(request.model.clone()),
        prompt_tokens,
        completion_tokens,
        total_tokens,
        latency_ms: Some(elapsed.as_millis() as i64),
        cost: None,
        status: "success".to_string(),
        error_message: None,
    };

    // Create the usage log in the database
    let _log = db::create_usage_log(&pool, usage_log).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create usage log: {}", e)))?;

    // Log the response body for debugging before building response
    if tracing::enabled!(tracing::Level::DEBUG) {
        let response_preview = if processed_response.len() > 500 {
            format!("{}... ({} chars)", &processed_response[..500], processed_response.len())
        } else {
            processed_response.clone()
        };
        tracing::debug!(
            user_id = user.id,
            response_preview = response_preview,
            "Final response built"
        );
    }

    // Build the response using the processed response body
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(processed_response))
        .unwrap();

    // Log successful response
    tracing::info!(
        user_id = user.id,
        model = request.model,
        status = 200,
        duration_ms = elapsed.as_millis(),
        prompt_tokens = prompt_tokens,
        completion_tokens = completion_tokens,
        "Chat completion response sent"
    );

    Ok(response)
}

/// Process backend response, handling both streaming and non-streaming
fn process_backend_response(
    response_body: &str,
    provider: &OpenAIProvider,
) -> Result<String, (StatusCode, String)> {
    // Check if response is streaming (contains "data: " prefix)
    if response_body.contains("data: ") {
        // Parse streaming chunks and build final response
        let mut final_content = String::new();
        let mut final_usage: Option<crate::models::Usage> = None;
        let mut last_model = String::new();
        let mut last_id = String::new();

        for line in response_body.split("\n\n") {
            let trimmed = line.trim();
            if trimmed.starts_with("data: ") {
                let json_str = trimmed.trim_start_matches("data: ");
                if json_str.is_empty() || json_str == "[DONE]" {
                    continue;
                }

                // Log the chunk for debugging
                tracing::debug!(chunk = json_str, "Processing streaming chunk");

                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(value) => {
                        match provider.parse_streaming_chunk(line.as_bytes()) {
                            Ok(Some(chunk)) => {
                                // Process chunk
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(delta) = &choice.delta {
                                        if let Some(content) = &delta.content {
                                            // Only add content if it's not empty
                                            // Empty content usually indicates the last chunk with just finish_reason
                                            if !content.is_empty() {
                                                tracing::debug!(content = content, "Adding content from chunk");
                                                final_content.push_str(content);
                                            } else {
                                                tracing::debug!("Skipping empty content chunk (likely finish_reason only)");
                                            }
                                        }
                                    }
                                    if let Some(reason) = &choice.finish_reason {
                                        tracing::debug!(finish_reason = reason, "Streaming finished");
                                    }
                                } else {
                                    tracing::debug!("Chunk has no choices or delta");
                                }
                                // Extract usage from chunk if available
                                if let Some(usage) = provider.parse_usage(&value) {
                                    final_usage = Some(usage);
                                }
                                last_model = value.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string();
                                last_id = value.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                            }
                            Ok(None) => {
                                tracing::debug!("Skipping chunk without data");
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to parse streaming chunk");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to parse chunk JSON");
                    }
                }
            }
        }

        // Build the final response
        let response = serde_json::json!({
            "id": last_id,
            "object": "chat.completion",
            "created": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "model": last_model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": final_content
                },
                "finish_reason": "stop"
            }],
            "usage": final_usage.unwrap_or_else(|| crate::models::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0
            })
        });

        // Log the final response for debugging
        if tracing::enabled!(tracing::Level::DEBUG) {
            let response_str = serde_json::to_string(&response).unwrap_or_default();
            let preview = if response_str.len() > 500 {
                format!("{}... ({} chars)", &response_str[..500], response_str.len())
            } else {
                response_str
            };
            tracing::debug!(final_response = preview, "Built final response");
        }

        Ok(serde_json::to_string(&response)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize response: {}", e)))?)
    } else {
        // Non-streaming response - return as-is
        Ok(response_body.to_string())
    }
}

async fn list_models() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Create a database pool
    let pool = db::init("sqlite://./llamp.db").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to connect to database: {}", e)))?;

    // Get all active backends
    let backends = db::get_all_backends(&pool).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch backends: {}", e)))?;

    // Transform backends into the expected JSON format
    let models: Vec<serde_json::Value> = backends.into_iter().map(|backend| {
        json!({
            "id": backend.model_alias,
            "object": "model",
            "created": 1234567890, // Placeholder timestamp
            "owned_by": "llamp"
        })
    }).collect();

    // Return the models as JSON
    Ok(Json(json!({
        "object": "list",
        "data": models
    })))
}

async fn health_check() -> &'static str {
    "Health check endpoint"
}

// Admin routes
async fn list_backends() -> &'static str {
    "List backends endpoint"
}

async fn create_backend() -> &'static str {
    "Create backend endpoint"
}

async fn get_backend() -> &'static str {
    "Get backend endpoint"
}

async fn update_backend() -> &'static str {
    "Update backend endpoint"
}

async fn delete_backend() -> &'static str {
    "Delete backend endpoint"
}

async fn test_backend() -> &'static str {
    "Test backend endpoint"
}

async fn list_users() -> &'static str {
    "List users endpoint"
}

async fn create_user() -> &'static str {
    "Create user endpoint"
}

async fn get_user() -> &'static str {
    "Get user endpoint"
}

async fn update_user() -> &'static str {
    "Update user endpoint"
}

async fn delete_user() -> &'static str {
    "Delete user endpoint"
}

async fn regenerate_user_key() -> &'static str {
    "Regenerate user key endpoint"
}

async fn stats_overview() -> &'static str {
    "Stats overview endpoint"
}

async fn stats_by_user() -> &'static str {
    "Stats by user endpoint"
}

async fn stats_by_model() -> &'static str {
    "Stats by model endpoint"
}

async fn get_logs() -> &'static str {
    "Get logs endpoint"
}
