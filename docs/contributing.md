---
layout: default
title: Contributing
nav_order: 7
description: "How to contribute to Hippo - code, documentation, testing, and more"
---

# Contributing to Hippo
{: .no_toc }

Help make Hippo better for everyone
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Welcome!

First off, thank you for considering contributing to Hippo! It's people like you that make Hippo such a great tool.

There are many ways to contribute:
- Reporting bugs
- Suggesting features
- Writing code
- Improving documentation
- Testing on different platforms
- Sharing your experience

---

## Quick Links

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Submitting Changes](#submitting-changes)
- [Style Guidelines](#style-guidelines)

---

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code.

**Our Standards**:
- Be respectful and inclusive
- Welcome newcomers
- Focus on what's best for the community
- Show empathy towards others

---

## Getting Started

### Types of Contributions

**Bug Reports**
: Found a bug? Open an issue with steps to reproduce

**Feature Requests**
: Have an idea? We'd love to hear it!

**Code Contributions**
: Fix bugs, add features, improve performance

**Documentation**
: Help others understand Hippo better

**Testing**
: Try Hippo on different systems and report issues

**Design**
: UI/UX improvements, icons, graphics

### Good First Issues

Look for issues labeled `good first issue` - these are great starting points for new contributors.

**Current good first issues**:
- Add new file type extractors
- Improve error messages
- Add keyboard shortcuts
- Write more tests

---

## Development Setup

### Prerequisites

```bash
# Install Rust (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Ollama (optional, for AI features)
# macOS
brew install ollama

# Or download from https://ollama.ai

# Pull required models
ollama pull qwen2:0.5b
ollama pull nomic-embed-text
```

### Clone and Build

```bash
# Fork the repository on GitHub first, then:
git clone https://github.com/YOUR_USERNAME/hippo.git
cd hippo

# Build all packages
cargo build

# Run the desktop app
cargo run --bin hippo-tauri

# Run the CLI
cargo run --bin hippo

# Run tests
cargo test
```

### Development Tools

```bash
# Watch for changes and rebuild
cargo install cargo-watch
cargo watch -x 'run --bin hippo-tauri'

# Check for issues
cargo clippy

# Format code
cargo fmt

# Run tests continuously
cargo watch -x test
```

---

## Project Structure

```
hippo/
├── hippo-core/           # Core library (start here!)
│   └── src/
│       ├── lib.rs        # Public API
│       ├── models.rs     # Data structures
│       ├── error.rs      # Error types
│       ├── storage/      # SQLite database
│       ├── search/       # Search engine
│       ├── indexer/      # File indexing
│       ├── embeddings/   # Vector search
│       └── ollama/       # AI integration
│
├── hippo-tauri/          # Desktop application
│   ├── src/main.rs       # Tauri commands (IPC)
│   └── ui/dist/index.html # Complete UI
│
├── hippo-cli/            # Command-line interface
│   └── src/main.rs       # CLI commands
│
├── hippo-web/            # Web server (REST API)
└── hippo-wasm/           # WebAssembly bindings
```

### Key Files

| File | Purpose |
|------|---------|
| `hippo-core/src/lib.rs` | Main Hippo struct and public API |
| `hippo-core/src/models.rs` | All data types (Memory, Tag, Source) |
| `hippo-core/src/storage/mod.rs` | Database operations |
| `hippo-core/src/search/mod.rs` | Search engine logic |
| `hippo-tauri/src/main.rs` | Tauri commands (frontend ↔ backend) |
| `hippo-tauri/ui/dist/index.html` | Complete UI in one file |

See [Architecture Guide](architecture) for detailed information.

---

## Making Changes

### Branch Naming

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation
- `refactor/` - Code refactoring
- `test/` - Test additions

Example: `feature/thumbnail-generation`

### Commit Messages

We follow conventional commits:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Tests
- `chore`: Maintenance

**Examples**:

```
feat(search): add fuzzy matching support

Implements fuzzy matching using the fuzzy-matcher crate.
Improves search UX for typos and partial matches.

Closes #123
```

```
fix(indexer): handle symlinks correctly

Previously, symlinks caused infinite loops.
Now we skip symlinks entirely.
```

```
docs(readme): add installation instructions
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_search

# Run tests with output
cargo test -- --nocapture

# Run clippy for lints
cargo clippy -- -D warnings

# Run tests on file change
cargo watch -x test
```

**Write tests for**:
- New features
- Bug fixes
- Edge cases

**Example test**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_with_tags() {
        let hippo = Hippo::new().await.unwrap();

        // Add test data
        // ...

        let results = hippo.search("query").await.unwrap();
        assert_eq!(results.memories.len(), 5);
    }
}
```

---

## Submitting Changes

### Pull Request Process

1. **Fork** the repository on GitHub
2. **Clone** your fork locally
3. **Create** a feature branch (`git checkout -b feature/amazing`)
4. **Make** your changes
5. **Test** thoroughly
6. **Commit** with clear messages
7. **Push** to your fork
8. **Open** a Pull Request

### PR Checklist

Before submitting, ensure:

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow convention
- [ ] PR description explains changes

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation

## Testing
How did you test this?

- [ ] Added unit tests
- [ ] Tested manually
- [ ] Tested on [OS/Platform]

## Screenshots (if applicable)

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings
```

---

## Style Guidelines

### Rust Code Style

```rust
// Use descriptive names
fn search_memories_with_filters(query: &SearchQuery) -> Result<Vec<Memory>> {
    // Implementation
}

// Document public APIs
/// Searches for memories matching the given query.
///
/// # Arguments
/// * `query` - The search query with filters
///
/// # Returns
/// A vector of matching memories, sorted by relevance
pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
    // Implementation
}

// Handle errors explicitly
let result = storage.get_memory(id).await?;

// Use early returns
if query.is_empty() {
    return Ok(vec![]);
}

// Prefer match over if-let chains
match kind {
    MemoryKind::Image { .. } => extract_image_metadata(path),
    MemoryKind::Video { .. } => extract_video_metadata(path),
    _ => None,
}
```

### JavaScript Style (UI)

```javascript
// Use const/let, not var
const memories = await invoke('search', { query, tags });

// Use async/await
async function handleSearch() {
    try {
        const results = await search(query);
        render(results);
    } catch (err) {
        showError(err.message);
    }
}

// Keep functions small and focused
function formatFileSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// Use meaningful variable names
const isIndexing = state.indexingInProgress;
const hasResults = memories.length > 0;
```

### Documentation

- Use clear, concise language
- Provide code examples
- Explain the "why", not just the "what"
- Keep documentation up-to-date with code changes

---

## Adding New Features

### Adding a New Tauri Command

**1. Define command in `hippo-tauri/src/main.rs`**:

```rust
#[tauri::command]
async fn my_new_command(
    state: State<'_, AppState>,
    param: String,
) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    // Your implementation
    Ok("Success".to_string())
}
```

**2. Register in builder**:

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        initialize,
        search,
        my_new_command,  // Add here
    ])
