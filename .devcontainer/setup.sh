#!/bin/bash
set -e

echo "Setting up Hippo development environment..."

# Install Tauri dependencies for Linux
echo "Installing Tauri system dependencies..."
sudo apt-get update
sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev

# Install ffmpeg for video duration extraction
echo "Installing ffmpeg..."
sudo apt-get install -y ffmpeg

# Install Rust components
echo "Setting up Rust toolchain..."
rustup component add clippy rustfmt
rustup target add wasm32-unknown-unknown

# Install cargo tools
echo "Installing cargo tools..."
cargo install tauri-cli --locked || true
cargo install cargo-watch || true

# Install Ollama
echo "Installing Ollama..."
curl -fsSL https://ollama.com/install.sh | sh || true

# Build the project to download dependencies
echo "Building project..."
cargo build

echo ""
echo "Setup complete!"
echo ""
echo "To start developing:"
echo "  cargo run --bin hippo-tauri    # Run the desktop app"
echo "  cargo run --bin hippo          # Run the CLI"
echo ""
echo "For AI features, start Ollama and pull models:"
echo "  ollama serve &"
echo "  ollama pull qwen2:0.5b"
echo "  ollama pull nomic-embed-text"
echo ""
