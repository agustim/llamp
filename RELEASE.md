# Release Process

This document describes how releases are automated for Llamp.

## Overview

Llamp uses GitHub Actions to automate:
1. **Continuous Integration (CI)**: Run tests, linting, and formatting checks on every push/PR
2. **Docker Build**: Cross-compile binaries for multiple Linux architectures using Docker
3. **Release Automation**: Create GitHub releases with pre-built binaries

## Workflows

### Docker Build Workflow (`.github/workflows/docker.yml`)

Runs on every push to `main`/`develop` and pull requests:

1. **Build Job**: Uses Docker Buildx to build multi-architecture Docker images for:
   - Linux amd64 (x86_64)
   - Linux arm64 (aarch64)
   - Linux arm/v7 (armv7)

### Release Workflow (`.github/workflows/release.yml`)

Runs when a new tag is pushed (e.g., `v0.3.0`):

1. **Build Job**: Uses Docker to cross-compile binaries for all architectures:
   - Linux amd64 (x86_64)
   - Linux arm64 (aarch64)
   - Linux armv7 (armv7)
2. **Release Job**: Creates a GitHub release with all binaries

## Creating a Release

### Manual Release

To manually create a release:

```bash
# Ensure you're on main branch and up to date
git checkout main
git pull

# Create and push a tag
git tag v0.3.0 -a -m "Release v0.3.0"
git push origin v0.3.0

# This will trigger the Release workflow
```

## Release Artifacts

Each release includes binaries for:
- `llamp-x86_64-unknown-linux-gnu` - Linux Intel/AMD 64-bit
- `llamp-aarch64-unknown-linux-gnu` - Linux ARM 64-bit (Raspberry Pi, ARM servers)
- `llamp-armv7-unknown-linux-gnueabihf` - Linux ARM v7 (older ARM devices)

## Versioning

Llamp follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for breaking changes
- **MINOR** version for new features
- **PATCH** version for bug fixes

## How It Works

### Cross-Compilation with Docker

The Docker-based cross-compilation approach provides:

1. **Consistent Build Environment**: All releases are built in the same Docker environment
2. **Multiple Architectures**: Single Docker build produces binaries for all supported architectures
3. **No Build Machine Configuration**: No need to configure cross-compilation tools on the CI runner
4. **Reliability**: Docker containers ensure reproducible builds across different CI runs

### Workflow Sequence

1. Push a tag (e.g., `v0.3.0`)
2. Release workflow runs:
   - Builds Docker image with cross-compilation tools
   - Compiles binaries for x86_64, aarch64, and armv7
   - Extracts binaries from Docker image
   - Uploads binaries as artifacts
3. Release job:
   - Downloads binaries from build job
   - Creates GitHub release with all binaries
