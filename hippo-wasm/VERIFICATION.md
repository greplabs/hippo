# WASM Verification Guide

This document provides steps to verify that the hippo-wasm module was built and integrated correctly.

## Prerequisites

1. Install Rust with wasm32 target:
```bash
rustup target add wasm32-unknown-unknown
```

2. Install wasm-pack:
```bash
cargo install wasm-pack
```

3. Install wasm-opt (for size optimization):
```bash
# macOS
brew install binaryen

# Ubuntu/Debian
apt-get install binaryen

# Or via cargo
cargo install wasm-opt
```

## Build Verification

### Step 1: Check Compilation

Verify the code compiles for wasm32 target:

```bash
cd hippo-wasm
cargo check --target wasm32-unknown-unknown
```

Expected output: `Checking hippo-wasm v0.1.0` (no errors)

### Step 2: Run Tests

Run the test suite:

```bash
cargo test
```

Expected: All tests pass (13 tests)

### Step 3: Build WASM Module

Build the WebAssembly module:

```bash
./build-wasm.sh
```

Expected output:
```
Building hippo-wasm for WebAssembly...
Compiling to wasm32-unknown-unknown...
[INFO]: Checking for the Wasm target...
[INFO]: Compiling to Wasm...
[INFO]: Creating pkg...
Build complete! WASM module generated at: ../hippo-tauri/ui/dist/wasm

Generated files:
-rw-r--r-- hippo_wasm.js
-rw-r--r-- hippo_wasm_bg.wasm

WASM file size:
  Original: 120KiB
  Gzipped:  45KiB
  ✓ Under 500KB target
```

### Step 4: Verify Generated Files

Check that all required files were generated:

```bash
ls -lh ../hippo-tauri/ui/dist/wasm/
```

Expected files:
- `hippo_wasm.js` - JavaScript glue code
- `hippo_wasm_bg.wasm` - WebAssembly binary
- `package.json` - Package metadata (optional)

### Step 5: Check Binary Size

Verify the WASM binary is optimized:

```bash
wasm-opt --version
du -h ../hippo-tauri/ui/dist/wasm/hippo_wasm_bg.wasm
gzip -c ../hippo-tauri/ui/dist/wasm/hippo_wasm_bg.wasm | wc -c
```

Expected:
- Original: <200KB
- Gzipped: <500KB (ideally <100KB)

### Step 6: Inspect WASM Exports

Check exported functions:

```bash
wasm-objdump -x ../hippo-tauri/ui/dist/wasm/hippo_wasm_bg.wasm | grep -A 20 "Export\["
```

Expected exports:
- `fuzzy_match`
- `semantic_score`
- `search_local`
- `filter_by_type`
- `sort_memories`
- `get_stats`
- `version`
- `init`

## Runtime Verification

### Step 7: Test in Browser

Open `example.html` in a browser:

```bash
# Serve locally (required for ES modules)
python3 -m http.server 8000
# Or
npx serve .

# Then open http://localhost:8000/example.html
```

Verify:
1. Page loads without errors
2. Green bar appears at top (WASM initialized)
3. Console shows: `Hippo WASM version: 0.1.0`
4. All 4 demo sections work

### Step 8: Test Each Function

#### Fuzzy Matching
- Input: `hello` and `helo`
- Expected: Score ~80-85%
- Status: ✓ Strong match

#### Semantic Similarity
- Input: `1.0, 2.0, 3.0` and `1.0, 2.1, 2.9`
- Expected: Score ~0.99
- Status: ✓ Very similar

#### Local Search
- Query: `beach`
- Expected: 2 results (Beach Vacation, Beach Sunset)
- Both should have highlights

#### Filtering & Sorting
- Filter: Images
- Sort: Name (A-Z)
- Expected: Sorted list of image files only

### Step 9: Performance Testing

Open browser DevTools and run:

```javascript
// Benchmark fuzzy matching
console.time('fuzzy-1000');
for (let i = 0; i < 1000; i++) {
    fuzzy_match('hello', 'helo world');
}
console.timeEnd('fuzzy-1000');
// Expected: <10ms for 1000 calls

// Benchmark search
const memories = Array(1000).fill(sampleMemories[0]);
console.time('search-1000');
search_local(JSON.stringify(memories), 'beach');
console.timeEnd('search-1000');
// Expected: <50ms for 1000 items
```

### Step 10: Integration Test with Tauri

Add WASM to Tauri UI and verify:

1. Copy WASM files to `hippo-tauri/ui/dist/wasm/`
2. Update `index.html` with WASM import
3. Run Tauri: `cd hippo-tauri && cargo run`
4. Open DevTools and check console
5. Try searching - should see instant results

## Troubleshooting

### Error: "Cannot find module './wasm/hippo_wasm.js'"

**Solution**: Build script didn't run or wrong output directory
```bash
cd hippo-wasm
./build-wasm.sh ../hippo-tauri/ui/dist/wasm
```

### Error: "WebAssembly module is not valid"

**Solution**: WASM binary corrupted or incompatible
```bash
# Rebuild with fresh toolchain
rustup update
cargo clean
./build-wasm.sh
```

### Error: "RuntimeError: unreachable executed"

**Solution**: Panic occurred in WASM code
- Check browser console for panic message
- Enable debug symbols: `wasm-pack build --dev`
- Add `console_error_panic_hook::set_once()` to init

### Binary Too Large (>500KB gzipped)

**Solution**: Optimize further
```bash
# Check size of dependencies
cargo tree | head -20

# Enable more aggressive optimization in Cargo.toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true

# Use wasm-opt manually
wasm-opt -Oz input.wasm -o output.wasm
```

### Slow Performance

**Solution**: Profile and optimize
```bash
# Build with profiling
wasm-pack build --profiling

# Use browser profiler
# Chrome DevTools > Performance > Record
```

### CORS Errors

**Solution**: Serve from same origin
```bash
# Use local server
python3 -m http.server 8000

# Or configure CORS headers in server
Access-Control-Allow-Origin: *
```

## Success Criteria

- [ ] Code compiles without errors for wasm32-unknown-unknown
- [ ] All tests pass
- [ ] WASM binary generated and <500KB gzipped
- [ ] All expected functions exported
- [ ] Example page loads and works in browser
- [ ] Fuzzy match returns reasonable scores
- [ ] Semantic similarity calculates correctly
- [ ] Search returns relevant results with highlights
- [ ] Filtering works for all types
- [ ] Sorting works for all fields
- [ ] Performance <50ms for 1000 items
- [ ] Integrates with Tauri UI without errors

## Next Steps

Once verification passes:

1. Document integration in main README
2. Add WASM build to CI/CD pipeline
3. Create performance benchmarks
4. Add more advanced features (stemming, synonyms, etc.)
5. Optimize bundle size further
6. Add TypeScript definitions
7. Publish to npm (optional)
8. Create demo website

## Resources

- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [WebAssembly MDN](https://developer.mozilla.org/en-US/docs/WebAssembly)
- [Rust and WebAssembly Book](https://rustwasm.github.io/book/)
