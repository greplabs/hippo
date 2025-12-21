# GitHub Actions Quick Reference

## Common Commands

### View Workflow Status
```bash
# List recent workflow runs
gh run list

# Watch current run
gh run watch

# View specific run
gh run view <run-id>

# View logs
gh run view <run-id> --log
```

### Trigger Workflows Manually
```bash
# Trigger CI
gh workflow run ci.yml

# Trigger release (manual)
gh workflow run release.yml -f tag=v1.0.0
```

### Create a Release
```bash
# 1. Update version in Cargo.toml
# [workspace.package]
# version = "1.0.0"

# 2. Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to 1.0.0"
git tag -a v1.0.0 -m "Release v1.0.0"

# 3. Push tag (triggers release workflow)
git push origin main
git push origin v1.0.0

# 4. Monitor
gh run watch
```

### Manage Caches
```bash
# List caches
gh cache list

# Delete cache
gh cache delete <cache-key>

# Delete all caches
gh cache delete --all
```

## Workflow Overview

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push, PR | Tests, lint, build verification |
| `release.yml` | Tag `v*` | Multi-platform release builds |

## Release Artifacts

When you create a release tag (e.g., `v1.0.0`), you get:

### CLI Binaries (5 variants)
- `hippo-cli-linux-x86_64.tar.gz`
- `hippo-cli-linux-aarch64.tar.gz`
- `hippo-cli-macos-x86_64.tar.gz`
- `hippo-cli-macos-aarch64.tar.gz`
- `hippo-cli-windows-x86_64.exe.zip`

### Web Server (4 variants)
- `hippo-web-linux-x86_64.tar.gz`
- `hippo-web-macos-x86_64.tar.gz`
- `hippo-web-macos-aarch64.tar.gz`
- `hippo-web-windows-x86_64.exe.zip`

### WASM Module
- `hippo-wasm.tar.gz`

### Desktop Installers
- **macOS**: `.dmg`, `.app.zip`
- **Windows**: `.msi`, `-setup.exe`
- **Linux**: `.AppImage`, `.deb`

### Checksums
- Individual `.sha256` files
- Master `SHA256SUMS.txt`

## Commit Message Conventions

For automatic changelog generation:

```bash
# Features (appears in Features section)
git commit -m "feat: add new search feature"
git commit -m "feature: implement cloud sync"

# Bug fixes (appears in Bug Fixes)
git commit -m "fix: resolve crash on startup"
git commit -m "bug: correct file indexing"

# Documentation (appears in Documentation)
git commit -m "docs: update API guide"

# Performance (appears in Performance)
git commit -m "perf: optimize search speed"

# Other
git commit -m "chore: update dependencies"
git commit -m "refactor: simplify code"
```

## Pre-Release Versions

```bash
# Alpha release
git tag -a v1.0.0-alpha.1 -m "Alpha release"

# Beta release
git tag -a v1.0.0-beta.1 -m "Beta release"

# Release candidate
git tag -a v1.0.0-rc.1 -m "Release candidate"

# Push tag
git push origin v1.0.0-alpha.1
```

Pre-releases are automatically marked as "Pre-release" on GitHub.

## Troubleshooting

### Build Failed?
1. Check logs: `gh run view --log`
2. Look for error in specific job
3. Re-run failed jobs: Actions → Re-run failed jobs

### Tag Already Exists?
```bash
# Delete tag
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Recreate
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

### Cache Issues?
```bash
# Clear all caches
gh cache delete --all

# Re-run workflow
gh run rerun <run-id>
```

## Required Secrets (Optional)

| Secret | Purpose | Required? |
|--------|---------|-----------|
| `GITHUB_TOKEN` | Auto-provided | ✅ Yes |
| `CARGO_REGISTRY_TOKEN` | Publish to crates.io | ❌ Optional |

Set at: Settings → Secrets and variables → Actions

## Variables (Optional)

| Variable | Value | Purpose |
|----------|-------|---------|
| `PUBLISH_TO_CRATES` | `true` | Enable crates.io publishing |

Set at: Settings → Variables → Actions

## Typical Timeline

### CI Workflow
- **With cache**: 15-25 minutes
- **Without cache**: 25-35 minutes

### Release Workflow
- **With cache**: 30-45 minutes
- **Without cache**: 60-90 minutes

## Support

- Full guide: [RELEASE_GUIDE.md](RELEASE_GUIDE.md)
- Workflow details: [workflows/README.md](workflows/README.md)
- Implementation: [WORKFLOW_SUMMARY.md](WORKFLOW_SUMMARY.md)

## Quick Checks

### Before Committing
```bash
# Format code
cargo fmt --all

# Check for warnings
cargo clippy --workspace --all-targets --all-features

# Run tests
cargo test --workspace
```

### Before Releasing
```bash
# Verify all tests pass
cargo test --workspace --all-features

# Build release locally
cargo build --release --workspace

# Check version updated
grep "version" Cargo.toml
```

## Platform-Specific Notes

### macOS
- Supports both Intel (x86_64) and Apple Silicon (aarch64)
- DMG installer + App bundle

### Linux
- AppImage: Universal, works on most distros
- .deb: Debian/Ubuntu systems
- Built on Ubuntu latest

### Windows
- MSI: Standard Windows installer
- NSIS exe: Alternative installer
- MSVC toolchain

---

**Last Updated**: 2025-12-21
