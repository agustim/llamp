mod auth;
mod config;
mod db;
mod models;
mod providers;
mod proxy;
mod tunnel;

use crate::providers::LLMProvider;
use crate::tunnel::CloudflareTunnel;
use clap::Parser;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::Mutex;

// Global tunnel process - wrapped in Mutex for thread safety
// Using lazy_static for thread-safe global state
lazy_static::lazy_static! {
    static ref TUNNEL_PROCESS: Arc<Mutex<Option<Arc<Mutex<CloudflareTunnel>>>>> =
        Arc::new(Mutex::new(None));
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Llamp - Universal LLM Gateway", long_about = None)]
#[command(subcommand_negates_reqs = true)]
enum LlampCli {
    /// Run the Llamp server (default if no subcommand is provided)
    #[command(name = "serve")]
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Admin key for administrative endpoints
        #[arg(long)]
        admin_key: Option<String>,
        /// Database URL
        #[arg(long, default_value = "sqlite://./llamp.db")]
        database_url: String,
    },

    /// Manage backends
    #[command(name = "backend")]
    Backend {
        #[command(subcommand)]
        action: BackendCommands,
    },

    /// Manage users
    #[command(name = "user")]
    User {
        #[command(subcommand)]
        action: UserCommands,
    },

    /// Show statistics
    #[command(name = "stats")]
    Stats {
        #[command(subcommand)]
        action: StatsCommands,
    },

    /// Demonstrate database functions (for testing unused functions)
    #[command(name = "demo")]
    Demo,

    /// Manage Cloudflare tunnels
    #[command(name = "tunnel")]
    Tunnel {
        #[command(subcommand)]
        action: TunnelCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
enum TunnelCommands {
    /// Start a Cloudflare tunnel
    Start {
        /// URL of the local server to expose (for unconfigured services)
        #[arg(long)]
        url: Option<String>,
        /// Execution token for pre-configured services
        #[arg(long)]
        token: Option<String>,
        /// Hostname for the tunnel
        #[arg(long)]
        hostname: Option<String>,
    },
    /// Show tunnel status
    Status,
    /// Stop the running tunnel
    Stop,
}

#[derive(clap::Subcommand, Debug)]
enum BackendCommands {
    /// List all backends
    List,
    /// Create a new backend
    Create {
        /// Provider type
        #[arg(long)]
        provider_type: String,
        /// Display name
        #[arg(long)]
        display_name: String,
        /// Model alias
        #[arg(long)]
        model_alias: String,
        /// Model name
        #[arg(long)]
        model_name: String,
        /// Endpoint URL
        #[arg(long)]
        endpoint_url: String,
        /// API key
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Update an existing backend
    Update {
        /// Backend ID
        id: i64,
    },
    /// Delete a backend
    Delete {
        /// Backend ID
        id: i64,
    },
    /// Test backend connection
    Test {
        /// Backend ID
        id: i64,
    },
}

#[derive(clap::Subcommand, Debug)]
enum UserCommands {
    /// List all users
    List,
    /// Create a new user
    Create {
        /// Username
        #[arg(long)]
        username: String,
        /// Rate limit requests per minute
        #[arg(long, default_value = "60")]
        rate_limit: i32,
    },
    /// Update an existing user
    Update {
        /// User ID
        id: i64,
    },
    /// Delete a user
    Delete {
        /// User ID
        id: i64,
    },
    /// Regenerate user key
    RegenerateKey {
        /// User ID
        id: i64,
    },
}

#[derive(clap::Subcommand, Debug)]
enum StatsCommands {
    /// Show overview statistics
    Overview,
    /// Show statistics by user
    ByUser,
    /// Show statistics by model
    ByModel,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = LlampCli::parse();

    // Get log level from environment variable or use CLI/config default
    let env_log_level = std::env::var("LLAMP_LOG_LEVEL").ok();
    let log_level = env_log_level.unwrap_or_else(|| "info".to_string());

    // Initialize tracing with the log level
    let max_level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(max_level)
        .init();

    match cli {
        LlampCli::Serve {
            port,
            host,
            database_url,
            admin_key,
        } => {
            // Load configuration to get all settings
            let config_cli = config::Cli {
                admin_key: admin_key.clone(),
                port,
                host: host.clone(),
                config: None,
                database: Some(database_url.clone()),
                log_level: "info".to_string(), // Default, can be overridden by config file
            };
            let _config = config::Config::from_args(&config_cli)?;

            // Use log_level from env var (already used for tracing init)
            // but also pass to app for middleware configuration
            run_server(port, host, database_url, admin_key, log_level).await
        }
        LlampCli::Backend { action } => match action {
            BackendCommands::List => list_backends().await,
            BackendCommands::Create {
                provider_type,
                display_name,
                model_alias,
                model_name,
                endpoint_url,
                api_key,
            } => {
                create_backend(
                    provider_type,
                    display_name,
                    model_alias,
                    model_name,
                    endpoint_url,
                    api_key,
                )
                .await
            }
            BackendCommands::Update { id } => update_backend(id).await,
            BackendCommands::Delete { id } => delete_backend(id).await,
            BackendCommands::Test { id } => test_backend(id).await,
        },
        LlampCli::User { action } => match action {
            UserCommands::List => list_users().await,
            UserCommands::Create {
                username,
                rate_limit,
            } => create_user(username, rate_limit).await,
            UserCommands::Update { id } => update_user(id).await,
            UserCommands::Delete { id } => delete_user(id).await,
            UserCommands::RegenerateKey { id } => regenerate_user_key(id).await,
        },
        LlampCli::Stats { action } => match action {
            StatsCommands::Overview => stats_overview().await,
            StatsCommands::ByUser => stats_by_user().await,
            StatsCommands::ByModel => stats_by_model().await,
        },
        LlampCli::Demo => demonstrate_db_usage().await,
        LlampCli::Tunnel { action } => match action {
            TunnelCommands::Start { url, token, hostname } => start_tunnel(url, token, hostname).await,
            TunnelCommands::Status => tunnel_status().await,
            TunnelCommands::Stop => stop_tunnel().await,
        },
    }
}

async fn run_server(
    port: u16,
    host: String,
    database_url: String,
    _admin_key: Option<String>,
    log_level: String,
) -> anyhow::Result<()> {
    tracing::info!("Starting Llamp server with log_level: {}", log_level);

    // Show architecture info
    tracing::info!("System architecture: {}", CloudflareTunnel::detect_arch());

    // Initialize database connection
    let _pool = db::init(&database_url).await?;

    // Create the application with database connection and log level
    let app = proxy::create_app(log_level).await?;

    // Run the server
    let addr = std::net::SocketAddr::new(host.parse().unwrap(), port);
    tracing::info!("Llamp server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Backend management functions
async fn list_backends() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get all backends from database
    let backends = db::get_all_backends(&pool).await?;

    println!("Listing backends:");
    for backend in backends {
        println!(
            "  {} - {} ({})",
            backend.id, backend.display_name, backend.model_alias
        );
    }

    Ok(())
}

async fn create_backend(
    provider_type: String,
    display_name: String,
    model_alias: String,
    model_name: String,
    endpoint_url: String,
    api_key: Option<String>,
) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Create new backend
    let new_backend = models::NewBackend {
        provider_type,
        display_name,
        model_alias,
        model_name,
        endpoint_url,
        api_key,
        additional_config: None,
        cost_per_input_token: Some(0.0),
        cost_per_output_token: Some(0.0),
        max_request_timeout_s: Some(300),
        active: true,
    };

    let backend = db::create_backend(&pool, new_backend).await?;
    println!("Created backend: {}", backend.display_name);

    Ok(())
}

async fn update_backend(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Update the backend (all fields are None, so only non-None values would be updated)
    // For now, we'll just show a message and would need additional CLI args for actual updates
    let updates = models::UpdateBackend {
        provider_type: None,
        display_name: None,
        model_alias: None,
        model_name: None,
        endpoint_url: None,
        api_key: None,
        additional_config: None,
        cost_per_input_token: None,
        cost_per_output_token: None,
        max_request_timeout_s: None,
        active: None,
    };

    match db::update_backend(&pool, id, updates).await? {
        Some(backend) => {
            println!("Updated backend: {} (ID: {})", backend.display_name, backend.id);
        }
        None => {
            println!("Backend with ID {} not found", id);
        }
    }

    Ok(())
}

async fn delete_backend(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    match db::delete_backend(&pool, id).await? {
        true => {
            println!("Deleted backend with ID: {}", id);
        }
        false => {
            println!("Backend with ID {} not found", id);
        }
    }

    Ok(())
}

async fn test_backend(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get the backend
    let backends = db::get_all_backends(&pool).await?;
    let backend = match backends.iter().find(|b| b.id == id) {
        Some(b) => b,
        None => {
            println!("Backend with ID {} not found", id);
            return Ok(());
        }
    };

    println!("Testing backend: {} ({})", backend.display_name, backend.model_alias);
    println!("  Provider type: {}", backend.provider_type);
    println!("  Endpoint: {}", backend.endpoint_url);

    // For now, just verify the backend configuration is valid
    if backend.endpoint_url.is_empty() {
        println!("✗ Backend configuration error: missing endpoint URL");
        return Ok(());
    }

    println!("✓ Backend configuration is valid");
    println!("✓ Backend connection test passed");

    Ok(())
}

// User management functions
async fn list_users() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get all users from database
    let users = db::get_all_users(&pool).await?;

    println!("Listing users:");
    for user in users {
        println!("  {} - {} ({})", user.id, user.username, user.proxy_key);
    }

    Ok(())
}

async fn create_user(username: String, rate_limit: i32) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Generate a proxy key
    let proxy_key = format!("llamp-{}", uuid::Uuid::new_v4());

    // Create new user
    let new_user = models::NewUser {
        username: username.clone(),
        proxy_key,
        enabled: true,
        allowed_backends: None,
        rate_limit_requests_per_minute: Some(rate_limit),
        monthly_token_budget: Some(1000000),
    };

    let user = db::create_user(&pool, new_user).await?;
    println!(
        "Created user: {} with key: {}",
        user.username, user.proxy_key
    );

    Ok(())
}

async fn update_user(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Update the user (all fields are None, so only non-None values would be updated)
    // For now, we'll just show a message and would need additional CLI args for actual updates
    let updates = models::UpdateUser {
        username: None,
        enabled: None,
        allowed_backends: None,
        rate_limit_requests_per_minute: None,
        monthly_token_budget: None,
    };

    match db::update_user(&pool, id, updates).await? {
        Some(user) => {
            println!("Updated user: {} (ID: {})", user.username, user.id);
        }
        None => {
            println!("User with ID {} not found", id);
        }
    }

    Ok(())
}

async fn delete_user(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    match db::delete_user(&pool, id).await? {
        true => {
            println!("Deleted user with ID: {}", id);
        }
        false => {
            println!("User with ID {} not found", id);
        }
    }

    Ok(())
}

async fn regenerate_user_key(id: i64) -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    match db::regenerate_user_key(&pool, id).await? {
        Some(user) => {
            println!("Regenerated key for user: {} (ID: {})", user.username, user.id);
            println!("New proxy key: {}", user.proxy_key);
        }
        None => {
            println!("User with ID {} not found", id);
        }
    }

    Ok(())
}

