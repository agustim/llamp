use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Admin key for administrative endpoints
    #[arg(long)]
    pub admin_key: Option<String>,

    /// Port to listen on
    #[arg(long, default_value = "8080")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Path to config file
    #[arg(long)]
    pub config: Option<String>,

    /// Database URL
    #[arg(long)]
    pub database: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub admin_key: Option<String>,
    pub database_url: String,
    pub log_level: String,
}

impl Config {
    pub fn from_args(cli: &Cli) -> anyhow::Result<Self> {
        let host = cli.host.clone();
        let port = cli.port;
        let database_url = cli
            .database
            .clone()
            .unwrap_or_else(|| "sqlite://./llamp.db".to_string());
        let log_level = "info".to_string();
        let admin_key = cli.admin_key.clone();

        Ok(Config {
            host,
            port,
            admin_key,
            database_url,
            log_level,
        })
    }

    // Add methods to actually use the config values
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn get_log_level(&self) -> &str {
        &self.log_level
    }

    pub fn get_admin_key(&self) -> Option<&String> {
        self.admin_key.as_ref()
    }
}
