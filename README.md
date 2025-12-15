<p align="center">
  <img src="assets/hippo-logo.png" alt="Hippo Logo" width="120" height="120">
</p>

<h1 align="center">Hippo</h1>

<p align="center">
  <strong>The Memory That Never Forgets</strong>
</p>

<p align="center">
  A local-first, AI-powered file organizer built with Rust + Tauri
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/tauri-2.0-blue.svg" alt="Tauri">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome">
</p>

---

## What is Hippo?

Hippo is a **privacy-first file organizer** that runs entirely on your machine. It indexes your files, extracts metadata, and uses local AI (via Ollama) to help you search, organize, and understand your digital life.

**No cloud. No subscriptions. Your files, your data, your control.**

```
┌─────────────────────────────────────────────────────────────┐
│  🦛 Hippo                                              ─ □ x │
├─────────────────────────────────────────────────────────────┤
│  🔍 Search your memories...                    [Images ▾]   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │ 📷   │  │ 📄   │  │ 💻   │  │ 🎵   │  │ 📁   │          │
│  │photo │  │ doc  │  │ code │  │audio │  │folder│          │
│  └──────┘  └──────┘  └──────┘  └──────┘  └──────┘          │
│                                                             │
│  137,000+ files indexed • 3 sources • AI Ready             │
└─────────────────────────────────────────────────────────────┘
```

## Features

### Core Features
- **Instant Search** - SQL-powered full-text search across 100K+ files in milliseconds
- **Smart Tags** - Auto-generated tags based on file type, location, and content
- **Multiple Sources** - Index Documents, Desktop, Downloads, or any folder
- **File Preview** - Quick preview with metadata display
- **Dark/Light Mode** - Easy on the eyes, any time of day

### AI Features (Local via Ollama)
- **Natural Language Search** - Ask questions about your files in plain English
- **Smart Organization** - AI suggests folder structures and categories
- **Duplicate Detection** - Find and manage duplicate files
- **Similar Files** - Discover related content across your library
- **Tag Suggestions** - AI-powered tag recommendations