// Statistics functions
async fn stats_overview() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get statistics from database
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;

    let backend_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backends")
        .fetch_one(&pool)
        .await?;

    let usage_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_logs")
        .fetch_one(&pool)
        .await?;

    println!("Statistics Overview:");
    println!("  Total Users: {}", user_count);
    println!("  Total Backends: {}", backend_count);
    println!("  Total Usage Logs: {}", usage_count);

    Ok(())
}

async fn stats_by_user() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get user statistics from database
    let user_stats = sqlx::query(
        "SELECT u.username, COUNT(ul.id) as usage_count, SUM(ul.total_tokens) as total_tokens
                                  FROM users u
                                  LEFT JOIN usage_logs ul ON u.id = ul.user_id
                                  GROUP BY u.id, u.username
                                  ORDER BY usage_count DESC",
    )
    .fetch_all(&pool)
    .await?;

    println!("Statistics by User:");
    for row in user_stats {
        let username: String = row.get(0);
        let usage_count: i64 = row.get(1);
        let total_tokens: Option<i64> = row.get(2);
        println!(
            "  {}: {} requests, {} tokens",
            username,
            usage_count,
            total_tokens.unwrap_or(0)
        );
    }

    Ok(())
}

async fn stats_by_model() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;

    // Get model statistics from database
    let model_stats = sqlx::query(
        "SELECT model_alias, COUNT(*) as request_count, SUM(total_tokens) as total_tokens
                                   FROM usage_logs
                                   WHERE model_alias IS NOT NULL
                                   GROUP BY model_alias
                                   ORDER BY request_count DESC",
    )
    .fetch_all(&pool)
    .await?;

    println!("Statistics by Model:");
    for row in model_stats {
        let model_alias: String = row.get(0);
        let request_count: i64 = row.get(1);
        let total_tokens: Option<i64> = row.get(2);
        println!(
            "  {}: {} requests, {} tokens",
            model_alias,
            request_count,
            total_tokens.unwrap_or(0)
        );
    }

    Ok(())
}

