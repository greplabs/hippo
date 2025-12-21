---
layout: default
title: Home
nav_order: 1
description: "Hippo - The Memory That Never Forgets. A local-first, AI-powered file organizer built with Rust and Tauri."
permalink: /
---

# Hippo
{: .fs-9 }

The Memory That Never Forgets
{: .fs-6 .fw-300 }

A local-first, AI-powered file organizer built with Rust + Tauri
{: .fs-5 .fw-300 }

[Get Started](installation){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/greplabs/hippo){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## What is Hippo?

Hippo is a **privacy-first file organizer** that runs entirely on your machine. It indexes your files, extracts metadata, and uses local AI (via Ollama or Claude) to help you search, organize, and understand your digital life.

**No cloud. No subscriptions. Your files, your data, your control.**

### Key Features

- **Instant Search** - SQL-powered full-text search across 100K+ files in milliseconds
- **Smart Tags** - Auto-generated tags based on file type, location, and content
- **Multiple Sources** - Index Documents, Desktop, Downloads, or any folder
- **File Preview** - Quick preview with rich metadata display
- **AI-Powered** - Local AI via Ollama or Claude API for intelligent organization
- **Duplicate Detection** - Find and manage duplicate files with hash-based detection
- **File Watching** - Automatically update index when files change
- **Cross-Platform** - macOS, Windows, and Linux support

### Use Cases

**Personal Knowledge Management**
: Index your documents, notes, and research papers for instant retrieval

**Photo Organization**
: Extract EXIF data, GPS coordinates, and camera information from images

**Code Navigation**
: Parse source code files to find functions, imports, and dependencies

**Media Library**
: Organize videos, audio files, and podcasts with metadata extraction

**Digital Decluttering**
: Find and remove duplicate files to reclaim disk space

## Quick Start

### Desktop App (Recommended)

The fastest way to get started is with the Tauri desktop application:

```bash
# Clone the repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Pull AI models (optional, for AI features)
ollama pull qwen2:0.5b        # Fast chat model (352MB)
ollama pull nomic-embed-text  # Embeddings (274MB)

# Build and run
cargo run --bin hippo-tauri
```

The desktop app provides a beautiful UI with:
- Grid and list view modes
- Real-time search with debouncing
- Type filters (Images, Videos, Audio, Code, Docs)
- Tag management
- Thumbnail previews
- Keyboard shortcuts

### CLI Tool

For power users who prefer the command line:

```bash
# Build the CLI
cargo build --bin hippo

# Index a folder
./target/debug/hippo chomp ~/Documents

# Search for files
./target/debug/hippo sniff "vacation photos"

# Find duplicates
./target/debug/hippo twins

# Watch for file changes
./target/debug/hippo wade

# Get AI-powered tags
export ANTHROPIC_API_KEY=your_key_here
./target/debug/hippo brain
```

See the [CLI Guide](cli-guide) for all available commands.

### Web Server

Deploy Hippo as a web service with Docker:

```bash
# Start with Docker Compose
docker-compose up -d

# Access at http://localhost:3000
```

See [Installation Guide](installation#docker-deployment) for detailed setup instructions.

## Architecture Overview

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
```

Hippo is built as a modular Rust workspace:

- **hippo-core**: Core library with indexing, search, and storage
- **hippo-tauri**: Desktop application with Tauri 2.0
- **hippo-cli**: Command-line interface with fun hippo-themed commands
- **hippo-web**: REST API server (Docker-ready)
- **hippo-wasm**: WebAssembly bindings for browser integration

Learn more in the [Architecture Guide](architecture).

## Performance

Hippo is designed for speed and efficiency:

| Operation | Time | Files |
|-----------|------|-------|
| Initial index | ~5 min | 100K files |
| Search | <50ms | 137K files |
| AI response | ~1.5s | With qwen2:0.5b |
| Memory usage | ~100MB | Idle |

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Backend | Rust | Core logic, performance |
| Desktop | Tauri 2.0 | Native app wrapper |
| Database | SQLite + FTS5 | Storage & full-text search |
| AI (Local) | Ollama | Local LLM inference |
| AI (Cloud) | Claude API | Advanced analysis |
| UI | Vanilla JS/CSS | Lightweight, no build step |
| Embeddings | nomic-embed-text | Vector similarity |
| Chat Model | qwen2:0.5b | Fast responses |

## Supported File Types

Hippo supports 70+ file types across multiple categories:

**Images**: JPG, PNG, GIF, WebP, HEIC, RAW formats
**Videos**: MP4, MOV, AVI, MKV, WebM
**Audio**: MP3, WAV, FLAC, AAC, OGG
**Documents**: PDF, DOCX, XLSX, PPTX, TXT, Markdown
**Code**: Rust, Python, JavaScript, Go, TypeScript, Java
**Archives**: ZIP, TAR, GZ, 7Z, RAR

See [Architecture Guide](architecture#supported-file-types) for the complete list.

## Next Steps

Ready to dive deeper?

- [Installation Guide](installation) - Detailed setup instructions
- [CLI Guide](cli-guide) - Master the command-line interface
- [Desktop App Guide](desktop-app) - Learn the Tauri app features
- [API Reference](api) - Developer documentation
- [Architecture](architecture) - Deep dive into the codebase
- [Contributing](contributing) - Help make Hippo better

## Community and Support

- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Ask questions and share ideas
- **Discord**: Coming soon!

---

Built with love by the Hippo community.

Fun fact: Hippos can hold their breath underwater for up to 5 minutes. Hippo (the app) can hold 100,000+ files in memory and never forget them!
