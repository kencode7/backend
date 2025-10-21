# Use a Rust base image
FROM rust:1.76-slim-bookworm as builder

# Set the working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock to leverage Docker cache
COPY Cargo.toml Cargo.lock ./

# Build dependencies - this step is cached if Cargo.toml and Cargo.lock don't change
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && rm -rf src

# Copy the source code
COPY src ./src

# Build the release version of the application
RUN cargo build --release

# --- Final stage ---
FROM debian:bookworm-slim

# Install OpenSSL and other necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/backend ./backend

# Expose the port your Actix-web server listens on
EXPOSE 8080

# Set the entrypoint to run the compiled binary
CMD ["./backend"]