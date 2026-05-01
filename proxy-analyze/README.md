# Debug Proxy for Llamp

This directory contains tools for debugging Llamp compatibility issues, particularly with Qwen Code.

## Overview

Two tools are available to analyze differences between direct provider calls and Llamp-processed calls:

### Automated Analysis (`analyze_diff.py`)
- **Purpose**: Automatically compare LiteLLM vs Llamp responses
- **What it does**: Sends test requests to both providers and generates a detailed report of differences
- **Best for**: Quick compatibility checks, identifying critical issues
- **Output**: JSON report with severity ratings (critical/warning/info)

### Manual Debug Proxy (`debug_proxy.py`)
- **Purpose**: Capture and inspect all requests/responses in detail
- **What it does**: Acts as a proxy, logging every request and response to JSON files
- **Best for**: Deep debugging, analyzing specific requests, streaming analysis
- **Output**: Individual JSON files for each request/response

## Files

- `analyze_diff.py` - Automated compatibility analysis tool
- `debug_proxy.py` - Manual debug proxy with detailed logging

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

3. **Run the proxy** (for manual debugging):
```bash
python debug_proxy.py
```

4. **Configure your client** (e.g., Qwen Code) to use `http://localhost:9000/v1`
   - Set the API key to your Llamp proxy key (llamp-xxxxx format)

5. **Make test requests** and analyze the logs in `../tmp/debug_proxy/`

## Automated Analysis

For a quick compatibility check, use the automated analysis tool:

```bash
# Configure your .env file first (see Setup section above)

# Option 1: Use the helper script
./check-compat.sh

# Option 2: Run directly
python analyze_diff.py

# Exit code: 0 = compatible, 1 = compatibility issues found or config error
echo "Exit code: $?"
```

The tool will:
- Send test requests to both LiteLLM and Llamp
- Compare responses field by field
- Identify critical issues (missing usage, wrong types, etc.)
- Provide diagnostic tips for common errors (404, 401, etc.)
- Generate a JSON report with severity ratings
- Save the report to `tmp/analysis/`

### What It Checks

The automated analyzer checks for:

| Field | Severity | Description |
|-------|----------|-------------|
| `usage.prompt_tokens` | critical | Token counts must be present |
| `usage.completion_tokens` | critical | Completion token count required |
| `usage.total_tokens` | critical | Total token count required |
| `system_fingerprint` | warning | May be required by some clients |
| `choices.finish_reason` | critical | Must be present and valid |
| `object` | info | Response type should match |
| `headers.Content-Type` | warning | Should be `application/json` |

### Model Validation

The analyzer automatically:
- Fetches available models from both providers
- Shows which models are configured vs available
- Warns if configured model doesn't exist
- Suggests available models when a 404 error occurs

### Understanding the Report

**Critical Issues**: These will likely cause Qwen Code to fail. Examples:
- Missing `usage` object
- Token counts are zero when they should have values
- Type mismatches (string vs integer)

**Warnings**: May cause issues in some cases:
- Missing optional fields
- Header differences

**Info**: Informational only:
- Non-critical field differences

## Troubleshooting

If you encounter issues:

1. Ensure `aiohttp` and `python-dotenv` are installed: `pip install aiohttp python-dotenv`
2. Verify LiteLLM and Llamp are running and accessible
3. Check that the proxy port is not already in use
4. Review the proxy logs for error messages
5. For `analyze_diff.py`, check that environment variables are properly set

## Examples

### Example: Running Automated Analysis

```bash
# Configure your .env file first
cp .env.example .env
# Edit .env with your credentials (especially LITELLM_MODEL and LAMP_MODEL)

# Run the analysis
./check-compat.sh

# Or run directly
python analyze_diff.py

# Output:
# ============================================================
# Llamp vs LiteLLM Compatibility Analyzer
# ============================================================
#
# LiteLLM URL: http://localhost:4000
# Llamp URL: http://localhost:8080
# LiteLLM Model: gpt-4
# Llamp Model: gpt-4-alias
#
# Fetching available models...
# LiteLLM models (3 available):
#   - gpt-4
#   - gpt-4-turbo
#   - gpt-3.5-turbo
# Llamp models (2 available):
#   - gpt-4-alias
#   - claude-3-5-sonnet
#
# Testing non-streaming request...
# Test payload model: gpt-4
# LiteLLM will use model: gpt-4
# Llamp will use model: gpt-4-alias
#
# Making request to LiteLLM...
# LiteLLM response: 200
# Making request to Llamp...
# Llamp response: 200
#
# ============================================================
# RESULTS
# ============================================================
#
# Status Codes: MATCH
#   LiteLLM: 200
#   Llamp:   200
#
# Critical Issues: 0
#
# Warnings: 1
#   system_fingerprint differs
#
# ============================================================
# COMPATIBILITY: ✅ PASS
# ============================================================
#
# Report saved to: tmp/analysis/analysis_20260501_120000.json
```

### Example: Debugging with Manual Proxy

```bash
# Start the proxy
python debug_proxy.py

# In another terminal, configure Qwen Code to use:
# Base URL: http://localhost:9000/v1
# API Key: your-lamp-proxy-key

# Make a request through Qwen Code

# Check the logs
ls -la tmp/debug_proxy/
cat tmp/debug_proxy/llamp_request_0001.json
cat tmp/debug_proxy/llamp_response_0001.json
```

## Integration with Llamp

This proxy is part of Llamp's debugging toolkit. See the main [README](../README.md) for more information about Llamp.

## License

MIT License - See LICENSE file for details.
