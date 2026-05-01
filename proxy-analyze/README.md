# Debug Proxy for Llamp

This directory contains tools for debugging Llamp compatibility issues, particularly with Qwen Code.

## Overview

The debug proxy helps analyze differences between direct provider calls and Llamp-processed calls by:

- Capturing all requests and responses
- Logging to JSON files for easy comparison
- Enabling side-by-side analysis of provider vs Llamp behavior

## Files

- `debug_proxy.py` - Main Python script for the debug proxy

## Setup

1. **Install dependencies:**
```bash
pip install aiohttp
```

2. **Set environment variables:**

   **Option 1: Using environment variables**
   ```bash
   export LITELLM_URL="http://localhost:4000"  # Your LiteLLM instance
   export LAMP_URL="http://localhost:8080"      # Your Llamp instance
   export LITELLM_API_KEY="your-key"            # Optional: API key for LiteLLM
   export LAMP_API_KEY="your-key"               # Optional: API key for Llamp (llamp-xxxxx format)
   export PROXY_PORT="9000"                     # Proxy port (optional, default: 9000)
   ```

   **Option 2: Using .env file**
   Create a `.env` file in the `proxy-analyze/` directory (see `.env.example` for template):
   ```env
   LITELLM_URL=http://localhost:4000
   LAMP_URL=http://localhost:8080
   LITELLM_API_KEY=your-key
   LAMP_API_KEY=your-key
   LITELLM_MODEL=your-model-name
   LAMP_MODEL=your-model-alias
   PROXY_PORT=9000
   ```

3. **Run the proxy:**
```bash
python debug_proxy.py
```

4. **Configure your client** (e.g., Qwen Code) to use `http://localhost:9000/v1`
   - Set the API key to your Llamp proxy key (llamp-xxxxx format)

5. **Make test requests** and analyze the logs in `../tmp/debug_proxy/`

## Usage

After running the proxy, make identical requests through both:
- Direct LiteLLM (configure client to use proxy, which forwards to LiteLLM)
- Llamp (configure client to use proxy, which forwards to Llamp)

Compare the logs to identify any differences in:
- Request headers
- Request body structure
- Response headers
- Response body structure
- Streaming format (if applicable)

## Logs

Logs are saved to `../tmp/debug_proxy/` with the following naming convention:

**Requests:**
- `litellm_request_*.json` - Direct requests to LiteLLM
- `llamp_request_*.json` - Requests to Llamp

**Responses:**
- `litellm_response_*.json` - Direct responses from LiteLLM
- `llamp_response_*.json` - Responses from Llamp

Each log file contains:
- Request/response ID and timestamp
- HTTP method and path
- All headers
- Body content
- Status code (for responses)

## Troubleshooting

If you encounter issues:

1. Ensure `aiohttp` is installed: `pip install aiohttp`
2. Verify LiteLLM and Llamp are running and accessible
3. Check that the proxy port is not already in use
4. Review the proxy logs for error messages

## Integration with Llamp

This proxy is part of Llamp's debugging toolkit. See the main [README](../README.md) for more information about Llamp.
