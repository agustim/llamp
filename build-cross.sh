#!/bin/bash
set -e

echo "=== Llamp Cross-Compilation Script ==="
echo ""

# Check if docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building Llamp for multiple architectures..."
echo ""

# Build with Docker
docker build -f Dockerfile.cross -t llamp-builder .

echo ""
echo "=== Build complete! ==="
echo ""
echo "Binaries are available in the Docker image."
echo "To extract them, run:"
echo "  docker create --name temp llamp-builder"
echo "  docker cp temp:/app/llamp-x86_64 ./llamp-x86_64"
echo "  docker cp temp:/app/llamp-aarch64 ./llamp-aarch64"
echo "  docker rm temp"
echo ""
echo "Or use the extract_binaries.sh script"
