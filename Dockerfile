# syntax=docker/dockerfile:1

# Base image with cargo-chef installed
FROM rustlang/rust:nightly AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# Planner stage - analyzes dependencies and creates recipe
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

# Server builder
FROM chef AS server-builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cook dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --package bw-server --recipe-path recipe.json

# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY migrations migrations
RUN cargo build --release --package bw-server

# Frontend builder
FROM chef AS frontend-builder

# Install trunk and wasm target
RUN rustup target add wasm32-unknown-unknown && \
    cargo install trunk --locked && \
    cargo install wasm-pack --locked

# Install Node.js for GM editor JS bundle
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Cook dependencies for WASM target (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown --package bw-frontend --package bw-gm-editor --recipe-path recipe.json

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY crates crates

# Build frontend with trunk
RUN cd crates/bw-frontend && trunk build --release

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
