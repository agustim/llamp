#!/bin/bash
# Build multi-arch Docker images for Llamp
# Usage: ./build-docker.sh [version]

set -e

VERSION="${1:-latest}"

echo "Building Llamp Docker image for multiple architectures..."
echo "Version: $VERSION"

# Create a new buildx builder
docker buildx create --name llamp-builder --use

# Build for multiple platforms
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t agustim/llamp:$VERSION \
    -t agustim/llamp:amd64-$VERSION \
    -t agustim/llamp:arm64-$VERSION \
    --push \
    .

echo "Build complete!"
echo "Images pushed to agustim/llamp:$VERSION"
