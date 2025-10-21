# Backend Dockerfile for Render deployment

FROM rust:1.76-slim-bookworm as builder

# Install build dependencies required by native crates (openssl, libgit2, etc.)
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libssh2-1-dev \
    zlib1g-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the full backend project
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the release binary
RUN cargo build --release

# --- Final stage ---
FROM debian:bookworm-slim

# Minimal runtime deps
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libssh2-1 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary
COPY --from=builder /app/target/release/backend ./backend

# Expose port (Render provides PORT env var)
EXPOSE 8080

# Start server
CMD ["./backend"]