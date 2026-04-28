#[cfg(test)]
mod tests {
    use llamp::db;
    use llamp::models::{NewBackend, NewUsageLog, NewUser, UpdateBackend, UpdateUser};
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

    #[tokio::test]
    async fn test_update_backend() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a backend first
        let new_backend = NewBackend {
            provider_type: "openai".to_string(),
            display_name: "Original Name".to_string(),
            model_alias: "test-alias".to_string(),
            model_name: "test-model".to_string(),
            endpoint_url: "https://original.com".to_string(),
            api_key: Some("original-key".to_string()),
            additional_config: None,
            cost_per_input_token: Some(0.01),
            cost_per_output_token: Some(0.02),
            max_request_timeout_s: Some(300),
            active: true,
        };

        let created = db::create_backend(&pool, new_backend).await?;

        // Update the backend
        let updates = UpdateBackend {
            provider_type: Some("openai".to_string()),
            display_name: Some("Updated Name".to_string()),
            model_alias: None,  // model_alias unchanged
            model_name: None,  // model_name unchanged
            endpoint_url: Some("https://updated.com".to_string()),
            api_key: None,  // api_key unchanged
            additional_config: None,  // additional_config unchanged
            cost_per_input_token: None,  // cost_per_input_token unchanged
            cost_per_output_token: None,  // cost_per_output_token unchanged
            max_request_timeout_s: None,  // max_request_timeout_s unchanged
            active: Some(false),  // active changed to false
        };

        let updated = db::update_backend(&pool, created.id, updates).await?;

        assert!(updated.is_some());
        let backend = updated.unwrap();
        assert_eq!(backend.display_name, "Updated Name");
        assert_eq!(backend.endpoint_url, "https://updated.com");
        assert_eq!(backend.provider_type, "openai");
        assert!(!backend.active);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_backend() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a backend first
        let new_backend = NewBackend {
            provider_type: "openai".to_string(),
            display_name: "To Delete".to_string(),
            model_alias: "delete-alias".to_string(),
            model_name: "delete-model".to_string(),
            endpoint_url: "https://delete.com".to_string(),
            api_key: Some("key".to_string()),
            additional_config: None,
            cost_per_input_token: Some(0.01),
            cost_per_output_token: Some(0.02),
            max_request_timeout_s: Some(300),
            active: true,
        };

        let created = db::create_backend(&pool, new_backend).await?;

        // Delete the backend
        let result = db::delete_backend(&pool, created.id).await?;
        assert!(result);

        // Verify it's gone
        let backends = db::get_all_backends(&pool).await?;
        assert!(!backends.iter().any(|b| b.id == created.id));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_user() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a user first
        let new_user = NewUser {
            username: "original".to_string(),
            proxy_key: "llamp-original".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        let created = db::create_user(&pool, new_user).await?;

        // Update the user
        let updates = UpdateUser {
            username: Some("updated".to_string()),
            enabled: Some(false),  // disabled
            allowed_backends: None,  // allowed_backends unchanged
            rate_limit_requests_per_minute: Some(Some(120)),  // rate limit changed
            monthly_token_budget: None,  // monthly_token_budget unchanged
        };

        let updated = db::update_user(&pool, created.id, updates).await?;

        assert!(updated.is_some());
        let user = updated.unwrap();
        assert_eq!(user.username, "updated");
        assert!(!user.enabled);
        assert_eq!(user.rate_limit_requests_per_minute, Some(120));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_user() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a user first
        let new_user = NewUser {
            username: "to-delete".to_string(),
            proxy_key: "llamp-to-delete".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        let created = db::create_user(&pool, new_user).await?;

        // Delete the user
        let result = db::delete_user(&pool, created.id).await?;
        assert!(result);

        // Verify it's gone
        let users = db::get_all_users(&pool).await?;
        assert!(!users.iter().any(|u| u.id == created.id));

        Ok(())
    }

    #[tokio::test]
    async fn test_regenerate_user_key() -> anyhow::Result<()> {
        let pool = setup_test_db().await?;

        // Create a user first
        let new_user = NewUser {
            username: "key-test".to_string(),
            proxy_key: "llamp-original-key".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        let created = db::create_user(&pool, new_user).await?;
        let original_key = created.proxy_key.clone();

        // Regenerate the key
        let updated = db::regenerate_user_key(&pool, created.id).await?;
        assert!(updated.is_some());
        let user = updated.unwrap();

        // Verify the key changed
        assert_ne!(user.proxy_key, original_key);
        assert!(user.proxy_key.starts_with("llamp-"));

        Ok(())
    }
}
