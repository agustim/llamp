#[cfg(test)]
mod tests {
    use llamp::models::{NewBackend, NewUser, NewUsageLog, ChatCompletionRequest, Message, ChatCompletionResponse, Choice, Usage};

    #[test]
    fn test_new_backend_creation() {
        let new_backend = NewBackend {
            provider_type: "openai".to_string(),
            display_name: "Test Backend".to_string(),
            model_alias: "gpt-4".to_string(),
            model_name: "gpt-4".to_string(),
            endpoint_url: "https://api.openai.com/v1".to_string(),
            api_key: Some("test-key".to_string()),
            additional_config: None,
            cost_per_input_token: Some(0.01),
            cost_per_output_token: Some(0.02),
            max_request_timeout_s: Some(300),
            active: true,
        };

        assert_eq!(new_backend.provider_type, "openai");
        assert_eq!(new_backend.display_name, "Test Backend");
        assert_eq!(new_backend.model_alias, "gpt-4");
        assert!(new_backend.api_key.is_some());
    }

    #[test]
    fn test_backend_with_optional_fields() {
        let new_backend = NewBackend {
            provider_type: "custom".to_string(),
            display_name: "Custom Backend".to_string(),
            model_alias: "custom-model".to_string(),
            model_name: "custom-model".to_string(),
            endpoint_url: "https://custom.api.com".to_string(),
            api_key: None,
            additional_config: Some(r#"{"custom_field": "value"}"#.to_string()),
            cost_per_input_token: None,
            cost_per_output_token: None,
            max_request_timeout_s: None,
            active: false,
        };

        assert!(new_backend.api_key.is_none());
        assert!(new_backend.additional_config.is_some());
        assert!(!new_backend.active);
    }

    #[test]
    fn test_new_user_creation() {
        let new_user = NewUser {
            username: "testuser".to_string(),
            proxy_key: "llamp-key-123".to_string(),
            enabled: true,
            allowed_backends: None,
            rate_limit_requests_per_minute: Some(60),
            monthly_token_budget: Some(1000000),
        };

        assert_eq!(new_user.username, "testuser");
        assert!(new_user.enabled);
        assert_eq!(new_user.rate_limit_requests_per_minute, Some(60));
    }

    #[test]
    fn test_user_with_allowed_backends() {
        let new_user = NewUser {
            username: "restricted_user".to_string(),
            proxy_key: "llamp-key-456".to_string(),
            enabled: true,
            allowed_backends: Some("gpt-4,claude-3".to_string()),
            rate_limit_requests_per_minute: Some(30),
            monthly_token_budget: Some(500000),
        };

        assert!(new_user.allowed_backends.is_some());
        let backends = new_user.allowed_backends.unwrap();
        assert!(backends.contains("gpt-4"));
        assert!(backends.contains("claude-3"));
    }

    #[test]
    fn test_new_usage_log() {
        let new_log = NewUsageLog {
            user_id: Some(1),
            model_alias: Some("gpt-4".to_string()),
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
            latency_ms: Some(500),
            cost: Some(0.005),
            status: "success".to_string(),
            error_message: None,
        };

        assert_eq!(new_log.prompt_tokens, 100);
        assert_eq!(new_log.completion_tokens, 200);
        assert_eq!(new_log.total_tokens, 300);
        assert!(new_log.cost.is_some());
    }

    #[test]
    fn test_chat_completion_request() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                },
            ],
            stream: Some(true),
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert!(request.stream.is_some());
    }

    #[test]
    fn test_chat_completion_response() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: "Hello there!".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
        };

        assert_eq!(response.choices.len(), 1);
        assert!(response.usage.is_some());
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 150);
    }

    #[test]
    fn test_usage_calculation() {
        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };

        assert_eq!(usage.prompt_tokens + usage.completion_tokens, usage.total_tokens);
    }
}
