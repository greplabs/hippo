# Hippo CLI Testing Guide

Comprehensive testing guide for hippo-cli development.

## Quick Start

```bash
# Run all tests
cargo test --package hippo-cli

# Or use make
make test

# Or use the test script
./run_tests.sh
```

## Test Organization

### Test Files

```
hippo-cli/
├── tests/
│   ├── cli_tests.rs          # Main test suite
│   └── README.md             # Test documentation
├── run_tests.sh              # Test runner script
├── Makefile                  # Build and test commands
└── TESTING.md                # This file
```

### Test Modules

1. **command_parsing_tests** - Unit tests for parsing and formatting
2. **integration_tests** - Integration tests for each command
3. **watch_tests** - File watching functionality
4. **error_tests** - Error handling and edge cases
5. **workflow_tests** - End-to-end workflows
6. **brain_tests** - AI functionality (with/without API key)

## Test Commands

### Using Cargo

```bash
# All tests
cargo test --package hippo-cli

# Specific module
cargo test --package hippo-cli integration_tests

# Specific test
cargo test --package hippo-cli test_chomp_index_folder

# With output
cargo test --package hippo-cli -- --nocapture

# With logging
RUST_LOG=debug cargo test --package hippo-cli -- --nocapture

# Ignored tests (require API key)
cargo test --package hippo-cli -- --ignored
```

### Using Make

```bash
# All tests
make test

# Unit tests only
make test-unit

# Integration tests
make test-integration

# Workflow tests
make test-workflow

# With output
make test-verbose

# AI tests (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=your_key
make test-ai

# Quick subset of tests
make quick

# Specific test
make test-one TEST=test_chomp_index_folder

# All CI checks
make ci
```

### Using Test Script

```bash
# Make executable (first time)
chmod +x run_tests.sh

# Run all tests
./run_tests.sh

# With output
./run_tests.sh --nocapture

# With AI tests
./run_tests.sh --ignored
```

## Test Coverage by Command

### ✅ chomp (index a folder)
- ✅ Index valid folder
- ✅ Index multiple folders
- ✅ Handle non-existent paths
- ✅ Wait for async indexing

**Tests:**
- `test_chomp_index_folder`
- `test_chomp_invalid_path`
- `test_multi_source_workflow`

### ✅ sniff (search)
- ✅ Text search
- ✅ Search with tags
- ✅ Empty results
- ✅ Limit results

**Tests:**
- `test_sniff_search`
- `test_sniff_no_results`
- `test_full_workflow`

### ✅ remember (list memories)
- ✅ List all memories
- ✅ Limit results
- ✅ Empty index

**Tests:**
- `test_remember_list_memories`
- `test_full_workflow`

### ✅ weight (stats)
- ✅ Get statistics
- ✅ Track memory count
- ✅ Empty index stats

**Tests:**
- `test_weight_stats`
- All workflow tests

### ✅ herd (list sources)
- ✅ Empty sources
- ✅ Single source
- ✅ Multiple sources

**Tests:**
- `test_herd_list_sources`
- `test_multi_source_workflow`

### ✅ mark (add tags)
- ✅ Add single tag
- ✅ Add multiple tags
- ✅ Tag filtering
- ✅ Invalid memory ID

**Tests:**
- `test_mark_add_tags`
- `test_mark_invalid_memory`
- `test_tag_filtering_workflow`

### ✅ twins (find duplicates)
- ✅ Detect duplicates
- ✅ Calculate wasted space
- ✅ No duplicates case
- ✅ Minimum size filter

**Tests:**
- `test_twins_find_duplicates`
- `test_twins_no_duplicates`

### ⚠️ brain (AI organize)
- ✅ Structure without API key
- ✅ File selection for tagging
- 🔒 Actual AI analysis (ignored, requires API key)

**Tests:**
- `test_brain_without_api_key`
- `test_brain_with_api_key` (ignored)

### ✅ splash (reindex)
- ✅ Reindex existing source
- ✅ Maintain memory count

**Tests:**
- `test_splash_reindex`

### ✅ stomp (remove source)
- ✅ Remove source
- ✅ Delete memories
- ✅ Keep memories option

**Tests:**
- `test_stomp_remove_source`

### ⚠️ yawn (open file)
- ⚠️ Platform-specific (limited testing)
- ✅ File search before open

**Tests:**
- Tested indirectly in search tests

### ⚠️ wade (watch for changes)
- ⚠️ Async, may be flaky
- ✅ Start watching
- ✅ Stop watching
- ⚠️ Detect new files (timing-dependent)

**Tests:**
- `test_wade_watch_for_changes`

### ✅ den (show config)
- ✅ Access config directories
- ✅ Verify data paths

**Tests:**
- `test_den_show_config`

### ✅ forget (reset index)
- ✅ Clear all data
- ✅ Reset sources
- ✅ Verify empty state

**Tests:**
- `test_forget_reset_index`

## Test Helpers

### create_test_hippo()
Creates an isolated Hippo instance with temp data directory.

