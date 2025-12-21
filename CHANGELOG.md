# Changelog

All notable changes to Hippo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-12-21

### Added
- **Audio Waveform Thumbnails**: Visual waveform previews for audio files
- **AI-Powered Features**:
  - Claude API integration for intelligent file analysis
  - Ollama support for local AI processing
  - Smart tag suggestions with confidence scores
  - File captioning and description generation
  - RAG-based queries over your file collection
- **Duplicate Detection**: Hash-based duplicate file finder with `hippo twins`
- **File Watching**: Real-time file system monitoring with `hippo wade`
- **Colorful CLI**: Enhanced terminal output with icons and colors
- **Dark Mode UI**: Beautiful dark theme support in the desktop app
- **Bulk Operations**: Multi-select files for batch tagging and actions
- **Virtual Paths**: Organize files into virtual collections
- **Search Enhancements**:
  - Natural language search parsing
  - Date range filtering
  - Saved searches
  - Recent search history
- **Mind Map View**: Visual knowledge graph of file relationships
- **Code Graph**: Visualize code dependencies and imports
- **Symbol Search**: Find code symbols across your codebase

### Improved
- UI animations and transitions
- Thumbnail caching with smart invalidation
- Progress indicators during indexing
- Error handling and user feedback
- Search performance with better filtering

### Fixed
- CI/CD pipeline stability
- Docker build compatibility
- Cross-platform builds for macOS, Linux, Windows
- Memory usage during large file indexing

## [0.1.0] - 2024-12-01

### Added
- Initial release
- Core indexing functionality for 70+ file types
- SQLite-based local storage
- Basic search with tag filtering
- Tauri desktop application
- CLI tool with fun hippo-themed commands
- EXIF metadata extraction for images
- Code parsing for Rust, Python, JavaScript, Go
- Thumbnail generation for images and videos
