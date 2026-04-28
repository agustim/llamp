#[cfg(test)]
mod tests {
    use llamp::providers::openai::OpenAIProvider;
    use llamp::providers::LLMProvider;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new();
        assert_eq!(provider.content_type(), "application/json");
    }
}