# GitHub Actions Workflows

## Quick Start

```bash
# The workflows are already configured and will run automatically!
# Just push code or create a tag to trigger them.

# Create a release
git tag v1.0.0
git push origin v1.0.0

# Manual trigger
gh workflow run ci.yml
```

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    HIPPO CI/CD PIPELINE                     │
└─────────────────────────────────────────────────────────────┘

┌──────────────────┐
│  Push / PR       │
└────────┬─────────┘
         │
         ├─────────► ci.yml ──────────────► Check, Format, Lint, Test, Build
         │
         ├─────────► test.yml ────────────► Comprehensive Testing
         │                                   • Multiple Rust versions
         │                                   • Integration tests
         │                                   • Code coverage
         │                                   • Memory checks
         │
         ├─────────► docs.yml ─────────────► Documentation
         │                                   • Rustdoc
         │                                   • Link checks
         │
         ├─────────► performance.yml ──────► Benchmarks
         │                                   • Binary size
         │                                   • Compile time
         │
         └─────────► claude-review.yml ────► AI Code Review (PRs only)

┌──────────────────┐
│  Tag (v*)        │
└────────┬─────────┘
         │
         └─────────► release.yml ──────────► GitHub Release
                                            • Multi-platform binaries
                                            • Auto changelog
                                            • Checksums
                                            • crates.io publish

┌──────────────────┐
│  Schedule Daily  │
└────────┬─────────┘
         │
         ├─────────► nightly.yml ──────────► Nightly Builds
         │                                   • Rust nightly
         │                                   • Pre-release
         │
         └─────────► test.yml ─────────────► Daily Test Run

┌──────────────────┐
│  Schedule Weekly │
└────────┬─────────┘
         │
         └─────────► dependencies.yml ─────► Dependency Updates
                                            • Security audit
                                            • License check
                                            • Auto PR

┌──────────────────┐
│  Any Trigger     │
└────────┬─────────┘
         │
         └─────────► docker.yml ───────────► Container Images
                                            • Multi-platform
                                            • Security scan
                                            • Registry push
