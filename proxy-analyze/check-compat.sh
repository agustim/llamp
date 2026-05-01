#!/bin/bash
# Check Llamp vs LiteLLM compatibility

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Load environment variables from .env file
if [ -f .env ]; then
    # Load .env variables (skip comments and empty lines, handle quotes)
    set -a
    source .env
    set +a
fi

# Run the analysis
python analyze_diff.py
exit_code=$?

echo ""
echo "Exit code: $exit_code"

# Save exit code to file for automation
mkdir -p tmp/analysis
echo "$exit_code" > tmp/analysis/last_exit_code.txt

exit $exit_code