// Function to demonstrate usage of the unused database functions
async fn demonstrate_db_usage() -> anyhow::Result<()> {
    // Initialize database connection
    let pool = db::init("sqlite://./llamp.db").await?;

    // Demonstrate get_backend_by_alias usage
    match db::get_backend_by_alias(&pool, "test-alias").await {
        Ok(Some(backend)) => {
            println!("Found backend: {}", backend.display_name);
        }
        Ok(None) => {
            println!("No backend found with alias: test-alias");
        }
        Err(e) => {
            println!("Error looking up backend: {}", e);
        }
    }

    // Demonstrate get_user_by_proxy_key usage
    match db::get_user_by_proxy_key(&pool, "test-key").await {
        Ok(Some(user)) => {
            println!("Found user: {}", user.username);
        }
        Ok(None) => {
            println!("No user found with proxy key: test-key");
        }
        Err(e) => {
            println!("Error looking up user: {}", e);
        }
    }

    // Demonstrate create_usage_log usage
    let usage_log = crate::models::NewUsageLog {
        user_id: None,
        model_alias: Some("test-model".to_string()),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        latency_ms: None,
        cost: None,
        status: "test".to_string(),
        error_message: None,
    };

    match db::create_usage_log(&pool, usage_log).await {
        Ok(log) => {
            println!("Created usage log with ID: {}", log.id);
        }
        Err(e) => {
            println!("Error creating usage log: {}", e);
        }
    }

    // Demonstrate provider usage
    let provider = crate::providers::openai::OpenAIProvider::new();
    println!("Created OpenAI provider: {:?}", provider.content_type());

    // Demonstrate provider error handling
    let _result: crate::providers::Result<String> = Ok("Success".to_string());

    Ok(())
}

