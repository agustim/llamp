#[cfg(test)]
mod tests {
    use llamp::db;
    use llamp::models::{NewBackend, NewUsageLog, NewUser};
    use sqlx::sqlite::SqlitePool;

    async fn setup_test_db() -> anyhow::Result<SqlitePool> {
        // Use an in-memory database for testing
        let pool = db::init("sqlite::memory:").await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_create_and_get_backend() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a new backend
        let new_backend = NewBackend {
            provider_type: "openai".to_string(),
            display_name: "Test OpenAI".to_string(),
            model_alias: "gpt-4-test".to_string(),
            model_name: "gpt-4".to_string(),
            endpoint_url: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: Some("test-key".to_string()),
            additional_config: None,
            cost_per_input_token: Some(0.01),
            cost_per_output_token: Some(0.02),
            max_request_timeout_s: Some(300),
            active: true,
        };

        // Create the backend
        let created_backend = db::create_backend(&pool, new_backend).await?;

        // Verify the backend was created with the correct values
        assert_eq!(created_backend.provider_type, "openai");
        assert_eq!(created_backend.display_name, "Test OpenAI");
        assert_eq!(created_backend.model_alias, "gpt-4-test");

        // Get the backend by alias
        let fetched_backend = db::get_backend_by_alias(&pool, "gpt-4-test").await?;
        assert!(fetched_backend.is_some());
        let fetched_backend = fetched_backend.unwrap();
        assert_eq!(fetched_backend.display_name, "Test OpenAI");
        assert_eq!(fetched_backend.model_alias, "gpt-4-test");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_get_user() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a new user
        let new_user = NewUser {
            username: "testuser".to_string(),
            proxy_key: "llamp-test-key".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        // Create the user
        let created_user = db::create_user(&pool, new_user).await?;

        // Verify the user was created with the correct values
        assert_eq!(created_user.username, "testuser");
        assert_eq!(created_user.proxy_key, "llamp-test-key");

        // Get the user by proxy key
        let fetched_user = db::get_user_by_proxy_key(&pool, "llamp-test-key").await?;
        assert!(fetched_user.is_some());
        let fetched_user = fetched_user.unwrap();
        assert_eq!(fetched_user.username, "testuser");
        assert_eq!(fetched_user.proxy_key, "llamp-test-key");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_usage_log() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // First create a user for the foreign key reference
        let new_user = NewUser {
            username: "testuser".to_string(),
            proxy_key: "llamp-test-key".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        let created_user = db::create_user(&pool, new_user).await?;

        // Create a usage log
        let new_log = NewUsageLog {
            user_id: Some(created_user.id),
            model_alias: Some("gpt-4-test".to_string()),
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
            latency_ms: Some(500),
            cost: Some(0.005),
            status: "success".to_string(),
            error_message: None,
        };

        // Create the usage log
        let created_log = db::create_usage_log(&pool, new_log).await?;

        // Verify the usage log was created with the correct values
        assert_eq!(created_log.user_id, Some(created_user.id));
        assert_eq!(created_log.model_alias, Some("gpt-4-test".to_string()));
        assert_eq!(created_log.prompt_tokens, 100);
        assert_eq!(created_log.completion_tokens, 200);
        assert_eq!(created_log.total_tokens, 300);

        Ok(())
    }
}
