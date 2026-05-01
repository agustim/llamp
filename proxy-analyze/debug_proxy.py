#!/usr/bin/env python3
"""
Debug proxy to analyze differences between direct LiteLLM calls and Llamp calls.
This proxy sits between Qwen Code and the LLM provider to log all requests/responses.
"""

import asyncio
import json
import logging
import os
from datetime import datetime
from pathlib import Path
from typing import Optional

import aiohttp
from aiohttp import web
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Configuration (environment variables override .env)
LITELLM_URL = os.getenv("LITELLM_URL", "http://localhost:4000")
LAMP_URL = os.getenv("LAMP_URL", "http://localhost:8080")
LITELLM_API_KEY = os.getenv("LITELLM_API_KEY")  # Optional: API key for LiteLLM
LAMP_API_KEY = os.getenv("LAMP_API_KEY")        # Optional: API key for Llamp
LITELLM_MODEL = os.getenv("LITELLM_MODEL")      # Optional: Model name for LiteLLM
LAMP_MODEL = os.getenv("LAMP_MODEL")            # Optional: Model name for Llamp (model_alias)
PROXY_PORT = int(os.getenv("PROXY_PORT", "9000"))

# Paths for logging
LOG_DIR = Path(__file__).parent.parent / "tmp" / "debug_proxy"
LOG_DIR.mkdir(parents=True, exist_ok=True)


class DebugProxy:
    """Proxy that logs all requests and responses to analyze differences."""

    def __init__(self, target_url: str, name: str):
        self.target_url = target_url.rstrip('/')
        self.name = name
        self.request_count = 0

    async def proxy_request(self, request: web.Request) -> web.Response:
        """Proxy a request to the target URL and log details."""
        self.request_count += 1
        req_id = self.request_count

        # Read request body
        try:
            body = await request.read()
            body_str = body.decode('utf-8', errors='replace')
        except Exception as e:
            logger.error(f"Error reading request body: {e}")
            body_str = ""

        # Check if request is streaming
        is_streaming = False
        try:
            body_json = json.loads(body_str) if body_str else {}
            is_streaming = body_json.get("stream", False)
        except json.JSONDecodeError:
            pass

        # Log request
        logger.info(f"[{self.name} #{req_id}] Received request: {request.method} {request.path}")
        if is_streaming:
            logger.info(f"[{self.name} #{req_id}] Streaming request detected")
        logger.debug(f"[{self.name} #{req_id}] Headers: {dict(request.headers)}")
        logger.debug(f"[{self.name} #{req_id}] Body: {body_str[:500]}{'...' if len(body_str) > 500 else ''}")

        # Override model if configured
        model_key = "LITELLM_MODEL" if self.name == "litellm" else "LAMP_MODEL"
        target_model = globals().get(model_key)
        if target_model:
            try:
                body_json = json.loads(body_str) if body_str else {}
                if "model" in body_json:
                    logger.info(f"[{self.name} #{req_id}] Original model: {body_json['model']}")
                body_json["model"] = target_model
                body_str = json.dumps(body_json)
                body = body_str.encode('utf-8')
                logger.info(f"[{self.name} #{req_id}] Overriden model to: {target_model}")
            except json.JSONDecodeError as e:
                logger.warning(f"[{self.name} #{req_id}] Could not parse JSON body: {e}")

        # Save request to file
        self._save_request(req_id, request, body_str)

        try:
            # Forward request to target
            async with aiohttp.ClientSession() as session:
                # Start with headers from the original request
                headers = {k: v for k, v in request.headers.items()
                          if k.lower() not in ['host', 'content-length']}

                # Add/override API key if configured
                api_key = LITELLM_API_KEY if self.name == "litellm" else LAMP_API_KEY
                logger.info(f"[{self.name} #{req_id}] api_key from config: {api_key[:10] + '...' if api_key and len(api_key) > 10 else api_key}")
                logger.info(f"[{self.name} #{req_id}] Authorization header before: {headers.get('Authorization', 'NOT SET')}")
                
                if api_key:
                    # Always use the configured API key, overriding any existing Authorization header
                    headers['Authorization'] = f"Bearer {api_key}"
                    logger.info(f"[{self.name} #{req_id}] Using configured API key - Authorization set to: Bearer {api_key[:10]}...")

                logger.info(f"[{self.name} #{req_id}] Final headers: {list(headers.keys())}")

                # Log model being used
                try:
                    body_json = json.loads(body_str) if body_str else {}
                    if "model" in body_json:
                        logger.info(f"[{self.name} #{req_id}] Model in request: {body_json['model']}")
                except json.JSONDecodeError:
                    pass

                async with session.request(
                    method=request.method,
                    url=f"{self.target_url}{request.path}",
                    headers=headers,
                    data=body if request.method in ['POST', 'PUT'] else None
                ) as response:
                    # Handle streaming vs non-streaming responses
                    # Check if the RESPONSE is actually streaming (Content-Type is text/event-stream)
                    response_content_type = response.headers.get("Content-Type", "")
                    is_actual_streaming = response_content_type.startswith("text/event-stream")
                    
                    # Only do streaming if BOTH the request asked for it AND the response is SSE
                    should_stream = is_streaming and is_actual_streaming
                    
                    if should_stream:
                        logger.info(f"[{self.name} #{req_id}] Streaming response detected and will stream")
                        # For streaming, create a generator that yields chunks
                        async def stream_response():
                            logger.info(f"[{self.name} #{req_id}] Starting streaming response")
                            try:
                                async for line in response.content:
                                    if line:
                                        line_str = line.decode('utf-8', errors='replace')
                                        logger.debug(f"[{self.name} #{req_id}] Streaming chunk: {line_str[:100]}")
                                        yield line
                            except Exception as e:
                                logger.error(f"[{self.name} #{req_id}] Error streaming: {e}")
                        # Return StreamResponse with generator
                        # Remove Content-Encoding from headers (we've already decompressed)
                        proxy_headers = {k: v for k, v in response.headers.items()
                                        if k.lower() not in ['content-encoding', 'transfer-encoding']}
                        response_obj = web.StreamResponse(
                            status=response.status,
                            headers=proxy_headers
                        )
                        response_obj.content_type = "text/event-stream"
                        logger.info(f"[{self.name} #{req_id}] Streaming response headers: {list(proxy_headers.keys())}")
                        await response_obj.prepare(request)
                        async for chunk in stream_response():
                            await response_obj.write(chunk)
                        await response_obj.write_eof()
                        return response_obj
                    else:
                        # For non-streaming, read entire body
                        if is_streaming and not is_actual_streaming:
                            logger.warning(f"[{self.name} #{req_id}] Request asked for streaming but response is not SSE (Content-Type: {response_content_type}), returning non-streaming response")
                        resp_body = await response.read()
                        resp_body_str = resp_body.decode('utf-8', errors='replace')

                        # Log response
                        logger.info(f"[{self.name} #{req_id}] Response status: {response.status}")
                        logger.debug(f"[{self.name} #{req_id}] Response body: {resp_body_str[:500]}{'...' if len(resp_body_str) > 500 else ''}")

                        # Save response to file
                        self._save_response(req_id, response, resp_body_str)

                        # Copy headers but remove Content-Encoding (we've already decompressed)
                        proxy_headers = {k: v for k, v in response.headers.items()
                                        if k.lower() not in ['content-encoding', 'transfer-encoding']}
                        proxy_headers['Content-Length'] = str(len(resp_body))

                        # Return response
                        return web.Response(
                            status=response.status,
                            body=resp_body,
                            headers=proxy_headers
                        )

        except Exception as e:
            logger.error(f"[{self.name} #{req_id}] Error proxying request: {e}")
            return web.Response(
                status=502,
                text=str(e),
                content_type='text/plain'
            )

    def _save_request(self, req_id: int, request: web.Request, body: str):
        """Save request details to file."""
        timestamp = datetime.now().isoformat()
        request_log = {
            "id": req_id,
            "timestamp": timestamp,
            "name": self.name,
            "method": request.method,
            "path": request.path,
            "headers": dict(request.headers),
            "body": body
        }

        filepath = LOG_DIR / f"{self.name}_request_{req_id:04d}.json"
        with open(filepath, 'w') as f:
            json.dump(request_log, f, indent=2)

    def _save_response(self, req_id: int, response, body: str):
        """Save response details to file."""
        timestamp = datetime.now().isoformat()

        # Try to parse body as JSON for analysis
        try:
            body_json = json.loads(body)
        except json.JSONDecodeError:
            body_json = {"raw": body}

        response_log = {
            "id": req_id,
            "timestamp": timestamp,
            "name": self.name,
            "status": response.status,
            "headers": dict(response.headers),
            "body": body,
            "body_json": body_json
        }

        filepath = LOG_DIR / f"{self.name}_response_{req_id:04d}.json"
        with open(filepath, 'w') as f:
            json.dump(response_log, f, indent=2)


