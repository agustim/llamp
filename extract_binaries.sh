#!/bin/bash
set -e

echo "=== Extracting Llamp binaries ==="
echo ""

# Check if docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Create output directory
mkdir -p release

# Check if image exists
if ! docker images llamp-builder --format "{{.Repository}}" | grep -q llamp-builder; then
    echo "Error: Image 'llamp-builder' not found. Run build-cross.sh first."
    exit 1
fi

# Create temporary container
echo "Creating temporary container..."
docker create --name temp-extract llamp-builder > /dev/null

# Extract binaries
echo "Extracting binaries..."
docker cp temp-extract:/app/llamp-x86_64 ./release/llamp-x86_64-unknown-linux-gnu
docker cp temp-extract:/app/llamp-aarch64 ./release/llamp-aarch64-unknown-linux-gnu

# Remove temporary container
docker rm temp-extract > /dev/null

echo ""
echo "=== Extraction complete! ==="
echo ""
echo "Binaries saved to ./release/:"
ls -la release/
