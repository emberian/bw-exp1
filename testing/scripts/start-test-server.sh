#!/bin/bash

# Start the test server for local E2E testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Clean up test database
rm -f "$PROJECT_ROOT/testing/test.db"
rm -f "$PROJECT_ROOT/testing/test.db-shm"
rm -f "$PROJECT_ROOT/testing/test.db-wal"

echo "Starting test server..."
cd "$PROJECT_ROOT"

DATABASE_URL="sqlite:./testing/test.db" \
BW_PORT=3001 \
RUST_LOG=bw_server=info \
cargo run --release -p bw-server
