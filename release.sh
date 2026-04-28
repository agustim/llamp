#!/bin/bash
set -e

echo "=== Llamp Release Script ==="
echo ""

# Check if docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

# Check if tag is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <tag>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

TAG="$1"

# Verify tag format
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Tag must be in format v0.1.0"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Creating release: $TAG"
echo ""

# Step 1: Build with Docker
echo "Step 1: Building binaries with Docker..."
docker build -f Dockerfile.cross -t llamp-builder .

# Step 2: Extract binaries
echo "Step 2: Extracting binaries..."
mkdir -p release

docker create --name temp-extract llamp-builder > /dev/null

docker cp temp-extract:/app/llamp-x86_64 ./release/llamp-${TAG}-x86_64-unknown-linux-gnu
docker cp temp-extract:/app/llamp-aarch64 ./release/llamp-${TAG}-aarch64-unknown-linux-gnu

docker rm temp-extract > /dev/null

echo ""
echo "Binaries extracted:"
ls -la release/

# Step 3: Create GitHub release
echo ""
echo "Step 3: Creating GitHub release..."
echo "Note: You need GITHUB_TOKEN set in your environment"

if [ -z "$GITHUB_TOKEN" ]; then
    echo "Warning: GITHUB_TOKEN not set. Skipping GitHub release."
    echo "Set GITHUB_TOKEN and rerun to create GitHub release."
else
    echo "Creating GitHub release with token..."
    # Use gh CLI to create release
    gh release create "$TAG" \
        ./release/llamp-${TAG}-x86_64-unknown-linux-gnu \
        ./release/llamp-${TAG}-aarch64-unknown-linux-gnu \
        --title "Release $TAG" \
        --generate-notes
    
    echo ""
    echo "GitHub release created!"
fi

echo ""
echo "=== Release process complete! ==="
