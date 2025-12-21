# Release Guide for Hippo

This guide explains how to create releases for the Hippo project using the automated GitHub Actions workflows.

## Release Components

Each Hippo release includes multiple components:

1. **hippo-cli** - Command-line interface
2. **hippo-web** - REST API server
3. **hippo-wasm** - WebAssembly module
4. **hippo-tauri** - Desktop application (macOS, Linux, Windows)

## Pre-Release Checklist

- [ ] All tests passing on main branch
- [ ] Documentation updated
- [ ] CHANGELOG.md reviewed (or will be auto-generated)
- [ ] Version numbers updated in Cargo.toml

## Step-by-Step Release Process

### 1. Update Version Number

Edit the workspace version in `/Cargo.toml`:

```toml
[workspace.package]
version = "1.0.0"  # Update this
edition = "2021"
authors = ["Hippo Contributors"]
license = "MIT"
```

All workspace members inherit this version, so you only need to update it once.

### 2. Commit Version Bump

```bash
git add Cargo.toml
git commit -m "chore: bump version to 1.0.0"
git push origin main
```

### 3. Create and Push Tag

```bash
# Create annotated tag
git tag -a v1.0.0 -m "Release v1.0.0"

# Push tag to trigger release workflow
git push origin v1.0.0
```

**Important**: The tag MUST start with `v` (e.g., `v1.0.0`, `v2.1.3-beta.1`)

### 4. Monitor Workflow

The release workflow will automatically:

1. Build CLI binaries for all platforms (5 variants)
2. Build web server binaries (4 variants)
3. Build WASM module
4. Build Tauri desktop apps (.dmg, .msi, .AppImage, .deb)
5. Generate changelog from git commits
6. Create GitHub release with all artifacts
7. Optionally publish to crates.io

Monitor progress:

```bash
# Watch workflow in real-time
gh run watch

# Or view in browser
# https://github.com/YOUR_ORG/hippov20/actions
```

### 5. Verify Release

Once the workflow completes (30-45 minutes):

```bash
# View release
gh release view v1.0.0

# Or visit in browser
# https://github.com/YOUR_ORG/hippov20/releases/tag/v1.0.0
```

## Release Artifacts

Each release includes:

### CLI Binaries (Compressed Archives)
- `hippo-cli-linux-x86_64.tar.gz` (+ .sha256)
- `hippo-cli-linux-aarch64.tar.gz` (+ .sha256)
- `hippo-cli-macos-x86_64.tar.gz` (+ .sha256)
- `hippo-cli-macos-aarch64.tar.gz` (+ .sha256)
- `hippo-cli-windows-x86_64.exe.zip` (+ .sha256)

### Web Server Binaries
- `hippo-web-linux-x86_64.tar.gz` (+ .sha256)
- `hippo-web-macos-x86_64.tar.gz` (+ .sha256)
- `hippo-web-macos-aarch64.tar.gz` (+ .sha256)
- `hippo-web-windows-x86_64.exe.zip` (+ .sha256)

### WASM Module
- `hippo-wasm.tar.gz` (+ .sha256)
  - Contains: hippo_wasm.js, hippo_wasm_bg.wasm, package.json, etc.

### Desktop Applications
- **macOS**: `Hippo_1.0.0_x64.dmg`, `Hippo-macos.app.zip`
- **Windows**: `Hippo_1.0.0_x64_en-US.msi`, `Hippo_1.0.0_x64-setup.exe`
- **Linux**: `hippo_1.0.0_amd64.AppImage`, `hippo_1.0.0_amd64.deb`

### Checksums
- `SHA256SUMS.txt` - Master checksum file for all artifacts
- Individual `.sha256` files for each artifact

## Version Naming Conventions