```rust
let (hippo, temp_dir) = helpers::create_test_hippo().await?;
```

### create_test_files(temp_dir)
Creates a directory tree with various file types.

```rust
let test_path = helpers::create_test_files(&temp_dir)?;
```

Files created:
- `document.txt` - Text file with "vacation" content
- `readme.md` - Markdown file
- `main.rs` - Rust source file
- `script.py` - Python script
- `app.js` - JavaScript file
- `config.json` - JSON configuration
- `images/photo1.txt` - Simulated image (text)
- `images/photo2.txt` - Simulated image (text)

### create_duplicate_files(temp_dir)
Creates files with duplicate content.

```rust
let test_path = helpers::create_duplicate_files(&temp_dir)?;
```

Files created:
- `original.txt` - Original content
- `copy1.txt` - Duplicate
- `copy2.txt` - Duplicate
- `unique.txt` - Unique content

### wait_for_indexing(hippo, count, timeout)
Waits for async indexing to complete.

```rust
helpers::wait_for_indexing(&hippo, 5, 10).await?;
```

## Writing New Tests

### Template: Integration Test

```rust
#[tokio::test]
async fn test_my_new_feature() -> Result<()> {
    // Setup
    let (hippo, temp_dir) = helpers::create_test_hippo().await?;
    let test_path = helpers::create_test_files(&temp_dir)?;

    // Index files
    let source = Source::Local { root_path: test_path };
    hippo.add_source(source).await?;
    helpers::wait_for_indexing(&hippo, 1, 10).await?;

    // Test your feature
    // ...

    // Verify
    assert!(/* your assertion */);

    Ok(())
}
```

### Template: Unit Test

```rust
#[test]
fn test_helper_function() {
    let result = my_function(input);
    assert_eq!(result, expected);
}
```

### Template: Workflow Test

```rust
#[tokio::test]
async fn test_complete_workflow() -> Result<()> {
    let (hippo, temp_dir) = helpers::create_test_hippo().await?;

    // 1. Setup
    // 2. Execute multiple operations
    // 3. Verify each step
    // 4. Verify final state

    Ok(())
}
```

## CI/CD Integration

### GitHub Actions

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests
- Changes to `hippo-cli/` or `hippo-core/`

Workflow: `.github/workflows/cli-tests.yml`

Matrix:
- OS: Ubuntu, macOS, Windows
- Rust: stable

### Local CI Simulation

```bash
# Run all CI checks
make ci

# Individual checks
make fmt-check
make clippy
make build
make test
```

## Coverage

Generate coverage report:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate HTML report
make coverage

# View report
open coverage/index.html
```

## Debugging Tests

### Enable Logging

```bash
RUST_LOG=debug cargo test --package hippo-cli test_name -- --nocapture
```

### Debug Specific Test

```bash
cargo test --package hippo-cli test_name -- --nocapture
```

### Inspect Temp Files

Tests use `tempfile` which auto-cleans. To inspect files:

1. Add a sleep before test ends:
```rust
tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
```

2. Check `temp_dir.path()` location in logs

### Common Issues

**Issue: Tests timeout**
- Increase timeout in `wait_for_indexing`
- Check if indexing is stuck
- Verify no deadlocks

**Issue: Flaky watch tests**
- File watching is async and platform-dependent
- Tests are lenient for this reason
- Use `continue-on-error` in CI

**Issue: Permission errors**
- Check temp directory permissions
- Verify database can be created
- On Windows, check file locks

## Performance

Test suite performance:
- **Unit tests**: < 1 second
- **Integration tests**: 30-60 seconds
- **Workflow tests**: 30-60 seconds
- **Total**: < 2 minutes

Optimizations:
- Disabled embeddings (slow)
- Small test files
- Limited parallelism
- Reasonable timeouts

## Best Practices

1. **Isolation**: Each test gets fresh Hippo instance and temp directory
2. **Cleanup**: Use `tempfile` for automatic cleanup
3. **Async**: Use `#[tokio::test]` for async tests
4. **Timeouts**: Use reasonable timeouts in `wait_for_indexing`
5. **Assertions**: Clear, descriptive assertion messages
6. **Organization**: Group related tests in modules
7. **Documentation**: Document what each test verifies
8. **Leniency**: Be lenient with timing-dependent tests

## Troubleshooting

### Tests fail on Windows
- Check path separators
- Verify file permissions
- Watch tests may be more flaky

### Tests fail on macOS
- Check if APFS affects file watching
- Verify temp directory permissions

### Tests fail in CI
- Check timeout values
- Verify no interactive prompts
- Check resource constraints

### AI tests fail
- Verify `ANTHROPIC_API_KEY` is set
- Check API key is valid
- Network connectivity required

## Contributing

When adding new CLI features:

1. Write tests first (TDD)
2. Add unit tests for logic
3. Add integration test for command
4. Add workflow test if applicable
5. Update this documentation
6. Verify CI passes
7. Check coverage

## Resources

- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [tempfile crate](https://docs.rs/tempfile/)
- [GitHub Actions](https://docs.github.com/en/actions)