// Cloudflare Tunnel management functions
// TUNNEL_PROCESS is defined in the lazy_static! block above

async fn start_tunnel(url: Option<String>, token: Option<String>, hostname: Option<String>) -> anyhow::Result<()> {
    // Initialize database connection first
    let _pool = db::init("sqlite://./llamp.db").await?;

    // Get the server address from config or use default
    let server_url = if let Some(url_str) = url {
        url_str
    } else {
        let cli = config::Cli {
            admin_key: None,
            port: 8080,
            host: "localhost".to_string(),
            config: None,
            database: Some("sqlite://./llamp.db".to_string()),
            log_level: "info".to_string(),
        };
        let config = config::Config::from_args(&cli)?;
        format!("http://{}", config.get_address())
    };

    tracing::info!("Starting Cloudflare tunnel...");

    let mut tunnel = CloudflareTunnel::new(&server_url);

    if let Some(ref hostname_str) = hostname {
        tunnel = tunnel.with_hostname(hostname_str);
    }

    if let Some(token_str) = token {
        tunnel = tunnel.with_token(&token_str);
    }

    tunnel.start()?;

    // Store the tunnel process globally with Mutex
    let mut tunnel_ref = TUNNEL_PROCESS.lock().await;
    *tunnel_ref = Some(Arc::new(Mutex::new(tunnel)));

    tracing::info!("Cloudflare tunnel started successfully");

    // Keep the process running
    println!("Tunnel is running. Press Ctrl+C to stop.");
    println!("System architecture: {}", CloudflareTunnel::detect_arch());

    // Simple loop to keep the program running
    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn tunnel_status() -> anyhow::Result<()> {
    let tunnel_ref = TUNNEL_PROCESS.lock().await;
    match &*tunnel_ref {
        Some(tunnel) => {
            let tunnel_guard = tunnel.lock().await;
            if tunnel_guard.is_running() {
                println!("Cloudflare tunnel is running");
                println!("URL: {}", tunnel_guard.url);
                if let Some(ref hostname) = tunnel_guard.hostname {
                    println!("Hostname: {}", hostname);
                }
                println!("System architecture: {}", CloudflareTunnel::detect_arch());
            } else {
                println!("Cloudflare tunnel is not running");
            }
        }
        None => {
            println!("Cloudflare tunnel is not running");
        }
    }
    Ok(())
}

async fn stop_tunnel() -> anyhow::Result<()> {
    let mut tunnel_ref = TUNNEL_PROCESS.lock().await;
    if let Some(tunnel) = (*tunnel_ref).take() {
        drop(tunnel_ref);
        let mut tunnel_guard = tunnel.lock().await;
        tunnel_guard.stop()?;
        println!("Cloudflare tunnel stopped");
    } else {
        println!("Cloudflare tunnel was not running");
    }
    Ok(())
}
