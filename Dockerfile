# Dockerfile for Llamp
# Builds for the current architecture using glibc

FROM rust:1.86-bookworm as builder

# Install OpenSSL development libraries for glibc builds
RUN apt-get update && apt-get install -y \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create build user to avoid root
RUN useradd -m -u 1000 builder
USER builder
WORKDIR /home/builder

# Copy source - use explicit copy for each file/directory
COPY --chown=builder:builder Cargo.toml .
COPY --chown=builder:builder Cargo.lock .
COPY --chown=builder:builder src ./src
COPY --chown=builder:builder migrations ./migrations

# Build for current architecture
RUN cargo build --release --target=x86_64-unknown-linux-gnu

# Create release directory
RUN mkdir -p /home/builder/release

# Copy binary
RUN cp target/x86_64-unknown-linux-gnu/release/llamp /home/builder/release/llamp

# Final minimal image for distribution
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /home/builder/release/llamp /app/llamp

RUN chmod +x /app/llamp

# Use exec form so arguments are passed to llamp
ENTRYPOINT ["/app/llamp"]
CMD ["--help"]
