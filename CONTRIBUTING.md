# Contributing to Hippo

First off, thank you for considering contributing to Hippo! It's people like you that make Hippo such a great tool.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Submitting Changes](#submitting-changes)
- [Style Guidelines](#style-guidelines)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Types of Contributions

There are many ways to contribute:

- **Bug Reports**: Found a bug? Open an issue!
- **Feature Requests**: Have an idea? We'd love to hear it!
- **Code**: Fix bugs, add features, improve performance
- **Documentation**: Help others understand Hippo
- **Testing**: Try Hippo on different systems and report issues
- **Design**: UI/UX improvements, icons, graphics

### Good First Issues

Look for issues labeled `good first issue` - these are great starting points for new contributors.

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Ollama (for AI features)
# macOS
brew install ollama

# Or download from https://ollama.ai

# Pull required models
ollama pull qwen2:0.5b
ollama pull nomic-embed-text
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/greplabs/hippo.git
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

### Development Workflow

```bash
# Watch for changes and rebuild
cargo watch -x 'run --bin hippo-tauri'

# Check for issues
cargo clippy

# Format code
cargo fmt
```

## Project Structure

```
hippo/
├── hippo-core/           # Core library (start here!)
│   └── src/
│       ├── lib.rs        # Public API
│       ├── models.rs     # Data structures
│       ├── error.rs      # Error types
│       ├── storage/      # SQLite database
│       │   └── mod.rs    # CRUD operations
│       ├── search/       # Search engine
│       │   └── mod.rs    # Query & ranking
│       ├── indexer/      # File indexing
│       │   ├── mod.rs    # File discovery
│       │   ├── extractors.rs  # Metadata extraction
│       │   └── code_parser.rs # AST parsing
│       ├── embeddings/   # Vector search
│       │   └── mod.rs    # Embedding generation
│       └── ollama/       # AI integration
│           └── mod.rs    # Ollama API client
│
├── hippo-tauri/          # Desktop application
│   ├── src/
│   │   └── main.rs       # Tauri commands (IPC)
│   ├── ui/
│   │   └── dist/
│   │       └── index.html # Complete UI (no build step)
│   └── tauri.conf.json   # Tauri configuration
│
├── hippo-cli/            # Command-line interface
│   └── src/
│       └── main.rs       # CLI commands
│
└── docs/                 # Documentation
```

### Key Files to Know

| File | Purpose |
|------|---------|
| `hippo-core/src/lib.rs` | Main Hippo struct and public API |
| `hippo-core/src/models.rs` | All data types (Memory, Tag, Source) |
| `hippo-core/src/storage/mod.rs` | Database operations |
| `hippo-core/src/search/mod.rs` | Search engine logic |
| `hippo-tauri/src/main.rs` | Tauri commands (frontend ↔ backend) |
| `hippo-tauri/ui/dist/index.html` | Complete UI in one file |

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

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Tests
- `chore`: Maintenance

Examples:
```
feat(search): add fuzzy matching support
fix(indexer): handle symlinks correctly
docs(readme): add installation instructions
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run clippy for lints
cargo clippy -- -D warnings
```

## Submitting Changes

### Pull Request Process

1. **Fork** the repository
2. **Create** your feature branch (`git checkout -b feature/amazing`)
3. **Make** your changes
4. **Test** thoroughly
5. **Commit** with clear messages
6. **Push** to your fork
7. **Open** a Pull Request

### PR Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow convention

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

## Screenshots (if applicable)
```

## Style Guidelines

### Rust Code Style

```rust
// Use descriptive names
fn search_memories_with_filters(query: &SearchQuery) -> Result<Vec<Memory>>

// Document public APIs
/// Searches for memories matching the given query.
///
/// # Arguments
/// * `query` - The search query with filters
///
/// # Returns
/// A vector of matching memories, sorted by relevance
pub async fn search(&self, query: SearchQuery) -> Result<SearchResults>

// Handle errors explicitly
let result = storage.get_memory(id).await?;

// Use early returns
if query.is_empty() {
    return Ok(vec![]);
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
```

## Adding New Features

### Adding a New Tauri Command

1. Add the command in `hippo-tauri/src/main.rs`:
```rust
#[tauri::command]
async fn my_new_command(state: State<'_, AppState>, param: String) -> Result<String, String> {
    // Implementation
}
```

2. Register it in the builder:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    my_new_command,
])
```

3. Call from UI:
```javascript
const result = await window.__TAURI__.core.invoke('my_new_command', { param: 'value' });
```

### Adding a New File Type Extractor

1. Add to `hippo-core/src/indexer/extractors.rs`:
```rust
pub fn extract_my_format(path: &Path) -> Option<MemoryMetadata> {
    // Parse file and extract metadata
}
```

2. Register in `hippo-core/src/indexer/mod.rs`:
```rust
".myext" => extract_my_format(&path),
```

## Community

### Getting Help

- **GitHub Issues**: For bugs and feature requests
- **Discussions**: For questions and ideas
- **Discord**: [Join our server](#) (coming soon)

### Recognition

Contributors are recognized in:
- README.md contributors section
- Release notes
- Our eternal gratitude!

---

## Thank You!

Every contribution, no matter how small, makes Hippo better. Whether you're fixing a typo or adding a major feature, we appreciate your help!

**Happy coding!** 🦛
