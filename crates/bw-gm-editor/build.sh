#!/bin/bash
# Build script for bw-gm-editor
#
# This script builds both the WASM module and the JS bundle.
# The output goes to dist/ which should be copied to the frontend's dist/gm-editor/

set -e

cd "$(dirname "$0")"

echo "=== Building GM Editor ==="

# 1. Build WASM with wasm-pack
echo "Building WASM..."
wasm-pack build --target web --out-dir dist --out-name bw_gm_editor

# 2. Install npm dependencies and build JS
echo "Building JS bundle..."
npm install
npm run build:js

# 3. Move JS bundle to dist
mv dist/js/editor.js dist/editor.js 2>/dev/null || true
rmdir dist/js 2>/dev/null || true

echo "=== Build complete ==="
echo "Output in: $(pwd)/dist/"
echo ""
echo "Files:"
ls -la dist/
