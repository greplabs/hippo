# Quick Test Reference Card

One-page reference for hippo-cli testing.

## 🚀 Quick Start

```bash
# Run all tests
cargo test --package hippo-cli

# Or
make test
```

## 📝 Common Commands

```bash
# All tests
cargo test --package hippo-cli

# With output
cargo test --package hippo-cli -- --nocapture

# Specific module
cargo test --package hippo-cli integration_tests

# Specific test
cargo test --package hippo-cli test_chomp_index_folder

# With logging
RUST_LOG=debug cargo test --package hippo-cli -- --nocapture

# Ignored tests (needs API key)
ANTHROPIC_API_KEY=xxx cargo test --package hippo-cli -- --ignored
```

## 🎯 Make Targets

```bash
make test              # All tests
make test-unit         # Unit tests only
make test-integration  # Integration tests
make test-workflow     # Workflow tests
make test-verbose      # With output
make test-ai          # AI tests (needs key)
make quick            # Fast subset
make ci               # All CI checks
```

## 📦 Test Modules

| Module | Tests | Purpose |
|--------|-------|---------|
| `command_parsing_tests` | 3 | Helper functions |
| `integration_tests` | 10 | Command testing |
| `watch_tests` | 2 | File watching |
| `error_tests` | 4 | Error handling |
| `workflow_tests` | 3 | End-to-end |
| `brain_tests` | 2 | AI features |

## 🔧 Test Helpers

```rust
// Create test instance
let (hippo, temp_dir) = helpers::create_test_hippo().await?;

// Create test files
let test_path = helpers::create_test_files(&temp_dir)?;

// Create duplicates
let dup_path = helpers::create_duplicate_files(&temp_dir)?;

// Wait for indexing
helpers::wait_for_indexing(&hippo, 5, 10).await?;
```

## ✅ Command Coverage

| Command | Status | Key Tests |
|---------|--------|-----------|
| chomp | ✅ | index, invalid path |
| sniff | ✅ | search, no results |
| remember | ✅ | list, limits |
| weight | ✅ | stats |
| herd | ✅ | sources |
| mark | ✅ | tags, filtering |
| twins | ✅ | duplicates |
| brain | ⚠️ | needs API key |
| splash | ✅ | reindex |
| stomp | ✅ | remove |
| yawn | ⚠️ | platform specific |
| wade | ⚠️ | async/timing |
| den | ✅ | config |
| forget | ✅ | reset |

## 🐛 Debugging

```bash
# Debug single test
cargo test --package hippo-cli test_name -- --nocapture

# With logging
RUST_LOG=debug cargo test --package hippo-cli test_name -- --nocapture

# Using make
make test-one TEST=test_name

# Check test code
vim tests/cli_tests.rs
```

## 📊 Quick Stats

- **Total Tests**: 25+
- **Coverage**: 14/14 commands (100%)
- **Runtime**: < 2 minutes
- **Files**: 1 test file, 795 lines

## 🎨 Test Template

```rust
#[tokio::test]
async fn test_my_feature() -> Result<()> {
    // Setup
    let (hippo, temp_dir) = helpers::create_test_hippo().await?;
    let test_path = helpers::create_test_files(&temp_dir)?;

    // Index
    let source = Source::Local { root_path: test_path };
    hippo.add_source(source).await?;
    helpers::wait_for_indexing(&hippo, 1, 10).await?;

    // Test
    // ... your test code ...

    // Verify
    assert!(/* condition */);

    Ok(())
}
```

## 🔄 CI/CD

```yaml
# GitHub Actions runs on:
- Push to main/develop
- Pull requests
- Changes to hippo-cli/ or hippo-core/

# Matrix:
- Ubuntu, macOS, Windows
- Rust stable
```

## 📚 Documentation

- `tests/README.md` - Quick reference
- `TESTING.md` - Full guide
- `TEST_SUITE_SUMMARY.md` - Overview
- `tests/test_structure.md` - Diagrams

## ⚡ Performance Tips

- Tests use temp directories (fast cleanup)
- Embeddings disabled (speed)
- Parallel execution safe
- Reasonable timeouts

## 🎯 Best Practices

1. ✅ Each test isolated
2. ✅ Auto cleanup
3. ✅ Clear assertions
4. ✅ Async with tokio::test
5. ✅ Lenient for timing
6. ✅ Document edge cases

## 🚨 Common Issues

**Tests timeout?**
- Increase timeout in wait_for_indexing
- Check indexing isn't stuck

**Flaky tests?**
- Watch tests are timing-dependent
- Already lenient in implementation

**Permission errors?**
- Check temp dir permissions
- Verify database can be created

## 💡 Tips

```bash
# Fast iteration
make quick

# Before commit
make ci

# Coverage report
make coverage

# Watch mode
cargo watch -x "test --package hippo-cli"
```

## 🎓 Learning Resources

1. Read `TESTING.md` for full guide
2. Browse `tests/cli_tests.rs` for examples
3. Check `tests/test_structure.md` for diagrams
4. Use `make help` for all targets

## 📞 Need Help?

1. Check `TESTING.md` troubleshooting section
2. Review test code comments
3. Run with `--nocapture` for details
4. Enable `RUST_LOG=debug` for more info

---

**Remember**: Tests are isolated, cleanup is automatic, and you can run them as many times as needed!
