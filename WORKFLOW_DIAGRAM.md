# Release Workflow Architecture

## Workflow Job Structure

```
release.yml
│
├── build-cli (5 platforms)
│   ├── linux-x86_64 ✅
│   ├── linux-aarch64 ✅ (uses cross)
│   ├── macos-x86_64 ✅
│   ├── macos-aarch64 ✅
│   └── windows-x86_64 ✅
│
├── build-web (4 platforms)
│   ├── linux-x86_64 ✅
│   ├── macos-x86_64 ✅
│   ├── macos-aarch64 ✅
│   └── windows-x86_64 ✅
│
├── build-wasm (1 platform) 🔧 FIXED
│   └── ubuntu-latest
│       ├── Install Rust (stable + wasm32-unknown-unknown) ✅
│       ├── Cache cargo ✅
│       ├── Verify WASM target ✅ NEW
│       ├── Install wasm-pack (with cache check) ✅ IMPROVED
│       └── Build WASM ✅
│
├── build-tauri (3 platforms) 🔧 FIXED
│   ├── linux (x86_64)
│   │   ├── Install Rust (x86_64-unknown-linux-gnu) ✅
│   │   ├── Install Linux dependencies ✅
│   │   ├── Install Tauri CLI (cached) ✅ IMPROVED
│   │   ├── Build Tauri app ✅
│   │   └── Create artifacts (AppImage, .deb) ✅
│   │
│   ├── macos (universal) 🔧 MAJOR FIX
│   │   ├── Install Rust (universal-apple-darwin) ✅
│   │   ├── Install additional targets ✅ NEW
│   │   │   ├── x86_64-apple-darwin ✅
│   │   │   └── aarch64-apple-darwin ✅
│   │   ├── Install macOS dependencies ✅ NEW
│   │   ├── Install Tauri CLI (cached) ✅ IMPROVED
│   │   ├── Build Tauri app (--target universal-apple-darwin) ✅ FIXED
│   │   └── Create artifacts (DMG, .app.zip) ✅ FIXED PATHS
│   │
│   └── windows (x86_64)
│       ├── Install Rust (x86_64-pc-windows-msvc) ✅
│       ├── Install Tauri CLI (cached) ✅ IMPROVED
│       ├── Build Tauri app ✅
│       └── Create artifacts (MSI, NSIS) ✅
│
├── changelog
│   ├── Get previous tag ✅
│   ├── Generate changelog ✅
│   └── Upload artifact ✅
│
└── release (depends on all above)
    ├── Download all artifacts ✅
    ├── Prepare release assets ✅
    ├── Create GitHub release ✅
    └── Generate summary ✅

Optional:
└── publish-crates (after release, if not pre-release)
    ├── Publish hippo-core
    ├── Publish hippo-cli
    ├── Publish hippo-web
    └── Publish hippo-wasm
```

## macOS Universal Binary Build Flow

```
macOS Tauri Build (FIXED)
│
├── 1. Install Base Rust Toolchain
│   └── target: universal-apple-darwin
│
├── 2. Install Additional Architectures 🆕
│   ├── rustup target add x86_64-apple-darwin
│   └── rustup target add aarch64-apple-darwin
│
├── 3. Install macOS Dependencies 🆕
│   └── xcode-select --install
│
├── 4. Cache Setup
│   └── ~/.cargo, target/
│
├── 5. Install Tauri CLI (with cache check) ✅
│   └── if ! cargo-tauri exists, install it
│
├── 6. Build Universal Binary 🔧
│   └── cargo tauri build --verbose --target universal-apple-darwin
│
└── 7. Prepare Artifacts (fixed paths) 🔧
    ├── Check: target/universal-apple-darwin/release/bundle/
    ├── Fallback: target/release/bundle/
    ├── Copy DMG ✅
    ├── Zip .app bundle ✅
    └── Generate SHA256 checksums ✅
```

## WASM Build Flow

```
WASM Build (FIXED)
│
├── 1. Install Rust Toolchain
│   ├── version: stable
│   └── target: wasm32-unknown-unknown
│
├── 2. Setup Cache 🔧 MOVED UP
│   └── ~/.cargo, target/
│
├── 3. Verify WASM Target 🆕
│   ├── Check installed targets
│   ├── Add wasm32-unknown-unknown if missing
│   ├── Show rustc version
│   └── Show rustup configuration
│
├── 4. Install wasm-pack (with cache) 🔧
│   ├── Check if already installed
│   ├── Install via curl if missing
│   └── Verify installation
│
├── 5. Build WASM Module
│   └── wasm-pack build --target web --release
│
└── 6. Create Artifacts
    ├── tar -czf hippo-wasm.tar.gz
    └── Generate SHA256 checksum
```