Follow [Semantic Versioning](https://semver.org/):

- **Major version** (X.0.0): Breaking changes
- **Minor version** (1.X.0): New features, backwards compatible
- **Patch version** (1.0.X): Bug fixes, backwards compatible

### Pre-release Versions

For pre-releases, append identifiers:

- **Alpha**: `v1.0.0-alpha.1`
- **Beta**: `v1.0.0-beta.1`
- **Release Candidate**: `v1.0.0-rc.1`

Pre-releases are automatically marked in GitHub releases.

## Changelog Generation

The workflow automatically generates changelogs from git commits.

### Commit Message Conventions

Use conventional commits for automatic categorization:

```bash
# Features
git commit -m "feat: add semantic search"
git commit -m "feature: add cloud sync"

# Bug fixes
git commit -m "fix: resolve thumbnail generation issue"
git commit -m "bug: correct metadata extraction"

# Documentation
git commit -m "docs: update API documentation"

# Performance
git commit -m "perf: optimize indexing speed"

# Other (will appear in "Other Changes")
git commit -m "refactor: simplify code structure"
git commit -m "chore: update dependencies"
```

## Publishing to crates.io (Optional)

To enable automatic publishing to crates.io:

### 1. Create API Token

1. Visit https://crates.io/settings/tokens
2. Create new token with "publish" scope
3. Copy token

### 2. Add Secret to GitHub

1. Go to repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `CARGO_REGISTRY_TOKEN`
4. Value: Your crates.io token

### 3. Enable Publishing

1. Go to Settings → Variables
2. Create variable: `PUBLISH_TO_CRATES` = `true`

Now releases will automatically publish to crates.io (excluding pre-releases).

## Manual Release (Alternative)

You can also trigger releases manually without a tag:

```bash
gh workflow run release.yml -f tag=v1.0.0
```

Or via GitHub UI:
1. Actions → Release workflow
2. Click "Run workflow"
3. Enter tag (e.g., `v1.0.0`)

## Troubleshooting

### Build Failures

**Q: Build failed on specific platform**
- Check workflow logs for that platform
- Common issues: missing dependencies, system library version mismatches
- Re-run the specific job from GitHub Actions UI

**Q: Tauri build failed**
- Ensure `tauri.conf.json` version matches tag
- Check icon files exist in `hippo-tauri/icons/`
- Verify all UI assets are present in `hippo-tauri/ui/dist/`

**Q: WASM build failed**
- Verify `hippo-wasm` compiles locally with `wasm-pack build`
- Check for WASM-incompatible dependencies

### Tag Issues

**Q: Tag already exists**
```bash
# Delete local and remote tag
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Recreate
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

**Q: Wrong version tagged**
```bash
# Create new patch version
git tag -a v1.0.1 -m "Release v1.0.1 (fix)"
git push origin v1.0.1
```

### Release Artifacts Missing

**Q: Some artifacts missing from release**
- Check if specific build job failed
- Verify artifact upload step completed
- Re-run failed jobs from Actions UI

## Hotfix Releases

For urgent fixes:

```bash
# Create hotfix branch from tag
git checkout -b hotfix/1.0.1 v1.0.0

# Make fixes
git commit -m "fix: critical security patch"

# Update version
# Edit Cargo.toml: version = "1.0.1"
git commit -m "chore: bump version to 1.0.1"

# Merge to main
git checkout main
git merge --no-ff hotfix/1.0.1

# Tag and release
git tag -a v1.0.1 -m "Hotfix v1.0.1"
git push origin main
git push origin v1.0.1
```

## Release Checklist Template

Copy this for each release:

```markdown
## Release v1.0.0 Checklist

### Pre-Release
- [ ] All tests passing
- [ ] Version updated in Cargo.toml
- [ ] Documentation reviewed
- [ ] Breaking changes documented
- [ ] Migration guide written (if needed)

### Release
- [ ] Version committed and pushed
- [ ] Tag created: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Tag pushed: `git push origin v1.0.0`
- [ ] Workflow started successfully

### Post-Release
- [ ] All builds completed (5 CLI + 4 Web + 1 WASM + 3 Tauri)
- [ ] GitHub release created
- [ ] All artifacts present (verify checksums)
- [ ] Changelog accurate
- [ ] crates.io published (if enabled)
- [ ] Release announcement prepared

### Verification
- [ ] Download and test CLI on each platform
- [ ] Test desktop installers
- [ ] Verify WASM module loads
- [ ] Test web server binary
```

## Communication

After release:

1. **GitHub Release**: Automatically created with changelog
2. **Announcement**: Post in discussions or create announcement issue
3. **Social Media**: Share release highlights
4. **Documentation**: Update version in docs
5. **Users**: Notify via email/newsletter if applicable

## Support

For issues with the release process:

1. Check workflow logs in Actions tab
2. Review this guide
3. Check [Workflow README](.github/workflows/README.md)
4. Create issue with `ci` label

---

**Last Updated**: 2025-12-21
**Workflow Version**: 1.0.0
