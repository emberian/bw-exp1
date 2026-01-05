#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[run]${NC} $1"; }
warn() { echo -e "${YELLOW}[run]${NC} $1"; }
err() { echo -e "${RED}[run]${NC} $1" >&2; }

cleanup() {
    log "Shutting down..."
    kill $SERVER_PID 2>/dev/null || true
    kill $TRUNK_PID 2>/dev/null || true
    exit 0
}
trap cleanup SIGINT SIGTERM

# Check dependencies
command -v cargo >/dev/null || { err "cargo not found"; exit 1; }
command -v trunk >/dev/null || { err "trunk not found (cargo install trunk)"; exit 1; }

# Build frontend
log "Building frontend..."
(cd crates/bw-frontend && trunk build)

# Link dist to play dir for server
rm -rf play
ln -sf crates/bw-frontend/dist play

# Create static dir if missing
mkdir -p static

# Run server in background
cargo run -p bw-runtime &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Optionally run trunk in watch mode for frontend dev
if [[ "${1:-}" == "--watch" ]]; then
    log "Starting trunk watch mode..."
    (cd crates/bw-frontend && trunk watch) &
    TRUNK_PID=$!
fi

log "Ready! Open http://localhost:3000/play/"
wait $SERVER_PID
