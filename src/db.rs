use sqlx::sqlite::SqlitePool;
use crate::models::{Backend, NewBackend, User, NewUser, UsageLog, NewUsageLog};

pub async fn init(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = sqlx::SqlitePool::connect(database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

// Database operations for backends
pub async fn get_backend_by_alias(pool: &SqlitePool, alias: &str) -> anyhow::Result<Option<Backend>> {
    let result = sqlx::query_as::<_, Backend>(
        "SELECT id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                active, created_at, updated_at
         FROM backends WHERE model_alias = ? AND active = TRUE"
    )
    .bind(alias)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn get_all_backends(pool: &SqlitePool) -> anyhow::Result<Vec<Backend>> {
    let result = sqlx::query_as::<_, Backend>(
        "SELECT id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                active, created_at, updated_at
         FROM backends WHERE active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    Ok(result)
}

pub async fn create_backend(pool: &SqlitePool, backend: NewBackend) -> anyhow::Result<Backend> {
    let result = sqlx::query_as::<_, Backend>(
        "INSERT INTO backends (provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                              additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s, active)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                   additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                   active, created_at, updated_at"
    )
    .bind(&backend.provider_type)
    .bind(&backend.display_name)
    .bind(&backend.model_alias)
    .bind(&backend.model_name)
    .bind(&backend.endpoint_url)
    .bind(&backend.api_key)
    .bind(&backend.additional_config)
    .bind(backend.cost_per_input_token)
    .bind(backend.cost_per_output_token)
    .bind(backend.max_request_timeout_s)
    .bind(backend.active)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

// User operations
pub async fn get_user_by_proxy_key(pool: &SqlitePool, proxy_key: &str) -> anyhow::Result<Option<User>> {
    let result = sqlx::query_as::<_, User>(
        "SELECT id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute,
                monthly_token_budget, created_at, updated_at
         FROM users WHERE proxy_key = ?"
    )
    .bind(proxy_key)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn create_user(pool: &SqlitePool, user: NewUser) -> anyhow::Result<User> {
    let result = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute, monthly_token_budget)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute, monthly_token_budget, created_at, updated_at"
    )
    .bind(&user.username)
    .bind(&user.proxy_key)
    .bind(user.enabled)
    .bind(&user.allowed_backends)
    .bind(user.rate_limit_requests_per_minute)
    .bind(user.monthly_token_budget)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

// Usage log operations
pub async fn create_usage_log(pool: &SqlitePool, log: NewUsageLog) -> anyhow::Result<UsageLog> {
    let result = sqlx::query_as::<_, UsageLog> (
        "INSERT INTO usage_logs (user_id, model_alias, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, status, error_message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, user_id, model_alias, prompt_tokens, completion_tokens, total_tokens, latency_ms, cost, status, error_message, timestamp"
    )
    .bind(log.user_id)
    .bind(&log.model_alias)
    .bind(log.prompt_tokens)
    .bind(log.completion_tokens)
    .bind(log.total_tokens)
    .bind(log.latency_ms)
    .bind(log.cost)
    .bind(&log.status)
    .bind(&log.error_message)
    .fetch_one(pool)
    .await?;

    Ok(result)
}