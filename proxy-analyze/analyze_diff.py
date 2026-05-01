#!/usr/bin/env python3
"""
Automated analysis tool to compare LiteLLM vs Llamp responses.
Identifies differences that may cause Qwen Code compatibility issues.
"""

import asyncio
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional
from dataclasses import dataclass, field, asdict
import aiohttp
from dotenv import load_dotenv

# Find the .env file (could be in script directory or parent)
script_dir = Path(__file__).parent
env_path = script_dir / ".env"

# Load environment variables from the script's directory
if env_path.exists():
    load_dotenv(env_path)
    print(f"Loaded .env from: {env_path}", file=sys.stderr)
else:
    # Try parent directory
    parent_env = script_dir.parent / ".env"
    if parent_env.exists():
        load_dotenv(parent_env)
        print(f"Loaded .env from: {parent_env}", file=sys.stderr)
    else:
        print("Warning: No .env file found", file=sys.stderr)

# Configuration
LITELLM_URL = os.getenv("LITELLM_URL", "http://localhost:4000")
LAMP_URL = os.getenv("LAMP_URL", "http://localhost:8080")
LITELLM_API_KEY = os.getenv("LITELLM_API_KEY")
LAMP_API_KEY = os.getenv("LAMP_API_KEY")
LITELLM_MODEL = os.getenv("LITELLM_MODEL")
LAMP_MODEL = os.getenv("LAMP_MODEL")

# Critical fields that Qwen Code may expect
CRITICAL_FIELDS = {
    "usage": {
        "prompt_tokens": "Number of tokens in the prompt",
        "completion_tokens": "Number of tokens in the completion",
        "total_tokens": "Total number of tokens"
    },
    "system_fingerprint": "System fingerprint for caching/compatibility",
    "object": "Response object type (should be 'chat.completion')",
    "created": "Unix timestamp of response creation",
    "id": "Unique identifier for the response",
    "choices": {
        "message": {
            "role": "Message role (system, user, assistant)",
            "content": "Message content"
        },
        "finish_reason": "Reason the generation finished (stop, length, etc.)"
    }
}


@dataclass
class DiffReport:
    """Report of differences between two responses."""
    field_path: str
    litellm_value: Any
    lamp_value: Any
    difference_type: str
    severity: str  # critical, warning, info
    description: str = ""


@dataclass
class AnalysisResult:
    """Complete analysis result."""
    timestamp: str
    litellm_url: str
    lamp_url: str
    model_comparison: str
    status_codes_match: bool
    headers_differences: list = field(default_factory=list)
    body_differences: list = field(default_factory=list)
    critical_issues: list = field(default_factory=list)
    is_compatible: bool = True


def analyze_field_difference(
    path: str,
    litellm_val: Any,
    lamp_val: Any,
    field_definition: Optional[Dict] = None
) -> Optional[DiffReport]:
    """Analyze a single field difference."""
    if litellm_val == lamp_val:
        return None

    # Determine difference type and severity
    diff_type = "value_mismatch"
    severity = "info"
    description = ""

    # Check for missing fields
    if lamp_val is None or (isinstance(lamp_val, (str, list, dict)) and len(lamp_val) == 0):
        if litellm_val is not None:
            diff_type = "missing_in_lamp"
            severity = "critical" if path in ["usage", "usage.prompt_tokens", "usage.completion_tokens"] else "warning"
            description = f"Field '{path}' is present in LiteLLM but missing/empty in Llamp"

    elif litellm_val is None or (isinstance(litellm_val, (str, list, dict)) and len(litellm_val) == 0):
        diff_type = "missing_in_litellm"
        severity = "warning"
        description = f"Field '{path}' is present in Llamp but missing/empty in LiteLLM"

    # Check for type mismatches
    elif type(litellm_val).__name__ != type(lamp_val).__name__:
        diff_type = "type_mismatch"
        severity = "critical"
        description = f"Type mismatch: LiteLLM={type(litellm_val).__name__}, Llamp={type(lamp_val).__name__}"

    # Check for numeric differences
    elif isinstance(litellm_val, (int, float)) and isinstance(lamp_val, (int, float)):
        diff_type = "numeric_difference"
        severity = "critical" if "token" in path.lower() else "warning"
        percent_diff = abs(litellm_val - lamp_val) / max(abs(litellm_val), 1) * 100
        description = f"Numeric difference: LiteLLM={litellm_val}, Llamp={lamp_val} ({percent_diff:.1f}% diff)"

    return DiffReport(
        field_path=path,
        litellm_value=litellm_val,
        lamp_value=lamp_val,
        difference_type=diff_type,
        severity=severity,
        description=description
    )


