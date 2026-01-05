# syntax=docker/dockerfile:1

# Build stage for the server
FROM rustlang/rust:nightly AS server-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/bw-core/Cargo.toml crates/bw-core/
COPY crates/bw-shared/Cargo.toml crates/bw-shared/
COPY crates/bw-scripting/Cargo.toml crates/bw-scripting/
COPY crates/bw-server/Cargo.toml crates/bw-server/
COPY crates/bw-frontend/Cargo.toml crates/bw-frontend/
COPY crates/bw-gm-editor/Cargo.toml crates/bw-gm-editor/

# Create dummy source files to cache dependencies
RUN mkdir -p crates/bw-core/src crates/bw-shared/src crates/bw-scripting/src crates/bw-server/src crates/bw-frontend/src crates/bw-gm-editor/src && \
    echo "pub fn dummy() {}" > crates/bw-core/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/bw-shared/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/bw-scripting/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/bw-server/src/lib.rs && \
    echo "fn main() {}" > crates/bw-server/src/main.rs && \
    echo "pub fn dummy() {}" > crates/bw-frontend/src/lib.rs && \
    echo "fn main() {}" > crates/bw-frontend/src/main.rs && \
    echo "pub fn dummy() {}" > crates/bw-gm-editor/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --package bw-server 2>/dev/null || true

# Copy actual source code
COPY crates/bw-core/src crates/bw-core/src
COPY crates/bw-shared/src crates/bw-shared/src
COPY crates/bw-scripting/src crates/bw-scripting/src
COPY crates/bw-server/src crates/bw-server/src
COPY migrations migrations

# Build the server
RUN touch crates/bw-core/src/lib.rs crates/bw-shared/src/lib.rs crates/bw-scripting/src/lib.rs crates/bw-server/src/lib.rs crates/bw-server/src/main.rs && \
    cargo build --release --package bw-server

# Build stage for the frontend
FROM rustlang/rust:nightly AS frontend-builder

WORKDIR /app

# Install trunk, wasm-pack, and wasm target
RUN rustup target add wasm32-unknown-unknown && \
    cargo install trunk --locked && \
    cargo install wasm-pack --locked

# Install Node.js for GM editor JS bundle
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/bw-shared/Cargo.toml crates/bw-shared/
COPY crates/bw-frontend/Cargo.toml crates/bw-frontend/
COPY crates/bw-gm-editor/Cargo.toml crates/bw-gm-editor/

# Create dummy files for dependency caching
RUN mkdir -p crates/bw-shared/src crates/bw-frontend/src crates/bw-gm-editor/src && \
    echo "pub fn dummy() {}" > crates/bw-shared/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/bw-frontend/src/lib.rs && \
    echo "fn main() {}" > crates/bw-frontend/src/main.rs && \
    echo "pub fn dummy() {}" > crates/bw-gm-editor/src/lib.rs

# Copy frontend build files
COPY crates/bw-frontend/index.html crates/bw-frontend/
COPY crates/bw-frontend/Trunk.toml crates/bw-frontend/
COPY crates/bw-frontend/style crates/bw-frontend/style

# Copy GM editor build files
COPY crates/bw-gm-editor/package.json crates/bw-gm-editor/
COPY crates/bw-gm-editor/vite.config.js crates/bw-gm-editor/
COPY crates/bw-gm-editor/js crates/bw-gm-editor/js

# Pre-build dependencies
RUN cd crates/bw-frontend && trunk build --release 2>/dev/null || true

# Copy actual source
COPY crates/bw-shared/src crates/bw-shared/src
COPY crates/bw-frontend/src crates/bw-frontend/src
COPY crates/bw-gm-editor/src crates/bw-gm-editor/src

# Build frontend
RUN touch crates/bw-shared/src/lib.rs crates/bw-frontend/src/lib.rs crates/bw-frontend/src/main.rs && \
    cd crates/bw-frontend && trunk build --release

# Build GM editor (WASM + JS bundle)
RUN cd crates/bw-gm-editor && \
    npm install && \
    wasm-pack build --target web --out-dir dist --out-name bw_gm_editor && \
    npm run build:js && \
    mv dist/js/editor.js dist/editor.js 2>/dev/null || true

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy server binary
COPY --from=server-builder /app/target/release/blackwing-server /app/blackwing-server

# Copy WASM app to /play
COPY --from=frontend-builder /app/crates/bw-frontend/dist /app/play

# Copy GM editor to /play/gm-editor
COPY --from=frontend-builder /app/crates/bw-gm-editor/dist /app/play/gm-editor

# Copy static landing page
COPY static /app/static

# Copy scripts
COPY scripts /app/scripts

# Copy default config (can be overridden by volume mount)
COPY config.toml /app/config.toml

# Create data directory for SQLite
RUN mkdir -p /app/data

# Set environment
ENV RUST_LOG=bw_server=info,tower_http=info
ENV DATABASE_URL=sqlite:/app/data/blackwing.db

EXPOSE 3000

CMD ["/app/blackwing-server"]
