CREATE TABLE IF NOT EXISTS backends (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    model_alias TEXT UNIQUE NOT NULL,
    model_name TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    api_key TEXT,
    additional_config TEXT,
    cost_per_input_token REAL DEFAULT 0,
    cost_per_output_token REAL DEFAULT 0,
    max_request_timeout_s INTEGER DEFAULT 300,
    active BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    proxy_key TEXT UNIQUE NOT NULL,
    enabled BOOLEAN DEFAULT TRUE,
    allowed_backends TEXT,
    rate_limit_requests_per_minute INTEGER DEFAULT 60,
    monthly_token_budget INTEGER DEFAULT -1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS usage_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES users(id),
    model_alias TEXT,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    latency_ms INTEGER,
    cost REAL DEFAULT 0,
    status TEXT DEFAULT 'success',
    error_message TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS system_config (
    key TEXT PRIMARY KEY,
    value TEXT
);