async def handle_proxy(request: web.Request) -> web.Response:
    """Handle requests and proxy to both LiteLLM and Llamp."""
    path = request.path

    # Determine target service based on query parameter or header
    # Use ?target=litellm or ?target=llamp, or X-Target header
    # Default to "llamp" (Llamp) as the primary target
    target = request.query.get("target", request.headers.get("X-Target", "llamp")).lower()

    if target == "litellm":
        logger.info(f"Proxying to LiteLLM (target={target})")
        return await litellm_proxy.proxy_request(request)
    else:
        logger.info(f"Proxying to Llamp (target={target})")
        return await lamp_proxy.proxy_request(request)


async def start_proxy():
    """Start the debug proxy server."""
    global litellm_proxy, lamp_proxy

    litellm_proxy = DebugProxy(LITELLM_URL, "litellm")
    lamp_proxy = DebugProxy(LAMP_URL, "llamp")

    app = web.Application()
    app.router.add_route('*', '/{path:.*}', handle_proxy)

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, 'localhost', PROXY_PORT)
    await site.start()

    logger.info(f"Debug proxy started on port {PROXY_PORT}")
    logger.info(f"LiteLLM URL: {LITELLM_URL}")
    logger.info(f"Llamp URL: {LAMP_URL}")
    logger.info(f"Log directory: {LOG_DIR}")

    return runner


async def stop_proxy(runner: web.AppRunner):
    """Stop the proxy server."""
    await runner.cleanup()
    logger.info("Proxy stopped")


async def run_proxy():
    """Run the proxy with proper async handling."""
    runner = await start_proxy()
    try:
        # Keep running until interrupted
        await asyncio.Event().wait()
    except asyncio.CancelledError:
        pass
    finally:
        await stop_proxy(runner)


def main():
    """Main entry point."""
    logger.info("Starting Debug Proxy...")
    logger.info("This proxy will log all requests and responses to analyze differences.")
    logger.info(f"Configure Qwen Code to use: http://localhost:{PROXY_PORT}")
    logger.info("Then compare logs in: " + str(LOG_DIR))

    try:
        asyncio.run(run_proxy())
    except KeyboardInterrupt:
        logger.info("Received shutdown signal")
    finally:
        logger.info("Debug proxy stopped")


if __name__ == "__main__":
    main()
