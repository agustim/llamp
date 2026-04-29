# Docker Images for Llamp

Llamp is a universal LLM gateway that provides an OpenAI-compatible API for local LLM backends.

## Quick Start

```bash
# Pull the latest image
docker pull agustim/llamp:latest

# Run with a volume for the database
docker run -p 8080:8080 \
  -v ./llamp.db:/app/llamp.db \
  agustim/llamp:latest \
  serve --database sqlite:///app/llamp.db
```

## Multi-Architecture Support

We provide pre-built images for multiple architectures:

- `linux/amd64` (x86_64)
- `linux/arm64` (aarch64)

You can use the appropriate image tag or let Docker automatically select the correct architecture.

## Building Your Own Image

### Standard Build (current architecture)

```bash
docker build -t llamp:latest .
```

### Cross-Compilation Build

```bash
docker build -f Dockerfile.cross -t llamp-builder .
```

This creates a Docker image that contains binaries for all supported architectures:
- `llamp-x86_64` - Linux Intel/AMD 64-bit
- `llamp-aarch64` - Linux ARM 64-bit

### Extract Binaries from Cross-Compilation Image

```bash
# Create a temporary container
docker create --name temp-extract llamp-builder

# Extract binaries
docker cp temp-extract:/app/llamp-x86_64 ./llamp-x86_64
docker cp temp-extract:/app/llamp-aarch64 ./llamp-aarch64

# Clean up
docker rm temp-extract
```

### Multi-Platform Build

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t llamp:latest \
  --push \
  .
```

## Cross-Compilation with Docker (Detailed)

The `Dockerfile.cross` uses Docker to cross-compile Llamp for multiple Linux architectures:

1. Uses a Rust base image with all necessary build tools
2. Installs cross-compilation toolchains for ARM64 target
3. Builds binaries for x86_64 and aarch64
4. Packages all binaries in a minimal Debian runtime image

This approach ensures:
- **Consistent builds** across different development environments
- **No need to configure cross-compilation** on your local machine
- **Reproducible releases** using the same Docker image

## Configuration

Llamp can be configured via command-line arguments:

```bash
docker run -p 8080:8080 \
  -v ./llamp.db:/app/llamp.db \
  agustim/llamp:latest \
  serve \
  --host 0.0.0.0 \
  --port 8080 \
  --database sqlite:///app/llamp.db
```

## Environment Variables

No environment variables are currently required, but you can set them if needed for your deployment.

## Database

Llamp uses SQLite by default. Mount a volume for persistence:

```bash
-v ./llamp.db:/app/llamp.db
```

## Tags

- `latest` - Latest stable release
- `v0.3.0` - Specific version tag
- `main` - Latest from main branch

## License

MIT License - see LICENSE file for details.
