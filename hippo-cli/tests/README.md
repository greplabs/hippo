# Hippo CLI Test Suite

Comprehensive test suite for the hippo-cli commands.

## Test Coverage

### Command Coverage

All CLI commands are tested:

1. **chomp** - Index a folder
2. **sniff** - Search memories
3. **remember** - List memories
4. **weight** - Show statistics
5. **herd** - List sources
6. **mark** - Add tags
7. **twins** - Find duplicates
8. **brain** - AI organize (mocked, requires API key)
9. **splash** - Refresh/reindex
10. **stomp** - Remove source
11. **yawn** - Open file (limited testing)
12. **wade** - Watch for changes
13. **den** - Show config
14. **forget** - Reset index

### Test Types

#### Unit Tests
- Command parsing validation
- Helper function tests (`format_bytes`, `get_kind_string`)

#### Integration Tests
- Full command workflows
- Multi-source operations
- Tag filtering
- Error handling

#### Workflow Tests
- Complete end-to-end scenarios
- Multi-source workflows
- Tag filtering workflows

### Running Tests

```bash
# Run all tests
cargo test --package hippo-cli

# Run specific test module
cargo test --package hippo-cli command_parsing_tests
cargo test --package hippo-cli integration_tests
cargo test --package hippo-cli workflow_tests

# Run with output
cargo test --package hippo-cli -- --nocapture

# Run ignored tests (requires API key)
cargo test --package hippo-cli -- --ignored
```

### Test Requirements

- **tempfile**: For creating temporary test directories
- **tokio**: For async test runtime
- **hippo-core**: Core library being tested

### Test Structure

```
tests/
├── cli_tests.rs          # Main test suite
└── README.md             # This file
```

Each test:
1. Creates a temporary Hippo instance with isolated data directory
2. Creates test files in a temporary directory
3. Executes CLI operations via the Hippo API
4. Verifies expected outcomes
5. Cleans up automatically (tempfile handles cleanup)

### Test Files

The test suite creates various test files:

- **Text files**: `.txt`, `.md`
- **Code files**: `.rs`, `.py`, `.js`
- **Data files**: `.json`
- **Subdirectories**: Nested file structures
- **Duplicates**: Identical files for duplicate detection

### AI Tests

Tests involving the `brain` command (AI tagging) are:

1. **Default**: Test structure without API calls
2. **Ignored**: Actual API tests (require `ANTHROPIC_API_KEY`)

To run AI tests:

```bash
export ANTHROPIC_API_KEY=your_key_here
cargo test --package hippo-cli -- --ignored
```

### Error Handling Tests

Tests cover:
- Invalid paths
- Non-existent files
- Empty search results
- Invalid memory IDs
- Missing API keys

### Known Limitations

1. **File Watching**: Tests are lenient due to async nature and platform differences
2. **AI Tests**: Require valid API key and are ignored by default
3. **Platform-specific**: Some features (like `yawn`) are platform-dependent

### Continuous Integration

Tests are designed to run in CI environments:

- No interactive prompts
- Isolated temporary directories
- Skip tests requiring API keys by default
- Reasonable timeouts

### Test Helpers

The `helpers` module provides:

- `create_test_hippo()`: Create isolated test instance
- `create_test_files()`: Generate test file hierarchy
- `create_duplicate_files()`: Generate duplicate test files
- `wait_for_indexing()`: Wait for async indexing to complete

### Adding New Tests

When adding new CLI commands or features:

1. Add unit tests for parsing/validation
2. Add integration test for the command
3. Add workflow test if it interacts with other commands
4. Add error handling test for edge cases
5. Update this README

Example:

```rust
#[tokio::test]
async fn test_new_command() -> Result<()> {
    let (hippo, temp_dir) = helpers::create_test_hippo().await?;
    let test_path = helpers::create_test_files(&temp_dir)?;

    // Setup
    let source = Source::Local { root_path: test_path };
    hippo.add_source(source).await?;
    helpers::wait_for_indexing(&hippo, 1, 10).await?;

    // Test the command
    // ... your test logic here ...

    // Verify
    assert!(/* your assertion */);

    Ok(())
}
```

### Debugging Tests

To debug a specific test:

```bash
# Run with output
cargo test --package hippo-cli test_name -- --nocapture

# Run with logging
RUST_LOG=debug cargo test --package hippo-cli test_name

# Run single test
cargo test --package hippo-cli test_name --test cli_tests
```

### Performance Considerations

Tests use:
- Minimal parallelism (2 threads) for indexing
- Small test files
- Reasonable timeouts (10-15 seconds)
- No embeddings (disabled for speed)

Total test suite should complete in under 2 minutes on most systems.