def flatten_dict(d: Dict, parent_key: str = '', sep: str = '.') -> Dict[str, Any]:
    """Flatten a nested dictionary."""
    items = []
    for k, v in d.items():
        new_key = f"{parent_key}{sep}{k}" if parent_key else k
        if isinstance(v, dict):
            items.extend(flatten_dict(v, new_key, sep).items())
        else:
            items.append((new_key, v))
    return dict(items)


def analyze_headers(litellm_headers: Dict, lamp_headers: Dict) -> list:
    """Analyze header differences, focusing on critical headers."""
    differences = []
    
    # Headers that are LiteLLM-specific and can be ignored (case-insensitive)
    LITELLM_ONLY_HEADERS = {
        'x-litellm-version',
        'x-litellm-model-id',
        'x-litellm-model-group',
        'x-litellm-model-api-base',
        'x-litellm-response-cost',
        'x-litellm-response-cost-original',
        'x-litellm-response-cost-discount-amount',
        'x-litellm-response-cost-margin-amount',
        'x-litellm-response-cost-margin-percent',
        'x-litellm-key-spend',
        'x-litellm-overhead-duration-ms',
        'x-litellm-response-duration-ms',
        'x-litellm-attempted-fallbacks',
        'x-litellm-attempted-retries',
        'x-litellm-callback-duration-ms',
        'x-litellm-call-id',
    }
    
    # Headers that are provider-specific infrastructure (case-insensitive)
    INFRASTRUCTURE_HEADERS = {
        'cf-cache-status',
        'cf-ray',
        'cf-request-id',
        'nel',
        'report-to',
        'alt-svc',
        'server',
        'date',
        'connection',
        'content-encoding',
        'transfer-encoding',
        'x-litellm',
        'llm_provider',
    }
    
    # Headers that must match (critical)
    CRITICAL_HEADERS = {
        'content-type',
        'content-length',
    }
    
    all_keys = set(litellm_headers.keys()) | set(lamp_headers.keys())

    for key in all_keys:
        lit_val = litellm_headers.get(key)
        lamp_val = lamp_headers.get(key)

        if lit_val != lamp_val:
            key_lower = key.lower()
            
            # Normalize Content-Type to check only the main type
            if key_lower == "content-type":
                lit_type = lit_val.split(';')[0].strip() if lit_val else ""
                lamp_type = lamp_val.split(';')[0].strip() if lamp_val else ""
                if lit_type == lamp_type:
                    continue  # Same main type, ignore parameter differences

            # Skip LiteLLM-only headers
            is_litellm_only = False
            for header in LITELLM_ONLY_HEADERS:
                if key_lower.startswith(header):
                    is_litellm_only = True
                    break
            if is_litellm_only:
                continue
                
            # Skip infrastructure headers (Cloudflare, server info)
            is_infrastructure = False
            for header in INFRASTRUCTURE_HEADERS:
                if key_lower.startswith(header):
                    is_infrastructure = True
                    break
            if is_infrastructure:
                continue

            # Only report critical headers as warnings
            if key_lower in CRITICAL_HEADERS:
                severity = "warning"
            else:
                severity = "info"  # Non-critical header differences
            
            differences.append(DiffReport(
                field_path=f"headers.{key}",
                litellm_value=lit_val,
                lamp_value=lamp_val,
                difference_type="header_mismatch",
                severity=severity,
                description=f"Header '{key}' differs between providers"
            ))

    return differences