## Caching Strategy

```
Caching Hierarchy
│
├── build-cli (per platform)
│   └── key: {os}-{target}-cargo-cli-{Cargo.lock hash}
│
├── build-web (per platform)
│   └── key: {os}-{target}-cargo-web-{Cargo.lock hash}
│
├── build-wasm ✅ IMPROVED
│   ├── key: wasm-cargo-{Cargo.lock hash}
│   └── Includes: wasm-pack binary ✅
│
└── build-tauri (per platform) ✅ IMPROVED
    ├── key: {os}-{target}-cargo-tauri-{Cargo.lock hash}
    └── Includes: tauri-cli binary ✅
```

## Error Handling & Fallbacks

```
macOS Artifact Collection
│
├── Try: target/universal-apple-darwin/release/bundle/
│   └── If fails ↓
│
└── Fallback: target/release/bundle/
    ├── DMG: *.dmg
    ├── App: *.app (zipped)
    └── Checksums: *.sha256
```

```
WASM Target Installation
│
├── Primary: Rust toolchain action with targets
│   └── If fails ↓
│
├── Verification: Check installed targets
│   └── If missing ↓
│
└── Fallback: rustup target add wasm32-unknown-unknown
```

```
Tool Installation (wasm-pack, tauri-cli)
│
├── Check: command -v {tool}
│   └── If exists: ✅ Use cached
│
└── If missing ↓
    ├── Install via standard method
    ├── Verify installation
    └── Cache for next run
```

## Artifact Flow

```
Build Jobs                  Release Job
│                          │
├── CLI Binaries          ─┐
├── Web Binaries          ─┤
├── WASM Module           ─┼─→ Download All
├── Tauri Apps            ─┤   │
└── Changelog             ─┘   │
                               ↓
                          Combine & Package
                               │
                               ├── Create tarballs/zips
                               ├── Generate checksums
                               ├── Create SHA256SUMS.txt
                               │
                               ↓
                          Create GitHub Release
                               │
                               ├── Upload all artifacts
                               ├── Add changelog
                               ├── Set prerelease flag
                               │
                               ↓
                          Generate Summary
                               │
                               └── Job summary with links
```

## Platform Matrix

```
Platform Support Matrix
│
├── CLI
│   ├── Linux   [x86_64 ✅] [aarch64 ✅]
│   ├── macOS   [x86_64 ✅] [aarch64 ✅]
│   └── Windows [x86_64 ✅]
│
├── Web Server
│   ├── Linux   [x86_64 ✅]
│   ├── macOS   [x86_64 ✅] [aarch64 ✅]
│   └── Windows [x86_64 ✅]
│
├── WASM
│   └── Browser [wasm32 ✅] 🔧 FIXED
│
└── Tauri Desktop
    ├── Linux   [x86_64 ✅] (AppImage, .deb)
    ├── macOS   [Universal ✅] 🔧 FIXED (DMG, .app)
    └── Windows [x86_64 ✅] (MSI, NSIS)
```

## Trigger Conditions

```
Workflow Triggers
│
├── On Tag Push (v*)
│   ├── Tag format: v{major}.{minor}.{patch}
│   ├── Examples: v0.2.0, v1.0.0
│   └── Auto-determines prerelease (alpha, beta, rc)
│
└── Manual Dispatch
    ├── Input: tag name
    ├── Example: v0.2.1-test
    └── Use for testing workflow
```

## Key Improvements Summary

| Component | Improvement | Impact |
|-----------|-------------|--------|
| macOS Tauri | Added arch targets | ✅ Universal binary works |
| macOS Tauri | Added macOS deps | ✅ Build dependencies met |
| macOS Tauri | Fixed build command | ✅ Correct target specified |
| macOS Tauri | Fixed artifact paths | ✅ Files found correctly |
| WASM | Target verification | ✅ Ensures target installed |
| WASM | Cache optimization | ✅ Faster subsequent builds |
| WASM | wasm-pack caching | ✅ Reuses cached binary |
| Tauri CLI | Installation caching | ✅ Faster builds |
| All builds | Verbose output | ✅ Better debugging |

## Legend

- ✅ Working / Completed
- 🔧 Fixed in this update
- 🆕 New step added
- ─→ Data flow
- ↓ Fallback chain
