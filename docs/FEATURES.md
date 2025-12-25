# Hippo Features

A comprehensive list of all features in Hippo - The Memory That Never Forgets.

---

## Core Features

### File Indexing
| Feature | Status | Description |
|---------|--------|-------------|
| Multi-folder indexing | ✅ | Index any number of local folders |
| 70+ file types | ✅ | Images, videos, audio, code, documents, archives |
| Parallel processing | ✅ | Multi-threaded batch indexing |
| Incremental updates | ✅ | Only process changed files |
| Progress tracking | ✅ | Real-time progress with ETA |
| Background indexing | ✅ | Non-blocking async operations |

### Metadata Extraction
| Feature | Status | Description |
|---------|--------|-------------|
| EXIF data | ✅ | Camera info, GPS, dimensions for images |
| Audio/video duration | ✅ | Length extraction via symphonia |
| Code parsing | ✅ | AST analysis for Rust, Python, JS, Go |
| Document text | ✅ | Text preview from PDFs and docs |
| File hashing | ✅ | SHA256 for duplicate detection |
| Custom metadata | ✅ | Extensible JSON metadata storage |

### Search Capabilities
| Feature | Status | Description |
|---------|--------|-------------|
| Full-text search | ✅ | SQLite FTS5 for instant results |
| Tag filtering | ✅ | Include/exclude tag modes |
| Semantic search | ✅ | Vector similarity via Qdrant |
| Hybrid search | ✅ | Combined text + semantic |
| Fuzzy matching | ✅ | Typo-tolerant with Levenshtein |
| Natural language | ✅ | Date parsing, smart queries |
| Type filtering | ✅ | Filter by file kind |
| Sort options | ✅ | Date, name, size sorting |

---

## AI Features (Local via Ollama)

### Text Generation
| Feature | Status | Model | Description |
|---------|--------|-------|-------------|
| RAG answers | ✅ | gemma2:2b | Context-aware responses |
| File analysis | ✅ | gemma2:2b | Document summarization |
| Code explanation | ✅ | gemma2:2b | Code understanding |
| Tag suggestions | ✅ | gemma2:2b | AI-powered tags |
| Organization tips | ✅ | gemma2:2b | Folder structure advice |

### Embeddings
| Feature | Status | Model | Description |
|---------|--------|-------|-------------|
| Text embeddings | ✅ | nomic-embed-text | 768-dim vectors |
| Semantic similarity | ✅ | nomic-embed-text | Find related files |
| Query embedding | ✅ | nomic-embed-text | Natural language search |

### Vision
| Feature | Status | Model | Description |
|---------|--------|-------|-------------|
| Image captioning | ✅ | llava:7b | Auto-describe photos |
| Visual search | 🔜 | llava:7b | Search by image |

---

## Storage & Database

### SQLite
| Feature | Status | Description |
|---------|--------|-------------|
| Local database | ✅ | No cloud required |
| Fast queries | ✅ | Optimized indexes |
| JSON columns | ✅ | Flexible schema |
| Tag counting | ✅ | Automatic aggregation |
| Export/import | ✅ | Backup capabilities |

### Qdrant Vector DB
| Feature | Status | Description |
|---------|--------|-------------|
| Vector storage | ✅ | 768-dim embeddings |
| Similarity search | ✅ | Cosine distance |
| Auto-managed | ✅ | Automatic download/start |
| Persistent | ✅ | Data survives restarts |

---

## File Watching

| Feature | Status | Description |
|---------|--------|-------------|
| Real-time monitoring | ✅ | Native OS events via notify |
| Auto re-indexing | ✅ | Files updated automatically |
| Deletion tracking | ✅ | Removed files cleaned up |
| Debouncing | ✅ | Configurable delay (500ms) |
| Pause/resume | ✅ | Temporarily disable watching |
| Event statistics | ✅ | Track processed events |

---

## Thumbnails

| Feature | Status | Description |
|---------|--------|-------------|
| Image thumbnails | ✅ | 256x256 JPEG |
| Video thumbnails | ✅ | Frame extraction at 2s |
| PDF thumbnails | ✅ | First page rendering |
| Office thumbnails | ✅ | Embedded preview extraction |
| Memory cache | ✅ | LRU with 50MB limit |
| Disk cache | ✅ | Persistent SHA256-named |
| Smart invalidation | ✅ | Regenerate on file change |

---

## UI Features (Desktop App)

### Layout
| Feature | Status | Description |
|---------|--------|-------------|
| Grid view | ✅ | Card-based file display |
| List view | ✅ | Compact table format |
| Detail panel | ✅ | File info sidebar |
| Responsive | ✅ | Adapts to window size |

### Search UI
| Feature | Status | Description |
|---------|--------|-------------|
| Real-time search | ✅ | Debounced input |
| Type filter pills | ✅ | All, Images, Videos, etc. |
| Tag suggestions | ✅ | Tab to add as filter |
| Sort dropdown | ✅ | Multiple sort options |

