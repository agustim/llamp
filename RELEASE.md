# Release Process

This document describes how releases are automated for Llamp.

## Overview

Llamp uses GitHub Actions to automate:
1. **Continuous Integration (CI)**: Run tests, linting, and formatting checks on every push/PR
2. **Release Automation**: Create GitHub releases with binaries for multiple architectures

## Workflows

### CI/CD Workflow (`.github/workflows/ci.yml`)

Runs on every push to `main`/`develop` and pull requests:

1. **Test Job**: Runs `cargo test`, `cargo clippy`, and `cargo fmt`
2. **Build Job**: Builds binaries for:
   - Linux x86_64
   - Linux aarch64
   - macOS x86_64
   - macOS aarch64
3. **Release Job**: Creates GitHub release when tags are pushed

### Release-plz Workflow (`.github/workflows/release-plz.yml`)

Automatically creates release PRs when changes are merged to `main`:

1. Runs all checks (tests, clippy, fmt)
2. Creates a release PR with version bump and changelog

## Creating a Release

### Automatic Release (Recommended)

When you merge to `main`, release-plz will:
1. Detect version bump (minor for features, patch for bugs)
2. Create a release PR
3. Merge the PR to create the release

### Manual Release

To manually create a release:

```bash
# Ensure you're on main branch and up to date
git checkout main
git pull

# Create and push a tag
git tag v0.2.0
git push origin v0.2.0

# This will trigger the Release workflow
```

## Release Artifacts

Each release includes binaries for:
- `llamp-linux-x86_64` - Linux Intel/AMD 64-bit
- `llamp-linux-aarch64` - Linux ARM 64-bit (Raspberry Pi, etc.)
- `llamp-macos-x86_64` - macOS Intel
- `llamp-macos-aarch64` - macOS Apple Silicon

## Versioning

Llamp follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for breaking changes
- **MINOR** version for new features
- **PATCH** version for bug fixes

The version is automatically determined by:
- Conventional commit messages
- Release-plz configuration

## Configuration

See `release-plz.yml` for release configuration. Key settings:

- `release-count`: Number of releases to keep
- `changelog`: Enable changelog generation
- `git-push`: Automatically push releases
