# Makefile for hippo-cli testing and development

.PHONY: help test test-unit test-integration test-all test-verbose test-ai clean build check fmt clippy

# Default target
help:
	@echo "🦛 Hippo CLI - Development Commands"
	@echo "===================================="
	@echo ""
	@echo "Testing:"
	@echo "  make test              Run all tests (excluding ignored)"
	@echo "  make test-unit         Run only unit tests"
	@echo "  make test-integration  Run only integration tests"
	@echo "  make test-workflow     Run only workflow tests"
	@echo "  make test-verbose      Run all tests with output"
	@echo "  make test-ai           Run AI tests (requires ANTHROPIC_API_KEY)"
	@echo "  make test-all          Run all tests including ignored"
	@echo ""
	@echo "Development:"
	@echo "  make build             Build the CLI"
	@echo "  make check             Check code without building"
	@echo "  make fmt               Format code"
	@echo "  make clippy            Run clippy linter"
	@echo "  make clean             Clean build artifacts"
	@echo ""
	@echo "CI/CD:"
	@echo "  make ci                Run all CI checks"

# Run all tests (excluding ignored)
test:
	@echo "🧪 Running all tests..."
	cargo test --package hippo-cli

# Run unit tests only
test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --package hippo-cli command_parsing_tests

# Run integration tests only
test-integration:
	@echo "🧪 Running integration tests..."
	cargo test --package hippo-cli integration_tests
	cargo test --package hippo-cli watch_tests
	cargo test --package hippo-cli error_tests

# Run workflow tests only
test-workflow:
	@echo "🧪 Running workflow tests..."
	cargo test --package hippo-cli workflow_tests

# Run tests with output
test-verbose:
	@echo "🧪 Running all tests with output..."
	cargo test --package hippo-cli -- --nocapture

# Run AI tests (requires API key)
test-ai:
	@echo "🧪 Running AI tests..."
	@if [ -z "$$ANTHROPIC_API_KEY" ]; then \
		echo "❌ Error: ANTHROPIC_API_KEY environment variable not set"; \
		exit 1; \
	fi
	cargo test --package hippo-cli -- --ignored --nocapture

# Run all tests including ignored
test-all: test test-ai

# Build the CLI
build:
	@echo "🔨 Building hippo-cli..."
	cargo build --package hippo-cli

# Check code without building
check:
	@echo "🔍 Checking code..."
	cargo check --package hippo-cli --all-features

# Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt --package hippo-cli

# Check formatting
fmt-check:
	@echo "🔍 Checking formatting..."
	cargo fmt --package hippo-cli -- --check

# Run clippy
clippy:
	@echo "📎 Running clippy..."
	cargo clippy --package hippo-cli -- -D warnings

# Clean build artifacts
clean:
	@echo "🧹 Cleaning..."
	cargo clean

# Run all CI checks
ci: fmt-check clippy build test
	@echo "✅ All CI checks passed!"

# Watch for changes and run tests
watch:
	@echo "👀 Watching for changes..."
	cargo watch -x "test --package hippo-cli"

# Run specific test
test-one:
	@echo "🧪 Running specific test..."
	@if [ -z "$(TEST)" ]; then \
		echo "Usage: make test-one TEST=test_name"; \
		exit 1; \
	fi
	cargo test --package hippo-cli $(TEST) -- --nocapture

# Generate test coverage
coverage:
	@echo "📊 Generating coverage report..."
	cargo tarpaulin --package hippo-cli --out html --output-dir coverage

# Install development dependencies
dev-deps:
	@echo "📦 Installing development dependencies..."
	cargo install cargo-watch
	cargo install cargo-tarpaulin

# Quick test (faster subset)
quick:
	@echo "⚡ Running quick tests..."
	cargo test --package hippo-cli command_parsing_tests integration_tests
