# Llamp - Rust Universal LLM Gateway

Llamp is a high-performance proxy written in Rust that centralizes calls to language models from external services (internet). The system redirects requests to multiple providers (OpenAI, Anthropic, Google Gemini, Groq, Azure OpenA
I, etc.), handling protocol translation, user authentication, consumption limits, and detailed logging.

## Features

- **Universal Compatibility**: Accepts OpenAI-compatible input format and normalizes all outputs to the same format
- **Multi-Provider Support**: Works with OpenAI, Anthropic, Google Gemini, Groq, Azure OpenAI, and any OpenAI-compatible API
- **High Performance**: Built with Rust and Axum for maximum efficiency
- **Rate Limiting**: Configurable rate limits per user
- **Usage Tracking**: Detailed logging of token consumption and costs
- **Resilience**: Automatic retries, circuit breaker, and health checks
- **Security**: Secure authentication with proxy keys

## Architecture

```
llamp/
├── src/
│   ├── main.rs                 # Entry point, CLI parsing
│   ├── config.rs               # Configuration management
│   ├── db.rs                   # Database connection and operations
│   ├── models.rs              # Data structures
│   ├── providers/               # LLM provider implementations
│   ├── proxy/                  # Core proxy functionality
│   ├── auth.rs                # Authentication middleware
│   └── rate_limit.rs         # Rate limiting implementation
├── migrations/                 # Database migrations
├── Cargo.toml               # Dependencies
└── llamp.toml              # Default configuration
```

## Quick Start

### Running the Server

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Set up environment variables:**
   ```bash
   export ADMIN_KEY="your-admin-key-here"
   ```

3. **Run the server:**
   ```bash
   cargo run -- serve
   ```

### Using the CLI

Llamp includes a powerful command-line interface for administration tasks:

```bash
# Start the server
./llamp serve

# Or use the CLI for administrative tasks
./llamp backend list
./llamp backend create --name "openai-gpt4" --provider "openai" ...
./llamp user create --username "john" --rate-limit 100
./llamp stats overview
```

## Configuration

Llamp can be configured through:
1. Command line arguments
2. Environment variables
3. Configuration file (`llamp.toml`)

### Environment Variables

| Variable | Description |
|---------|-------------|
| `ADMIN_KEY` | Key for accessing administrative endpoints |
| `LLAMP_PORT` | Server port (default: 8080) |
| `LLAMP_HOST` | Server host (default: 0.0.0.0) |
| `LLAMP_DATABASE_URL` | Database connection URL |

## API Endpoints

### Public API (requires `Authorization: Bearer llamp-xxxxx`)

- `POST /v1/chat/completions` - Chat completions (streaming + non-streaming)
- `GET /v1/models` - List available models for the user

### Administration API (requires admin key)

- `GET /admin/backends` - List backends
- `POST /admin/backends` - Create backend
- `PUT /admin/backends/:id` - Update backend
- `DELETE /admin/backends/:id` - Delete backend
- `POST /admin/backends/:id/test` - Test backend connection
- `GET /admin/users` - List users
- `POST /admin/users` - Create user
- `PUT /admin/users/:id` - Update user
- `DELETE /admin/users/:id` - Delete user
- `POST /admin/users/:id/regenerate-key` - Regenerate user key
- `GET /admin/stats/overview` - Usage statistics overview
- `GET /admin/stats/by-user` - Usage by user
- `GET /admin/stats/by-model` - Usage by model
- `GET /admin/logs` - Recent request logs

## License

MIT