```

## Workflow Files

| File | Trigger | Purpose | Duration |
|------|---------|---------|----------|
| `ci.yml` | Push, PR | Main CI pipeline with comprehensive testing | ~15-25 min |
| `release.yml` | Tag (v*) | Multi-platform release builds | ~30-45 min |
| `dependencies.yml` | Weekly, Manual | Dependency management | ~5-10 min |
| `docs.yml` | Push (docs), PR | Documentation | ~5-10 min |
| `docker.yml` | Push, Tag, PR | Container builds | ~10-15 min |
| `claude-review.yml` | PR | AI code review | ~2-5 min |
| `cli-tests.yml` | Push, PR | CLI testing | ~10-15 min |
| `claude.yml` | Manual | Claude Code integration | ~5 min |

## Status Badges

Add these to your README.md:

```markdown
![CI](https://github.com/OWNER/REPO/actions/workflows/ci.yml/badge.svg)
![Tests](https://github.com/OWNER/REPO/actions/workflows/test.yml/badge.svg)
![Release](https://github.com/OWNER/REPO/actions/workflows/release.yml/badge.svg)
[![codecov](https://codecov.io/gh/OWNER/REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/OWNER/REPO)
```

## Platform Support

### Build Platforms
- ✅ Linux (x86_64)
- ✅ macOS (x86_64, ARM64)
- ✅ Windows (x86_64)

### Test Platforms
- ✅ Ubuntu Latest
- ✅ macOS Latest
- ✅ Windows Latest

### Docker Platforms
- ✅ linux/amd64
- ✅ linux/arm64

## Required Secrets

### Automatic (No Setup)
- `GITHUB_TOKEN` - Provided by GitHub

### Optional (Enhanced Features)
- `CODECOV_TOKEN` - Code coverage reports
- `ANTHROPIC_API_KEY` - AI code reviews
- `CARGO_REGISTRY_TOKEN` - Publish to crates.io

**Setup**: Settings → Secrets and variables → Actions → New repository secret

## Common Tasks

### Run Workflows Manually

```bash
# CI
gh workflow run ci.yml

# Tests
gh workflow run test.yml

# Nightly build
gh workflow run nightly.yml

# Update dependencies
gh workflow run dependencies.yml
```

### Create a Release

```bash
# 1. Update version in Cargo.toml
# Edit the workspace version:
# [workspace.package]
# version = "1.0.0"

# 2. Commit changes
git add Cargo.toml
git commit -m "chore: bump version to 1.0.0"
git push

# 3. Tag and push
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 4. Wait for workflow (builds all components)
gh run watch

# 5. Verify release artifacts
gh release view v1.0.0
```

The release workflow will build:
- **CLI binaries** for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- **Web server** for all platforms
- **WASM module** (hippo-wasm.tar.gz)
- **Tauri desktop apps**: .dmg (macOS), .msi/.exe (Windows), .AppImage/.deb (Linux)

### View Workflow Status

```bash
# List recent runs
gh run list

# View specific run
gh run view <run-id>

# Watch running workflow
gh run watch

# View logs
gh run view --log
```

### Local Testing with `act`

```bash
# Install act
brew install act  # macOS
# or: https://github.com/nektos/act

# Setup secrets
cp .secrets.example .secrets
# Edit .secrets

# Run CI locally
act push

# Run specific job
act -j test

# List workflows
act -l
```

## Artifacts

### CI Artifacts (30 days)
- Linux binaries
- macOS binaries (x86_64, ARM64)
- Windows binaries
- Code coverage reports

### Release Artifacts (Permanent)
- **CLI binaries**: hippo-cli-{linux,macos,windows}-{x86_64,aarch64}.{tar.gz,zip}
- **Web server**: hippo-web-{platform}.{tar.gz,zip}
- **WASM module**: hippo-wasm.tar.gz
- **Desktop installers**: .dmg, .msi, .exe, .AppImage, .deb
- **SHA256 checksums**: Individual .sha256 files + master SHA256SUMS.txt
- **Auto-generated changelog** from git commits

### Nightly Artifacts (7 days)
- Pre-release binaries
- Auto-cleanup after 7 days

## Caching Strategy

All workflows cache:
- Cargo registry index
- Cargo registry cache
- Cargo git database
- Build artifacts (`target/`)

**Cache hit rate**: ~80-90% for subsequent runs
**Time saved**: ~60-70% on cached runs

## Monitoring

### GitHub Actions Tab
- View all workflow runs
- Check job status
- Download artifacts
- Re-run failed jobs

### Codecov Dashboard
- Code coverage trends
- File-level coverage
- PR coverage diff

### GitHub Pages (if enabled)
- API documentation (rustdoc)
- Benchmark results
- Performance graphs

## Troubleshooting

### Workflow Fails

1. Check workflow logs in Actions tab
2. Look for error messages in failed jobs
3. Review recent changes that might cause issues
4. Re-run workflow if transient failure

### Cache Issues

```bash
# List caches
gh cache list

# Delete specific cache
gh cache delete <cache-key>

# Delete all caches
gh cache delete --all
```

### Permission Errors

1. Settings → Actions → General
2. Workflow permissions → Read and write
3. Allow GitHub Actions to create PRs

### Build Failures

- **Linux**: Check system dependencies
- **macOS**: Verify Xcode Command Line Tools
- **Windows**: Check MSVC installation

## Documentation

- 📖 [Complete Workflow Documentation](../WORKFLOWS.md)
- 🚀 [Setup Guide](../CI_SETUP.md)
- 📊 [Implementation Summary](../CICD_SUMMARY.md)

## Best Practices

1. ✅ Review workflow logs regularly
2. ✅ Keep dependencies updated (automated weekly)
3. ✅ Monitor security advisories
4. ✅ Test locally before pushing when possible
5. ✅ Use semantic versioning for releases
6. ✅ Keep workflows updated with latest action versions

## Performance

### Typical Run Times (with cache)
- **CI**: 5-10 minutes
- **Tests**: 10-15 minutes
- **Release**: 20-30 minutes
- **Docs**: 3-5 minutes

### Resource Usage
- **Concurrent jobs**: Up to 20 (default limit)
- **Total minutes**: ~100-150 per workflow run
- **Storage**: ~5-10 GB for artifacts

## Support

### Issues
- Tag issues with `ci` label for CI/CD problems
- Include workflow run URL in issue description
- Attach relevant logs

### Contributions
- Test workflow changes with `act` before PR
- Update documentation when adding workflows
- Follow existing workflow patterns

## Changelog

See [WORKFLOWS.md](../WORKFLOWS.md) for detailed workflow documentation and changelog.

---

**Last Updated**: 2025-12-20
**Workflow Version**: 1.0.0
**Maintained by**: Hippo Contributors
