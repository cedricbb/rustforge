#========================================================
# Dockerfile for CoDev.rs Agent - Multi-stage build
#========================================================

#--------------------------------------------------------
# Stage 1 : Base image with Rust toolchain
#--------------------------------------------------------
FROM rust:1.85 AS rust-base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install useful development tools
RUN cargo install cargo-watch cargo-audit

WORKDIR /app

#---------------------------------------------------------
# Stage 2 : Dependencies cache layer
#---------------------------------------------------------
FROM rust-base AS dependencies

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/codev-core/Cargo.toml ./crates/codev-core/
COPY crates/codev-cli/Cargo.toml ./crates/codev-cli/
COPY crates/codev-shared/Cargo.toml ./crates/codev-shared/

# Create placeholder source files to cache dependencies
RUN mkdir -p crates/codev-core/src crates/codev-cli/src crates/codev-shared/src && \
    echo "fn main() {}" > crates/codev-core/src/main.rs && \
    echo "pub fn main() {}" > crates/codev-core/src/lib.rs && \
    echo "pub fn main() {}" > crates/codev-shared/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf target/release/deps/codev*

#----------------------------------------------------------
# Stage 3 : Development image
#----------------------------------------------------------
FROM rust-base AS development

# Copy cached dependencies
COPY --from=dependencies /usr/local/cargo /usr/local/cargo
COPY --from=dependencies /app/target /app/target

# Copy source code
COPY . .

# Create directories
RUN mkdir -p workspace data config logs

# Set permissions
RUN chmod +x scripts/*.sh || true

# Install dev dependencies and prepare for hot reload
RUN cargo build

# Default command for development
CMD ["cargo", "run", "--bin", "codev-cli"]

#----------------------------------------------------------
# Stage 4 : Production build
#---------------------------------------------------------
FROM dependencies AS builder

# Copy actual source code
COPY crates/ ./crates/
COPY src/ ./src/ 2>/dev/null || true

# Build the application
RUN cargo build --release --bin codev-cli

# Strip the binary to redeuce size
RUN strip target/release/codev-cli

#---------------------------------------------------------
# Stage 5 : Runtime image
#---------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user for security
RUN groupadd -r codev && useradd -r -g codev codev

# Create necessary directories
RUN mdir -p /app/{workspace,data,config,logs} && \
    chown -R codev:codev /app

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/codev-cli /usr/local/bin/codev

# Copy configuration files
COPY config/ ./config/
COPY scripts/entrypoint.sh ./entrypoint.sh

# set permissions
RUN chmod +x entrypoint.sh && \
    chmod +x /usr/local/bin/codev

# Switch to non root user
USER codev

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
CMD curl -f http://localhost:8080/health || exit &

# Expose ports
EXPOSE 8080

# Default command
ENTRYPOINT [".:entrypoint.sh"]
CMD ["codev", "--help"]

#---------------------------------------------------------
# Stage 6 : Testing image (for CI/CD)
#--------------------------------------------------------
FROM rust-base AS testing

# Copy souce
COPY . .

# Install test dependencies
RUN cargo install cargo-tarpaulin

# Run tests and generate coverage
RUN cago test --workspace
RUN cargo tarpaulin --out Xml --output-dir coverage

# Store test results
VOLUME ["/app/coverage"]