```

**3. Call from UI**:

```javascript
const result = await window.__TAURI__.core.invoke('my_new_command', {
    param: 'value'
});
```

### Adding a New File Type Extractor

**1. Add extension to supported list** (`hippo-core/src/indexer/mod.rs`):

```rust
const SUPPORTED_EXTENSIONS: &[&str] = &[
    // ... existing
    "newext",
];
```

**2. Create extractor** (`hippo-core/src/indexer/extractors.rs`):

```rust
pub fn extract_newtype_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Parse file
    let data = std::fs::read(path).ok()?;

    // Extract metadata
    Some(MemoryMetadata {
        title: Some("...".to_string()),
        file_size: data.len() as u64,
        // ... more fields
    })
}
```

**3. Register extractor** (`hippo-core/src/indexer/mod.rs`):

```rust
match extension {
    "newext" => extract_newtype_metadata(&path),
    // ... other cases
}
```

### Adding a New CLI Command

**1. Add to enum** (`hippo-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands

    /// My new command description
    MyCommand {
        /// Argument description
        #[arg(short, long)]
        option: String,
    },
}
```

**2. Implement handler**:

```rust
match cli.command {
    // ... existing handlers

    Commands::MyCommand { option } => {
        print_header("My Command");
        // Implementation
        print_success("Done!");
    }
}
```

---

## Community

### Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and ideas
- **Discord**: Coming soon!

### Recognition

Contributors are recognized in:
- README.md contributors section
- Release notes
- Our eternal gratitude!

### Code Review Process

1. Maintainer reviews PR
2. Feedback provided (if needed)
3. Author addresses feedback
4. Approved and merged
5. Contributor celebrated!

**Review focuses on**:
- Code quality
- Tests
- Documentation
- Performance
- Security

---

## Release Process

Hippo follows semantic versioning (SemVer):

- **Major** (1.0.0): Breaking changes
- **Minor** (0.1.0): New features (backward compatible)
- **Patch** (0.0.1): Bug fixes

**Release checklist**:
1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Run full test suite
4. Create git tag
5. Push to GitHub
6. Publish to crates.io
7. Create GitHub release

---

## Questions?

**Before asking**:
1. Check existing issues
2. Read the documentation
3. Search discussions

**When asking**:
- Be specific
- Provide context
- Include error messages
- Share code examples

---

## Thank You!

Every contribution, no matter how small, makes Hippo better. Whether you're fixing a typo or adding a major feature, we appreciate your help!

**Happy coding!** 🦛

---

## Additional Resources

- [Installation Guide](installation)
- [Architecture Guide](architecture)
- [API Reference](api)
- [CLI Guide](cli-guide)
- [Desktop App Guide](desktop-app)

---

Built with love by the Hippo community.
