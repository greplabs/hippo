#!/bin/bash
# Build script for hippo-wasm
# Compiles the Rust code to WebAssembly and generates JavaScript bindings

set -e

echo "Building hippo-wasm for WebAssembly..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# Determine output directory
OUTPUT_DIR=${1:-"../hippo-tauri/ui/dist/wasm"}

# Build for web target with release optimizations
echo "Compiling to wasm32-unknown-unknown..."
wasm-pack build \
    --target web \
    --out-dir "$OUTPUT_DIR" \
    --release \
    --no-typescript

echo ""
echo "Build complete! WASM module generated at: $OUTPUT_DIR"

# Show file sizes
echo ""
echo "Generated files:"
ls -lh "$OUTPUT_DIR" | grep -E '\.(wasm|js)$'

# Get compressed size
WASM_FILE="$OUTPUT_DIR/hippo_wasm_bg.wasm"
if [ -f "$WASM_FILE" ]; then
    ORIGINAL_SIZE=$(wc -c < "$WASM_FILE")
    GZIP_SIZE=$(gzip -c "$WASM_FILE" | wc -c)
    echo ""
    echo "WASM file size:"
    echo "  Original: $(numfmt --to=iec-i --suffix=B $ORIGINAL_SIZE)"
    echo "  Gzipped:  $(numfmt --to=iec-i --suffix=B $GZIP_SIZE)"

    # Check if under target
    TARGET_SIZE=$((500 * 1024))  # 500KB
    if [ $GZIP_SIZE -lt $TARGET_SIZE ]; then
        echo "  ✓ Under 500KB target"
    else
        echo "  ⚠ Over 500KB target ($(numfmt --to=iec-i --suffix=B $TARGET_SIZE))"
    fi
fi

echo ""
echo "To use in your web app, add to your HTML:"
echo '<script type="module">'
echo '  import init, { search_local, fuzzy_match, semantic_score } from "./wasm/hippo_wasm.js";'
echo '  await init();'
echo '  // Now you can use the functions'
echo '</script>'
