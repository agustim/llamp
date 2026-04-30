use crate::models::{Backend, NewBackend, NewUsageLog, NewUser, UpdateBackend, UpdateUser, UsageLog, User};
use sqlx::sqlite::SqlitePool;
use std::fs;

pub async fn init(database_url: &str) -> anyhow::Result<SqlitePool> {
    // Convert database URL to proper SQLite format with file: prefix for file-based DBs
    let pool_url = if database_url.starts_with("sqlite://") {
        let path = database_url.replace("sqlite://", "");
        // Remove ./ prefix and use file: prefix
        let clean_path = path.strip_prefix("./").unwrap_or(&path);
        format!("file:{}?mode=rwc", clean_path)
    } else {
        database_url.to_string()
    };

    // Determine if this is a file-based database
    let is_file_db = database_url.starts_with("sqlite://") || database_url.starts_with("./")
        || database_url.starts_with('/')
        || database_url.contains("llamp.db");

    let sqlite_path = if database_url.starts_with("sqlite://") {
        database_url.replace("sqlite://", "")
    } else {
        database_url.to_string()
    };

    // Create the database file if it doesn't exist
    let is_new_database = if is_file_db {
        let path_exists = std::path::Path::new(&sqlite_path).exists();
        if !path_exists {
            // Create the directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(&sqlite_path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            // Create an empty file
            fs::File::create(&sqlite_path)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    tracing::debug!("Pool URL: {}", pool_url);
    let pool = sqlx::SqlitePool::connect(&pool_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    if is_new_database {
        tracing::info!(
            "New database created at: {}",
            sqlite_path
        );
    }

    Ok(pool)
}

// Database operations for backends
pub async fn get_backend_by_alias(
    pool: &SqlitePool,
    alias: &str,
) -> anyhow::Result<Option<Backend>> {
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
    tracing::debug!("Getting all backends from database");
    let result = sqlx::query_as::<_, Backend>(
        "SELECT id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                active, created_at, updated_at
         FROM backends WHERE active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    tracing::debug!("Found {} backends", result.len());
    Ok(result)
}

pub async fn get_backend_by_id(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<Backend>> {
    let result = sqlx::query_as::<_, Backend>(
        "SELECT id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                active, created_at, updated_at
         FROM backends WHERE id = ? AND active = TRUE"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn create_backend(pool: &SqlitePool, backend: NewBackend) -> anyhow::Result<Backend> {
    // Start a transaction explicitly
    let mut tx = pool.begin().await?;
    
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
    .fetch_one(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    Ok(result)
}

pub async fn update_backend(
    pool: &SqlitePool,
    id: i64,
    updates: UpdateBackend,
) -> anyhow::Result<Option<Backend>> {
    let backend = sqlx::query_as::<_, Backend>(
        "UPDATE backends SET
            provider_type = COALESCE(?, provider_type),
            display_name = COALESCE(?, display_name),
            model_alias = COALESCE(?, model_alias),
            model_name = COALESCE(?, model_name),
            endpoint_url = COALESCE(?, endpoint_url),
            api_key = COALESCE(?, api_key),
            additional_config = COALESCE(?, additional_config),
            cost_per_input_token = COALESCE(?, cost_per_input_token),
            cost_per_output_token = COALESCE(?, cost_per_output_token),
            max_request_timeout_s = COALESCE(?, max_request_timeout_s),
            active = COALESCE(?, active),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, provider_type, display_name, model_alias, model_name, endpoint_url, api_key,
                   additional_config, cost_per_input_token, cost_per_output_token, max_request_timeout_s,
                   active, created_at, updated_at",
    )
    .bind(updates.provider_type)
    .bind(updates.display_name)
    .bind(updates.model_alias)
    .bind(updates.model_name)
    .bind(updates.endpoint_url)
    .bind(updates.api_key)
    .bind(updates.additional_config)
    .bind(updates.cost_per_input_token)
    .bind(updates.cost_per_output_token)
    .bind(updates.max_request_timeout_s)
    .bind(updates.active)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(backend)
}

pub async fn delete_backend(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM backends WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// User operations
pub async fn get_user_by_proxy_key(
    pool: &SqlitePool,
    proxy_key: &str,
) -> anyhow::Result<Option<User>> {
    let result = sqlx::query_as::<_, User>(
        "SELECT id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute,
                monthly_token_budget, created_at, updated_at
         FROM users WHERE proxy_key = ?",
    )
    .bind(proxy_key)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn create_user(pool: &SqlitePool, user: NewUser) -> anyhow::Result<User> {
    // Start a transaction explicitly
    let mut tx = pool.begin().await?;
    
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
    .fetch_one(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    Ok(result)
}

pub async fn update_user(
    pool: &SqlitePool,
    id: i64,
    updates: UpdateUser,
) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET
            username = COALESCE(?, username),
            enabled = COALESCE(?, enabled),
            allowed_backends = COALESCE(?, allowed_backends),
            rate_limit_requests_per_minute = COALESCE(?, rate_limit_requests_per_minute),
            monthly_token_budget = COALESCE(?, monthly_token_budget),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute, monthly_token_budget, created_at, updated_at",
    )
    .bind(updates.username)
    .bind(updates.enabled)
    .bind(updates.allowed_backends)
    .bind(updates.rate_limit_requests_per_minute)
    .bind(updates.monthly_token_budget)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn regenerate_user_key(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET
            proxy_key = 'llamp-' || LOWER(HEX(RANDOMBLOB(16))),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute, monthly_token_budget, created_at, updated_at",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn get_all_users(pool: &SqlitePool) -> anyhow::Result<Vec<User>> {
    let result = sqlx::query_as::<_, User>(
        "SELECT id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute,
                monthly_token_budget, created_at, updated_at
         FROM users",
    )
    .fetch_all(pool)
    .await?;

    Ok(result)
}

pub async fn get_user_by_id(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<User>> {
    let result = sqlx::query_as::<_, User>(
        "SELECT id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute,
                monthly_token_budget, created_at, updated_at
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
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