def analyze_body(
    litellm_body: Dict,
    lamp_body: Dict
) -> tuple[list[DiffReport], list[DiffReport]]:
    """Analyze body differences, focusing on critical fields."""
    critical_issues = []
    warnings = []

    # Flatten both bodies for easier comparison
    litellm_flat = flatten_dict(litellm_body)
    lamp_flat = flatten_dict(lamp_body)

    all_keys = set(litellm_flat.keys()) | set(lamp_flat.keys())

    for key in all_keys:
        lit_val = litellm_flat.get(key)
        lamp_val = lamp_flat.get(key)

        # Check if this is a critical field
        is_critical = any(
            key.startswith(crit_key) or key == crit_key
            for crit_key in CRITICAL_FIELDS.keys()
        )

        diff = analyze_field_difference(key, lit_val, lamp_val)

        if diff:
            if is_critical and diff.severity in ["critical", "warning"]:
                critical_issues.append(diff)
            elif not is_critical and diff.severity == "warning":
                warnings.append(diff)

    return critical_issues, warnings


async def make_request(
    url: str,
    api_key: Optional[str],
    model: Optional[str],
    payload: Dict
) -> tuple[int, Dict, Dict]:
    """Make a request and return status, headers, and body."""
    # Remove trailing slash from URL if present
    clean_url = url.rstrip('/')
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}" if api_key else ""
    }

    # Override model if specified
    if model and "model" in payload:
        payload["model"] = model

    async with aiohttp.ClientSession() as session:
        try:
            async with session.post(
                f"{clean_url}/v1/chat/completions",
                headers=headers,
                json=payload
            ) as response:
                status = response.status
                resp_headers = dict(response.headers)
                try:
                    body = await response.json()
                except:
                    body_text = await response.text()
                    body = {"raw": body_text}

                return status, resp_headers, body
        except Exception as e:
            return 500, {}, {"error": str(e)}


async def get_available_models(url: str, api_key: Optional[str] = None) -> list[str]:
    """Get list of available models from a provider."""
    # Remove trailing slash from URL if present
    clean_url = url.rstrip('/')
    endpoint = f"{clean_url}/v1/models"
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    async with aiohttp.ClientSession() as session:
        try:
            async with session.get(endpoint, headers=headers) as response:
                if response.status == 200:
                    data = await response.json()
                    models = data.get("data", [])
                    return [m.get("id", "") for m in models if m.get("id")]
                else:
                    return []
        except Exception:
            return []


