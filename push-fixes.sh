#!/bin/bash

# Script to create branch and push workflow fixes

echo "Creating new branch fix/release-workflow..."
git checkout -b fix/release-workflow

echo "Staging changes..."
git add .github/workflows/release.yml
git add .github/workflows/release-backup.yml
git add RELEASE_WORKFLOW_FIXES.md

echo "Creating commit..."
git commit -m "Fix GitHub Actions release workflow issues

- Fix macOS Tauri build by adding universal binary support
  * Add x86_64-apple-darwin and aarch64-apple-darwin targets
  * Update artifact paths for universal-apple-darwin builds
  * Add macOS dependency installation step
  * Explicitly specify target in Tauri build command

- Fix WASM build by ensuring target installation
  * Add verification step for wasm32-unknown-unknown target
  * Improve wasm-pack installation with cache check
  * Move cargo cache before tool installation

- Improve Tauri CLI installation with caching
  * Check if cargo-tauri exists before installing
  * Reduces build time on cache hit

- Add better error handling and debugging
  * Add --verbose flag to Tauri builds
  * Add file existence checks in artifact preparation
  * Improve path handling for universal builds

Fixes build failures in GitHub Actions release workflow"

echo "Pushing to remote..."
git push -u origin fix/release-workflow

echo "Done! Branch fix/release-workflow created and pushed."
echo ""
echo "Next steps:"
echo "1. Go to GitHub and create a pull request"
echo "2. Review the changes in RELEASE_WORKFLOW_FIXES.md"
echo "3. Test the workflow with a test tag release"