### Technical Highlights
- **100% Local** - All processing happens on your machine
- **Blazing Fast** - Rust backend with SQLite FTS5 for instant search
- **Cross-Platform** - macOS, Windows, Linux support via Tauri
- **Lightweight** - ~50MB app size, minimal resource usage
- **Extensible** - Plugin-ready architecture

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [Ollama](https://ollama.ai/) (for AI features)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Pull the AI models
ollama pull qwen2:0.5b        # Fast chat model (352MB)
ollama pull nomic-embed-text  # Embeddings (274MB)

# Build and run
cargo run --bin hippo-tauri
```

### CLI Usage

Hippo also includes a fun CLI with hippo-themed commands:

```bash
# Build the CLI
cargo build --bin hippo

# Check your index stats
./target/debug/hippo weight
# => 137,339 memories indexed

# Search for files
./target/debug/hippo sniff "vacation photos"

# Find duplicates
./target/debug/hippo twins

# Watch for file changes
./target/debug/hippo wade

# List your sources
./target/debug/hippo herd
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         HIPPO ARCHITECTURE                       │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Tauri UI   │────▶│  hippo-tauri │────▶│  hippo-core  │
│  (HTML/JS)   │     │   (Commands) │     │    (Rust)    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                     ┌────────────────────────────┼────────────────┐
                     │                            │                │
                     ▼                            ▼                ▼
              ┌────────────┐              ┌────────────┐    ┌────────────┐
              │   SQLite   │              │   Ollama   │    │  Indexer   │
              │  (Storage) │              │    (AI)    │    │  (Files)   │
              └────────────┘              └────────────┘    └────────────┘
                     │                            │                │
                     │                            │                │
              ┌──────┴──────┐              ┌──────┴──────┐   ┌─────┴─────┐
              │  Memories   │              │  Embeddings │   │  Sources  │
              │    Tags     │              │     RAG     │   │  Watcher  │
              │   Search    │              │    Chat     │   │  Extract  │
              └─────────────┘              └─────────────┘   └───────────┘
```

### Project Structure

```
hippo/
├── hippo-core/          # Core Rust library
│   └── src/
│       ├── lib.rs       # Main API
│       ├── models.rs    # Data types
│       ├── storage/     # SQLite layer
│       ├── search/      # Search engine
│       ├── indexer/     # File discovery
│       ├── embeddings/  # Vector search
│       └── ollama/      # AI integration
│
├── hippo-tauri/         # Desktop app
│   ├── src/main.rs      # Tauri commands
│   └── ui/dist/         # Frontend (HTML/JS)
│
├── hippo-cli/           # Command-line interface
│   └── src/main.rs      # CLI commands
│
└── docs/                # Documentation
    └── ARCHITECTURE.md  # Detailed architecture
```

### Data Flow

```
File System                    Hippo                         User
    │                           │                              │
    │  ──── file changes ────▶  │                              │
    │                           │                              │
    │                     ┌─────┴─────┐                        │
    │                     │  Indexer  │                        │
    │                     │  Extract  │                        │
    │                     │  Metadata │                        │
    │                     └─────┬─────┘                        │
    │                           │                              │
    │                     ┌─────┴─────┐                        │
    │                     │  Storage  │                        │
    │                     │  SQLite   │                        │
    │                     └─────┬─────┘                        │
    │                           │                              │
    │                     ┌─────┴─────┐                        │
    │                     │  Search   │  ◀──── query ────────  │
    │                     │  Engine   │  ────── results ────▶  │
    │                     └─────┬─────┘                        │
    │                           │                              │
    │                     ┌─────┴─────┐                        │
    │                     │  Ollama   │  ◀──── AI query ─────  │
    │                     │   (AI)    │  ────── response ───▶  │
    │                     └───────────┘                        │
```

## Tech Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Backend | Rust | Core logic, performance |
| Desktop | Tauri 2.0 | Native app wrapper |
| Database | SQLite + FTS5 | Storage & full-text search |
| AI | Ollama | Local LLM inference |
| UI | Vanilla JS/CSS | Lightweight, no build step |
| Embeddings | nomic-embed-text | Vector similarity |
| Chat Model | qwen2:0.5b | Fast responses |

## Performance

Hippo is designed for speed:

| Operation | Time | Files |
|-----------|------|-------|
| Initial index | ~5 min | 100K files |
| Search | <50ms | 137K files |
| AI response | ~1.5s | With qwen2:0.5b |
| Memory usage | ~100MB | Idle |

## Contributing

We love contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Contribution Guide

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/hippo.git

# Create a branch
git checkout -b feature/amazing-feature

# Make your changes and test
cargo test
cargo run --bin hippo-tauri

# Commit and push
git commit -m "Add amazing feature"
git push origin feature/amazing-feature

# Open a Pull Request
```

### Areas We Need Help

- [ ] Windows/Linux testing
- [ ] More file type extractors
- [ ] Thumbnail generation
- [ ] Cloud sync (optional)
- [ ] Localization
- [ ] Accessibility improvements

## Roadmap

- [x] Core indexing & search
- [x] AI chat integration
- [x] Duplicate detection
- [x] File watching
- [x] Custom icons & branding
- [ ] Image thumbnails
- [ ] Face detection/clustering
- [ ] Mobile companion app
- [ ] Browser extension
- [ ] Sync between devices (E2E encrypted)

## License

MIT License - see [LICENSE](LICENSE) for details.

**Built with love by the Hippo community.**

---

<p align="center">
  <sub>
    🦛 Fun fact: Hippos can hold their breath underwater for up to 5 minutes.
    <br>
    Hippo (the app) can hold 100,000+ files in memory and never forget them!
  </sub>
</p>