async def run_comparison() -> AnalysisResult:
    """Run the full comparison analysis."""
    print("=" * 60)
    print("Llamp vs LiteLLM Compatibility Analyzer")
    print("=" * 60)
    print()

    # Check configuration
    if not LITELLM_URL or not LAMP_URL:
        print("ERROR: LITELLM_URL and LAMP_URL must be set in environment")
        sys.exit(1)

    print(f"LiteLLM URL: {LITELLM_URL}")
    print(f"Llamp URL: {LAMP_URL}")
    print(f"LiteLLM Model: {LITELLM_MODEL or 'not set'}")
    print(f"Llamp Model: {LAMP_MODEL or 'not set'}")
    print()

    if not LITELLM_MODEL:
        print("WARNING: LITELLM_MODEL is not set. Using 'test-model'.")
        print("         Set LITELLM_MODEL to match your actual model name.")
        print()

    if not LAMP_MODEL:
        print("WARNING: LAMP_MODEL is not set. Using 'test-model'.")
        print("         Set LAMP_MODEL to match your model alias configured in Llamp.")
        print()

    # Get available models for debugging
    print("Fetching available models...", file=sys.stderr)
    
    litellm_models = await get_available_models(LITELLM_URL, LITELLM_API_KEY)
    lamp_models = await get_available_models(LAMP_URL, LAMP_API_KEY)
    
    # If Llamp failed to fetch models, it might be down
    if not lamp_models and LAMP_URL:
        print("⚠️  WARNING: Could not fetch Llamp models - tunnel may be down or server unavailable", file=sys.stderr)
        print(f"💡 Consider starting tunnel with: llamp tunnel start --hostname yourdomain.com --token YOUR_TOKEN", file=sys.stderr)
        print(file=sys.stderr)

    if litellm_models:
        print(f"LiteLLM models ({len(litellm_models)} available):")
        for m in litellm_models[:5]:  # Show first 5
            print(f"  - {m}")
        if len(litellm_models) > 5:
            print(f"  ... and {len(litellm_models) - 5} more")
    else:
        print("LiteLLM: Could not fetch models (check URL/credentials)")
        print("💡 Try: curl -H 'Authorization: Bearer YOUR_KEY' https://ascent.x3t.eu/v1/models")

    if lamp_models:
        print(f"Llamp models ({len(lamp_models)} available):")
        for m in lamp_models[:5]:  # Show first 5
            print(f"  - {m}")
        if len(lamp_models) > 5:
            print(f"  ... and {len(lamp_models) - 5} more")
    else:
        print("Llamp: Could not fetch models (check URL/credentials)")
        print("💡 Try: curl -H 'Authorization: Bearer llamp-xxxxx' https://.../v1/models")
    print()

    # Check if configured models exist
    if LITELLM_MODEL and LITELLM_MODEL not in litellm_models:
        print(f"⚠️  WARNING: LiteLLM model '{LITELLM_MODEL}' not found!")
        if litellm_models:
            print(f"    Available models: {', '.join(litellm_models[:3])}...")
        print()

    if LAMP_MODEL and LAMP_MODEL not in lamp_models:
        print(f"⚠️  WARNING: Llamp model '{LAMP_MODEL}' not found!")
        if lamp_models:
            print(f"    Available models: {', '.join(lamp_models[:3])}...")
        print()

    # Test payload (similar to what Qwen Code would send)
    test_payload = {
        "model": LITELLM_MODEL or "test-model",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, how are you?"}
        ],
        "temperature": 0.7,
        "stream": False  # Test non-streaming first
    }

    print("Testing non-streaming request...")
    print(f"Test payload model: {test_payload['model']}")

    # Use the configured model if available, otherwise show warning
    if LITELLM_MODEL:
        print(f"LiteLLM will use model: {LITELLM_MODEL}")
    if LAMP_MODEL:
        print(f"Llamp will use model: {LAMP_MODEL}")
    print()

    # Make requests to both providers
    print("Making request to LiteLLM...")
    litellm_status, litellm_headers, litellm_body = await make_request(
        LITELLM_URL, LITELLM_API_KEY, LITELLM_MODEL, test_payload.copy()
    )
    print(f"LiteLLM response: {litellm_status}")

    print("Making request to Llamp...")
    lamp_status, lamp_headers, lamp_body = await make_request(
        LAMP_URL, LAMP_API_KEY, LAMP_MODEL, test_payload.copy()
    )
    print(f"Llamp response: {lamp_status}")
    print()

    # Analyze results
    result = AnalysisResult(
        timestamp=datetime.now().isoformat(),
        litellm_url=LITELLM_URL,
        lamp_url=LAMP_URL,
        model_comparison=f"LiteLLM: {LITELLM_MODEL or 'not set'} -> Llamp: {LAMP_MODEL or 'not set'}",
        status_codes_match=litellm_status == lamp_status
    )

    # Analyze headers
    result.headers_differences = analyze_headers(litellm_headers, lamp_headers)

    # Analyze body
    result.body_differences, body_warnings = analyze_body(litellm_headers, lamp_headers)
    result.critical_issues.extend(result.body_differences)
    result.critical_issues.extend(body_warnings)

    # Print results
    print("\n" + "=" * 60)
    print("RESULTS")
    print("=" * 60)

    print(f"\nStatus Codes: {'MATCH' if result.status_codes_match else 'DIFFER'}")
    print(f"  LiteLLM: {litellm_status}")
    print(f"  Llamp:   {lamp_status}")

    # Diagnostic for common errors
    def print_error_diagnostic(status: int, body: dict, provider: str, available_models: list[str], url: str, api_key: Optional[str]):
        if status == 404:
            print(f"\n💡 {provider} 404 Error Analysis:")
            if isinstance(body, dict):
                error_msg = body.get("error", {}).get("message", "") or body.get("message", "")
                if error_msg:
                    print(f"   Message: {error_msg}")
                if "model" in str(body).lower() or "not found" in str(body).lower():
                    print("   ⚠️  The model may not exist or the model name is incorrect")
                    print(f"   💡 Check that LITELLM_MODEL and LAMP_MODEL are set correctly")
                    if available_models:
                        print(f"   💡 Available models: {', '.join(available_models[:3])}...")
                    else:
                        print(f"   💡 Try fetching models manually:")
                        print(f"      curl -H 'Authorization: Bearer {api_key[:10]}...' {url}/v1/models")
            else:
                print(f"   Body: {body}")
        elif status == 401:
            print(f"\n💡 {provider} 401 Error Analysis:")
            print("   ⚠️  Authentication failed")
            print("   💡 Check that LITELLM_API_KEY and LAMP_API_KEY are set correctly")
        elif status == 403:
            print(f"\n💡 {provider} 403 Error Analysis:")
            print("   ⚠️  Forbidden - API key may not have permission")
        elif status >= 500:
            print(f"\n💡 {provider} {status} Error Analysis:")
            print(f"   ⚠️  Server error - {provider} may be down or misconfigured")

    if litellm_status >= 400:
        print_error_diagnostic(litellm_status, litellm_body, "LiteLLM", litellm_models, LITELLM_URL, LITELLM_API_KEY)
    if lamp_status >= 400:
        print_error_diagnostic(lamp_status, lamp_body, "Llamp", lamp_models, LAMP_URL, LAMP_API_KEY)

    # Check for critical issues
    critical = [d for d in result.critical_issues if d.severity == "critical"]
    warnings = [d for d in result.critical_issues if d.severity == "warning"]
    info = [d for d in result.critical_issues if d.severity == "info"]

    print(f"\nCritical Issues: {len(critical)}")
    for issue in critical:
        print(f"  ❌ {issue.field_path}")
        print(f"     LiteLLM: {issue.litellm_value}")
        print(f"     Llamp:   {issue.lamp_value}")
        print(f"     {issue.description}")

    print(f"\nWarnings: {len(warnings)}")
    for issue in warnings:
        print(f"  ⚠️  {issue.field_path}")
        print(f"     LiteLLM: {issue.litellm_value}")
        print(f"     Llamp:   {issue.lamp_value}")

    print(f"\nInfo: {len(info)}")
    for issue in info:
        print(f"  ℹ️  {issue.field_path}")

    # Determine compatibility - only if both providers returned 200
    both_success = litellm_status == 200 and lamp_status == 200
    result.is_compatible = len(critical) == 0 and both_success

    print("\n" + "=" * 60)
    if not both_success:
        print(f"STATUS: ⚠️  One or both providers returned errors")
        print(f"  LiteLLM: {litellm_status}")
        print(f"  Llamp:   {lamp_status}")
        print("\n💡 Run the analysis again after fixing the configuration issues above")
    else:
        print(f"COMPATIBILITY: {'✅ PASS' if result.is_compatible else '❌ FAIL'}")
    print("=" * 60)

    if not both_success:
        pass  # Already printed diagnostic above
    elif not result.is_compatible:
        print("\n⚠️  Llamp may have compatibility issues with Qwen Code")
        print("   Review the critical issues above")
    else:
        print("\n✅ Llamp appears compatible with standard OpenAI API")

    # Save report
    save_report(result)
    return result


def save_report(result: AnalysisResult):
    """Save the analysis report to a JSON file."""
    report_dir = Path(__file__).parent / "tmp" / "analysis"
    report_dir.mkdir(parents=True, exist_ok=True)

    report_path = report_dir / f"analysis_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

    report_data = {
        "timestamp": result.timestamp,
        "litellm_url": result.litellm_url,
        "lamp_url": result.lamp_url,
        "model_comparison": result.model_comparison,
        "status_codes_match": result.status_codes_match,
        "headers_differences": [asdict(d) for d in result.headers_differences],
        "body_differences": [asdict(d) for d in result.body_differences],
        "critical_issues": [asdict(d) for d in result.critical_issues],
        "is_compatible": result.is_compatible
    }

    with open(report_path, 'w') as f:
        json.dump(report_data, f, indent=2, default=str)

    print(f"\nReport saved to: {report_path}")


def main():
    """Main entry point."""
    try:
        result = asyncio.run(run_comparison())

        # Exit with appropriate code
        sys.exit(0 if result.is_compatible else 1)

    except KeyboardInterrupt:
        print("\nAnalysis interrupted")
        sys.exit(130)
    except Exception as e:
        print(f"\nError during analysis: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
