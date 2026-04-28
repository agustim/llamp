mod config;
mod db;
mod models;
mod providers;
mod auth;
mod proxy;

use clap::Parser;
use uuid;
use sqlx::Row;

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

    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    match cli {
        LlampCli::Serve { port, host, database_url, admin_key } => {
            // Run the server if no subcommand or run as server
            run_server(port, host, database_url, admin_key).await
        }
        LlampCli::Backend { action } => {
            match action {
                BackendCommands::List => list_backends().await,
                BackendCommands::Create { provider_type, display_name, model_alias, model_name, endpoint_url, api_key } => {
                    create_backend(provider_type, display_name, model_alias, model_name, endpoint_url, api_key).await
                },
                BackendCommands::Update { id } => update_backend(id).await,
                BackendCommands::Delete { id } => delete_backend(id).await,
                BackendCommands::Test { id } => test_backend(id).await,
            }
        }
        LlampCli::User { action } => {
            match action {
                UserCommands::List => list_users().await,
                UserCommands::Create { username, rate_limit } => create_user(username, rate_limit).await,
                UserCommands::Update { id } => update_user(id).await,
                UserCommands::Delete { id } => delete_user(id).await,
                UserCommands::RegenerateKey { id } => regenerate_user_key(id).await,
            }
        }
        LlampCli::Stats { action } => {
            match action {
                StatsCommands::Overview => stats_overview().await,
                StatsCommands::ByUser => stats_by_user().await,
                StatsCommands::ByModel => stats_by_model().await,
            }
        }
    }
}

async fn run_server(port: u16, host: String, database_url: String, admin_key: Option<String>) -> anyhow::Result<()> {
    // Create CLI struct with provided values
    let cli = config::Cli {
        admin_key,
        port,
        host: host.clone(),
        config: None,
        database: Some(database_url),
    };

    // Load configuration
    let config = config::Config::from_args(&cli)?;

    tracing::info!("Starting Llamp server with config: {:?}", config);

    // Initialize database connection using the config
    let _pool = db::init(&config.database_url).await?;

    // Create the application with database connection
    let app = proxy::create_app().await?;

    // Run the server
    let addr = std::net::SocketAddr::new(
        host.parse().unwrap(),
        config.port,
    );
    tracing::info!("Llamp server listening on {}", config.get_address());
    
    // Use the admin key if provided
    if let Some(_admin_key) = config.get_admin_key() {
        tracing::info!("Admin key is set for this server");
    }
    
    // Use the log level
    tracing::info!("Log level is set to: {}", config.get_log_level());

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
        println!("  {} - {} ({})", backend.id, backend.display_name, backend.model_alias);
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
    println!("Updating backend with ID: {}", id);
    // TODO: Implement real backend update
    Ok(())
}

async fn delete_backend(id: i64) -> anyhow::Result<()> {
    println!("Deleting backend with ID: {}", id);
    // TODO: Implement real backend deletion
    Ok(())
}

async fn test_backend(id: i64) -> anyhow::Result<()> {
    println!("Testing backend with ID: {}", id);
    // TODO: Implement real backend testing
    Ok(())
}

// User management functions
async fn list_users() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;
    
    // Get all users from database
    let users = sqlx::query_as::<_, models::User>(
        "SELECT id, username, proxy_key, enabled, allowed_backends, rate_limit_requests_per_minute,
                monthly_token_budget, created_at, updated_at
         FROM users"
    )
    .fetch_all(&pool)
    .await?;

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
    println!("Created user: {} with key: {}", user.username, user.proxy_key);

    Ok(())
}

async fn update_user(id: i64) -> anyhow::Result<()> {
    println!("Updating user with ID: {}", id);
    // TODO: Implement real user update
    Ok(())
}

async fn delete_user(id: i64) -> anyhow::Result<()> {
    println!("Deleting user with ID: {}", id);
    // TODO: Implement real user deletion
    Ok(())
}

async fn regenerate_user_key(id: i64) -> anyhow::Result<()> {
    println!("Regenerating user key for user ID: {}", id);
    // TODO: Implement real user key regeneration
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
    let user_stats = sqlx::query("SELECT u.username, COUNT(ul.id) as usage_count, SUM(ul.total_tokens) as total_tokens
                                  FROM users u
                                  LEFT JOIN usage_logs ul ON u.id = ul.user_id
                                  GROUP BY u.id, u.username
                                  ORDER BY usage_count DESC")
        .fetch_all(&pool)
        .await?;

    println!("Statistics by User:");
    for row in user_stats {
        let username: String = row.get(0);
        let usage_count: i64 = row.get(1);
        let total_tokens: Option<i64> = row.get(2);
        println!("  {}: {} requests, {} tokens", username, usage_count, total_tokens.unwrap_or(0));
    }

    Ok(())
}

async fn stats_by_model() -> anyhow::Result<()> {
    // Initialize database connection with default URL
    let pool = db::init("sqlite://./llamp.db").await?;
    
    // Get model statistics from database
    let model_stats = sqlx::query("SELECT model_alias, COUNT(*) as request_count, SUM(total_tokens) as total_tokens
                                   FROM usage_logs
                                   WHERE model_alias IS NOT NULL
                                   GROUP BY model_alias
                                   ORDER BY request_count DESC")
        .fetch_all(&pool)
        .await?;

    println!("Statistics by Model:");
    for row in model_stats {
        let model_alias: String = row.get(0);
        let request_count: i64 = row.get(1);
        let total_tokens: Option<i64> = row.get(2);
        println!("  {}: {} requests, {} tokens", model_alias, request_count, total_tokens.unwrap_or(0));
    }

    Ok(())
}