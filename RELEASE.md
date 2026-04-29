# Release Process

This document describes how releases are automated for Llamp.

## Overview

Llamp uses GitHub Actions to automate:
1. **Continuous Integration (CI)**: Run tests, linting, and formatting checks on every push/PR
2. **Release Automation**: Create GitHub releases with pre-built binaries for multiple architectures

## Workflows

### Docker Build Workflow (`.github/workflows/docker.yml`)

Runs on every push to `main`/`develop` and pull requests:

1. **Build Job**: Uses Docker Buildx to build multi-architecture Docker images for:
   - Linux amd64 (x86_64)
   - Linux arm64 (aarch64)
   - Linux arm/v7 (armv7)
2. **Output**: Docker images are built but not pushed (for testing)

### Release Workflow (`.github/workflows/release.yml`)

Runs when a new tag is pushed (e.g., `v0.3.0`):

1. **Build Job**: Compiles binaries for all supported architectures:
   - Linux amd64 (x86_64)
   - Linux arm64 (aarch64)
   - Linux armv7 (armv7)
   - Linux arm (armv6)
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
- `llamp-arm-unknown-linux-gnueabihf` - Linux ARM v6 (Raspberry Pi Zero, etc.)

## Versioning

Llamp follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for breaking changes
- **MINOR** version for new features
- **PATCH** version for bug fixes

## How It Works

### Release Process

1. Push a tag (e.g., `v0.3.0`)
2. Release workflow runs on Ubuntu:
   - Installs Rust toolchain with all target architectures
   - Compiles binaries for x86_64, aarch64, armv7, and arm
   - Uploads binaries as artifacts
3. Release job:
   - Downloads binaries from build job
   - Creates GitHub release with all binaries
   - Generates release notes from commit history

### Prerequisites

- Rust toolchain with targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`, `arm-unknown-linux-gnueabihf`
- Cross-compilation tools: `gcc-aarch64-linux-gnu`, `gcc-arm-linux-gnueabihf`
- OpenSSL development libraries