### File Actions
| Feature | Status | Description |
|---------|--------|-------------|
| Open file | ✅ | Default application |
| Reveal in Finder | ✅ | Show in file manager |
| Toggle favorite | ✅ | Star/unstar files |
| Add/remove tags | ✅ | Manual tagging |

### Code Preview
| Feature | Status | Description |
|---------|--------|-------------|
| Syntax highlighting | ✅ | Prism.js, 20+ languages |
| Line numbers | ✅ | Clickable navigation |
| Language detection | ✅ | Auto from extension |

### AI Chat
| Feature | Status | Description |
|---------|--------|-------------|
| RAG-powered | ✅ | Context from your files |
| Semantic retrieval | ✅ | Finds relevant documents |
| Streaming (planned) | 🔜 | Real-time typing effect |

---

## CLI Features

### Core Commands
| Command | Aliases | Description |
|---------|---------|-------------|
| `chomp` | eat, index, add | Index a folder |
| `sniff` | search, find, s | Search files |
| `remember` | list, ls | List memories |
| `weight` | stats, info | Show statistics |
| `herd` | sources, folders | List sources |

### Organization
| Command | Aliases | Description |
|---------|---------|-------------|
| `mark` | tag | Add tags to files |
| `twins` | duplicates, dupes | Find duplicates |
| `brain` | ai, organize | AI auto-organize |

### Management
| Command | Aliases | Description |
|---------|---------|-------------|
| `splash` | refresh, reindex | Reindex all |
| `stomp` | remove, rm | Remove source |
| `yawn` | open, reveal | Open in Finder |
| `wade` | watch | Watch changes |
| `den` | config, home | Show config |
| `forget` | reset, clear | Clear all data |

### CLI Experience
| Feature | Status | Description |
|---------|--------|-------------|
| Colored output | ✅ | Beautiful terminal UI |
| Progress spinners | ✅ | Visual feedback |
| Table formatting | ✅ | Clean data display |
| Emoji icons | ✅ | File type indicators |
| Hippo ASCII art | ✅ | Fun branding |

---

## Technical Specifications

### Performance
| Metric | Value |
|--------|-------|
| Index speed | ~20K files/minute |
| Search latency | <50ms (text), <200ms (semantic) |
| Memory usage | ~100MB idle |
| App size | ~50MB |

### File Type Support
- **Images**: jpg, jpeg, png, gif, webp, bmp, tiff, heic, heif, raw, cr2, nef, ico
- **Videos**: mp4, mov, avi, mkv, webm, m4v, wmv, flv
- **Audio**: mp3, wav, flac, m4a, ogg, aac, wma
- **Documents**: pdf, doc, docx, txt, md, rtf, odt, pages
- **Code**: rs, py, js, ts, jsx, tsx, go, java, c, cpp, h, hpp, rb, php, swift, kt, scala, sh, bash, zsh, sql, html, css, scss, sass, json, yaml, yml, toml, xml
- **Data**: csv, tsv, xlsx, xls, numbers
- **Archives**: zip, tar, gz, tgz, 7z, rar, bz2

### Platforms
| Platform | Status |
|----------|--------|
| macOS (ARM) | ✅ Full support |
| macOS (Intel) | ✅ Full support |
| Linux (x64) | ✅ Full support |
| Windows (x64) | ✅ Full support |
| iOS | 🔜 Planned |
| Android | 🔜 Planned |

---

## Upcoming Features

### v0.3.0 (Next)
- [ ] Dark mode theme
- [ ] Streaming AI responses
- [ ] Search history
- [ ] Saved searches / smart folders

### v0.4.0
- [ ] Knowledge graph visualization
- [ ] Timeline view
- [ ] Geographic map for photos
- [ ] Batch file operations

### v0.5.0
- [ ] Google Drive integration
- [ ] iCloud integration
- [ ] Dropbox integration

### v1.0.0
- [ ] iOS app
- [ ] Android app
- [ ] E2E encrypted sync
- [ ] Web app (self-hosted)

---

## Comparison with Alternatives

| Feature | Hippo | Spotlight | Alfred | Everything |
|---------|-------|-----------|--------|------------|
| 100% Local | ✅ | ✅ | ✅ | ✅ |
| AI-Powered | ✅ | ❌ | ❌ | ❌ |
| Semantic Search | ✅ | ❌ | ❌ | ❌ |
| Custom Tags | ✅ | ❌ | ✅ | ❌ |
| Duplicate Detection | ✅ | ❌ | ❌ | ❌ |
| Cross-Platform | ✅ | ❌ | ❌ | ❌ |
| Open Source | ✅ | ❌ | ❌ | ❌ |
| Auto-Tagging | ✅ | ❌ | ❌ | ❌ |
| Code Syntax | ✅ | ❌ | ❌ | ❌ |

---

<p align="center">
  <sub>🦛 Hippo - The Memory That Never Forgets</sub>
</p>
