---
layout: default
title: Installation
nav_order: 2
description: "How to install and set up Hippo - CLI, Desktop App, and Web Server"
---

# Installation Guide
{: .no_toc }

Complete guide to installing Hippo on your system
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Prerequisites

Before installing Hippo, ensure you have the following:

### Required

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Optional (For AI Features)

- **Ollama**: For local AI features
  - macOS: `brew install ollama` or download from [ollama.ai](https://ollama.ai)
  - Linux: See [Ollama installation](https://ollama.ai/download)
  - Windows: Download installer from [ollama.ai](https://ollama.ai)

- **Claude API Key**: For cloud AI features
  - Get one from [Anthropic](https://console.anthropic.com/)

### Optional (For Docker Deployment)

- **Docker & Docker Compose**: [Get Docker](https://www.docker.com/get-started)

---

## Installation Methods

### Method 1: Desktop App (Recommended)

The easiest way to use Hippo is through the Tauri desktop application.

#### 1. Clone the Repository

```bash
git clone https://github.com/greplabs/hippo.git
cd hippo
```

#### 2. Pull AI Models (Optional)

If you want AI features, pull the required Ollama models:

```bash
# Start Ollama service
ollama serve

# In another terminal, pull models
ollama pull qwen2:0.5b        # Fast chat model (352MB)
ollama pull nomic-embed-text  # Text embeddings (274MB)
```

#### 3. Build and Run

```bash
# Build and run the desktop app
cargo run --bin hippo-tauri

# Or build a release version
cargo build --release --bin hippo-tauri
./target/release/hippo-tauri
```

#### 4. First Launch

On first launch, Hippo will:
1. Create a database at `~/Library/Application Support/Hippo/hippo.db` (macOS)
2. Initialize the SQLite schema
3. Optionally start Qdrant for vector search

Now you can add folders to index!

---

### Method 2: CLI Tool

For command-line usage, build the CLI binary.

#### 1. Clone and Build

```bash
git clone https://github.com/greplabs/hippo.git
cd hippo

# Build the CLI
cargo build --release --bin hippo

# Optional: Add to PATH
sudo cp target/release/hippo /usr/local/bin/
```

#### 2. Verify Installation

```bash
hippo --version
hippo --help
```

#### 3. Start Using

```bash
# Index your first folder
hippo chomp ~/Documents

# Check stats
hippo weight

# Search files
hippo sniff "important"
```

See the [CLI Guide](cli-guide) for all available commands.

---

### Method 3: Docker Deployment

Deploy Hippo as a web service using Docker Compose.

#### 1. Clone the Repository

```bash
git clone https://github.com/greplabs/hippo.git
cd hippo
```

#### 2. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit .env to customize
nano .env
```

Key environment variables:

```bash
# Server settings
HIPPO_HOST=0.0.0.0
HIPPO_PORT=3000

# Index paths (add your directories here)
HIPPO_INDEX_PATH_1=/path/to/documents
HIPPO_INDEX_PATH_2=/path/to/photos

# Resource limits
HIPPO_CPU_LIMIT=2.0
HIPPO_MEM_LIMIT=2G

# Qdrant settings
QDRANT_HOST=qdrant
QDRANT_PORT=6333

# Ollama (if using local AI)
OLLAMA_HOST=http://host.docker.internal:11434
```

#### 3. Update Docker Compose Volumes

Edit `docker-compose.yml` to mount your folders:

```yaml
services:
  hippo-web:
    volumes:
      - hippo-data:/data
      - /path/to/documents:/mnt/index/documents:ro
      - /path/to/photos:/mnt/index/photos:ro
      - /path/to/videos:/mnt/index/videos:ro
```

#### 4. Start Services

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f hippo-web

# Check health
curl http://localhost:3000/health
```

#### 5. Access the Web UI

Open your browser to `http://localhost:3000`

#### Management Commands

```bash
# Stop services
docker-compose down

# Restart services
docker-compose restart

# View Qdrant dashboard
open http://localhost:6333/dashboard

# Remove all data (careful!)
docker-compose down -v
```

---

## Platform-Specific Instructions

### macOS

#### Build from Source

```bash
# Install Xcode Command Line Tools (if not already installed)
xcode-select --install

# Clone and build
git clone https://github.com/greplabs/hippo.git
cd hippo
cargo build --release --bin hippo-tauri
```

#### Database Location

```
~/Library/Application Support/Hippo/hippo.db
```

#### Permissions

You may need to grant permissions for:
- File access (when adding folders)
- Network access (for Qdrant/Ollama)

### Linux

#### Dependencies

```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y \
    libssl-dev \
    pkg-config \
    libsqlite3-dev \
    libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    libgtk-3-dev

# Arch Linux
sudo pacman -S webkit2gtk base-devel curl wget openssl sqlite
```

#### Build from Source

```bash
git clone https://github.com/greplabs/hippo.git
cd hippo
cargo build --release --bin hippo-tauri
```

#### Database Location

```
~/.local/share/Hippo/hippo.db
```

### Windows

#### Dependencies

- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

#### Build from Source

```powershell
# Clone repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Build
cargo build --release --bin hippo-tauri
```

#### Database Location

```
%APPDATA%\Hippo\hippo.db
```

---

## Post-Installation Setup

### Configure AI Features

#### Option 1: Local AI with Ollama

```bash
# Start Ollama
ollama serve

# Pull models
ollama pull qwen2:0.5b
ollama pull nomic-embed-text

# Hippo will auto-detect Ollama at localhost:11434
```

#### Option 2: Cloud AI with Claude

```bash
# Set API key
export ANTHROPIC_API_KEY=your_key_here

# Or use in CLI
hippo brain --api-key your_key_here
```

### Add Your First Source

#### Desktop App

1. Click "Add Source" button
2. Select a folder (e.g., Documents, Downloads)
3. Wait for indexing to complete
4. Start searching!

#### CLI

```bash
# Add a folder
hippo chomp ~/Documents

# Check progress
hippo weight

# List sources
hippo herd
```

### Enable File Watching

Keep your index up-to-date automatically:

```bash
# Watch all indexed sources
hippo wade

# Watch specific paths
hippo wade ~/Documents ~/Downloads
```

---

## Troubleshooting

### Database Issues

**Problem**: Database locked error
```bash
# Close all Hippo instances, then:
rm ~/Library/Application\ Support/Hippo/hippo.db-wal
rm ~/Library/Application\ Support/Hippo/hippo.db-shm
```

**Problem**: Database corrupted
```bash
# Reset the database (WARNING: loses all data)
hippo forget --force
```

### Qdrant Issues

**Problem**: Qdrant not starting
```bash
# Check if port 6333 is available
lsof -i :6333

# Kill conflicting process
kill -9 <PID>

# Restart Hippo
```

**Problem**: Vector search not working
```bash
# Verify Qdrant is running
curl http://localhost:6333/

# Check Qdrant logs
docker logs qdrant  # if using Docker
```

### Ollama Issues

**Problem**: Ollama not detected
```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve

# Verify models are pulled
ollama list
```

### Build Issues

**Problem**: Compilation errors
```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build
```

**Problem**: Missing dependencies (Linux)
```bash
# Install all required packages
sudo apt-get install -y libssl-dev pkg-config libsqlite3-dev
```

### Performance Issues

**Problem**: Slow indexing
- Reduce batch size in config
- Index smaller folders first
- Exclude large binary files

**Problem**: High memory usage
- Close other applications
- Index fewer files at once
- Disable thumbnail generation

---

## Upgrading

### From Source

```bash
cd hippo
git pull
cargo build --release
```

### Docker

```bash
cd hippo
docker-compose pull
docker-compose up -d
```

### Database Migration

Hippo automatically migrates the database schema on startup. No manual intervention needed.

---

## Uninstalling

### Remove Application

```bash
# Remove binaries
rm /usr/local/bin/hippo
rm target/release/hippo-tauri

# Remove source
rm -rf ~/path/to/hippo
```

### Remove Data

```bash
# macOS
rm -rf ~/Library/Application\ Support/Hippo

# Linux
rm -rf ~/.local/share/Hippo

# Windows
rmdir /s %APPDATA%\Hippo
```

### Docker

```bash
# Stop and remove containers
docker-compose down

# Remove volumes (deletes all data)
docker-compose down -v

# Remove images
docker rmi hippo-web qdrant/qdrant
```

---

## Next Steps

Now that Hippo is installed:

- [CLI Guide](cli-guide) - Learn all CLI commands
- [Desktop App Guide](desktop-app) - Master the desktop interface
- [API Reference](api) - Integrate Hippo into your workflow
- [Architecture](architecture) - Understand how Hippo works

---

Need help? [Open an issue](https://github.com/greplabs/hippo/issues) on GitHub.
