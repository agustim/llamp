#[cfg(test)]
mod tests {
    use llamp::config::{Cli, Config};
    use clap::Parser;

    #[test]
    fn test_config_from_args() {
        let cli = Cli::parse_from([
            "llamp",
            "--port", "3000",
            "--host", "127.0.0.1",
        ]);

        let config = Config::from_args(&cli).unwrap();

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.database_url, "sqlite://./llamp.db"); // default
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_config_custom_database() {
        let cli = Cli::parse_from([
            "llamp",
            "--database", "sqlite://./custom.db",
        ]);

        let config = Config::from_args(&cli).unwrap();

        assert_eq!(config.database_url, "sqlite://./custom.db");
    }

    #[test]
    fn test_config_admin_key() {
        let cli = Cli::parse_from([
            "llamp",
            "--admin-key", "my-secret-key",
        ]);

        let config = Config::from_args(&cli).unwrap();

        assert_eq!(config.get_admin_key(), Some(&"my-secret-key".to_string()));
    }

    #[test]
    fn test_config_get_address() {
        let cli = Cli::parse_from([
            "llamp",
            "--port", "8080",
            "--host", "0.0.0.0",
        ]);

        let config = Config::from_args(&cli).unwrap();
        assert_eq!(config.get_address(), "0.0.0.0:8080");
    }

    #[test]
    fn test_config_default_values() {
        let cli = Cli::parse_from(["llamp"]);

        let config = Config::from_args(&cli).unwrap();

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
    }
}
