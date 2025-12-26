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
  <a href="https://github.com/greplabs/hippo/actions/workflows/quick-ci.yml"><img src="https://github.com/greplabs/hippo/actions/workflows/quick-ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/tauri-2.0-blue.svg" alt="Tauri">
  <img src="https://img.shields.io/badge/ollama-local_AI-purple.svg" alt="Ollama">
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
- **Semantic Search** - Vector similarity search powered by Qdrant
- **Smart Tags** - Auto-generated tags based on file type, location, and AI analysis
- **Multiple Sources** - Index Documents, Desktop, Downloads, or any folder
- **File Preview** - Quick preview with metadata, thumbnails, and code syntax highlighting
- **Real-time Watching** - Automatic re-indexing when files change

### AI Features (Local via Ollama - No API Key Required)
- **Natural Language Search** - Ask questions about your files in plain English
- **RAG-Powered Answers** - AI finds relevant documents and generates accurate responses
- **Smart Organization** - AI suggests folder structures and categories
- **Similar Files** - Discover related content using vector similarity
- **Auto-Tagging** - AI-powered tag suggestions for new files
- **Image Captioning** - Automatic descriptions for photos (llava:7b)
- **Code Analysis** - Understand code structure and patterns

### Developer Features
- **Syntax Highlighting** - Prism.js support for 20+ languages
- **Code Preview** - View source code with proper formatting
- **Git-Friendly** - Works alongside your version control

### Technical Highlights
- **100% Local** - All processing happens on your machine, no cloud required
- **Privacy First** - Your files never leave your computer
- **Blazing Fast** - Rust backend with SQLite + Qdrant for instant search
- **Cross-Platform** - macOS, Windows, Linux support via Tauri 2.0
- **Lightweight** - ~50MB app size, minimal resource usage
- **Extensible** - Clean architecture for easy customization

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [Ollama](https://ollama.ai/) (for AI features)
- [Docker](https://www.docker.com/) & Docker Compose (for web deployment)

### Quick Start (Desktop App)

```bash
# Clone the repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Pull the AI models (one-time setup)
ollama pull nomic-embed-text  # Required: embeddings (274MB)
ollama pull gemma2:2b         # Recommended: excellent quality (1.6GB)
# Optional: ollama pull llava:7b  # For image analysis

# Build and run
cargo run --bin hippo-tauri
```

### Vercel Deployment (Live Demo)

Deploy a live demo of Hippo with static sample data:

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy from project root
vercel

# Production deployment
vercel --prod
```

Or click here: [![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/greplabs/hippo)

The demo uses the `demo/` directory with mock API responses and sample data.

### Docker Deployment (Web Server)

Deploy Hippo as a web service with Docker Compose:

```bash
# Clone the repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Configure environment variables
cp .env.example .env
# Edit .env to customize your setup (ports, paths, etc.)

# Start services (Hippo Web + Qdrant)
docker-compose up -d

# View logs
docker-compose logs -f hippo-web

# Stop services
docker-compose down
```

The web interface will be available at `http://localhost:3000`.

#### Docker Configuration

**Customize indexed paths**: Edit `.env` and add your directories:

```bash
# .env
HIPPO_INDEX_PATH_1=/path/to/your/documents
HIPPO_INDEX_PATH_2=/path/to/your/photos
HIPPO_INDEX_PATH_3=/path/to/your/videos
```

Then update `docker-compose.yml` to mount these paths:

```yaml
volumes:
  - /path/to/your/documents:/mnt/index/documents:ro
  - /path/to/your/photos:/mnt/index/photos:ro
  - /path/to/your/videos:/mnt/index/videos:ro
```

**Access Qdrant Web UI**: Navigate to `http://localhost:6333/dashboard`

**Connect to Ollama on host**: Ollama running on your host machine is accessible via `http://host.docker.internal:11434`

#### Development with Docker

For development with hot reload:

```bash
# Start with development overrides
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up

# This mounts your source code and rebuilds on changes
# Using cargo-watch for automatic recompilation
```

#### Docker Services

The Docker Compose stack includes:

- **hippo-web**: Web server with REST API (port 3000)
- **qdrant**: Vector database for semantic search (ports 6333, 6334)

#### Health Checks

Check service health:

```bash
# Hippo Web health endpoint
curl http://localhost:3000/health

# Qdrant health
curl http://localhost:6333/
```

#### Resource Management

Default resource limits (configurable in `.env`):

- **Hippo Web**: 2 CPU cores, 2GB RAM
- **Qdrant**: 1 CPU core, 1GB RAM

Adjust in `.env`:

```bash
HIPPO_CPU_LIMIT=4.0
HIPPO_MEM_LIMIT=4G
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
| Database | SQLite + Qdrant | Storage & vector search |
| AI | Ollama | Local LLM inference |
| UI | Vanilla JS/CSS | Lightweight, no build step |
| Embeddings | nomic-embed-text | Vector similarity (768-dim) |
| Chat Model | gemma2:2b | Quality responses |
| Vision | llava:7b | Image understanding |

## Performance

Hippo is designed for speed:

| Operation | Time | Details |
|-----------|------|---------|
| Initial index | ~5 min | 100K files with metadata extraction |
| Text search | <50ms | SQLite FTS5, 100K+ files |
| Semantic search | <200ms | Qdrant vector similarity |
| AI response | ~2s | With gemma2:2b (quality mode) |
| Memory usage | ~100MB | Idle, scales with index size |
| App size | ~50MB | Minimal dependencies |

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

### Completed
- [x] Core indexing & search (SQL + FTS5)
- [x] Semantic vector search (Qdrant)
- [x] AI chat with RAG integration
- [x] Auto-tagging with Ollama
- [x] Real-time file watching (notify crate)
- [x] Image/video/PDF thumbnails
- [x] Code syntax highlighting (Prism.js)
- [x] Parallel CI/CD pipeline

### Coming Soon
- [ ] Dark mode theme toggle
- [ ] Search history & saved searches
- [ ] Streaming AI responses
- [ ] Batch file analysis
- [ ] Knowledge graph visualization

### Future
- [ ] Cloud integrations (Google Drive, iCloud, Dropbox)
- [ ] Face clustering for photos
- [ ] Mobile companion app
- [ ] Browser extension
- [ ] E2E encrypted sync

## Documentation

- [Deployment Guide](DEPLOYMENT.md) - Complete deployment guide (Vercel, Docker, Desktop)
- [Docker Deployment Guide](DOCKER.md) - Comprehensive Docker documentation
- [Docker Quick Start](QUICKSTART-DOCKER.md) - Get started in 5 minutes
- [Contributing Guidelines](CONTRIBUTING.md) - How to contribute
- [Architecture Overview](CLAUDE.md) - Detailed project structure

## Acknowledgements

Hippo is built on the shoulders of giants. Special thanks to:

- [Tauri](https://tauri.app/) - Cross-platform desktop framework
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [SQLite](https://www.sqlite.org/) - Embedded database
- [Qdrant](https://qdrant.tech/) - Vector search engine
- [Ollama](https://ollama.ai/) - Local AI model runner
- [Prism.js](https://prismjs.com/) - Syntax highlighting

## License

MIT License - see [LICENSE](LICENSE) for details.

Copyright (c) 2024-2025 [GrepLabs](https://github.com/greplabs)

**Built with love by the GrepLabs team.**

---

<p align="center">
  <sub>
    🦛 Fun fact: Hippos can hold their breath underwater for up to 5 minutes.
    <br>
    Hippo (the app) can hold 100,000+ files in memory and never forget them!
  </sub>
</p>
