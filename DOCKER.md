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
- `linux/arm/v7` (armv7)

You can use the appropriate image tag or let Docker automatically select the correct architecture.

## Building Your Own Image

### Standard Build (current architecture)

```bash
docker build -t llamp:latest .
```

### Multi-Platform Build

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t llamp:latest \
  --push \
  .
```

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
