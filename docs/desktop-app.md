---
layout: default
title: Desktop App
nav_order: 4
description: "Complete guide to the Hippo Tauri desktop application - features, keyboard shortcuts, and tips"
---

# Desktop App Guide
{: .no_toc }

Master the Hippo desktop application built with Tauri
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

The Hippo desktop app is built with Tauri 2.0, providing a native application experience with a lightweight web-based UI. It combines the performance of Rust with the flexibility of modern web technologies.

**Key Features**:
- Native file system access
- Real-time search with debouncing
- Grid and list view modes
- Thumbnail previews for images and videos
- Tag management
- Keyboard shortcuts
- Cross-platform (macOS, Windows, Linux)

---

## Getting Started

### First Launch

When you first launch Hippo:

1. The app initializes the SQLite database
2. Optionally starts Qdrant for vector search
3. Shows an empty state with "Add Source" button

### Adding Your First Source

1. Click the **"Add Source"** button in the top navigation
2. Select a folder using the native folder picker
3. Wait for indexing to complete (progress shown in UI)
4. Files appear in the grid/list view

**Recommended folders to start**:
- Documents
- Downloads
- Pictures
- Desktop

---

## User Interface

### Main Layout

```
┌─────────────────────────────────────────────────────────────┐
│  🦛 Hippo                    [Add Source] [Settings]  ─ □ x │
├─────────────────────────────────────────────────────────────┤
│  🔍 Search...                           [Grid/List] [Sort]  │
│  [All] [Images] [Videos] [Audio] [Code] [Docs]              │
├─────────────────────────────────────────────────────────────┤
│  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │ 📷   │  │ 📄   │  │ 💻   │  │ 🎵   │  │ 📁   │          │
│  │photo │  │ doc  │  │ code │  │audio │  │folder│          │
│  └──────┘  └──────┘  └──────┘  └──────┘  └──────┘          │
│                                                             │
│  137,000+ files indexed • 3 sources                        │
└─────────────────────────────────────────────────────────────┘
```

### Navigation Bar

**Left Side**:
- Hippo logo and app name
- Status indicators

**Right Side**:
- **Add Source**: Index a new folder
- **Settings**: App preferences (coming soon)
- **Window controls**: Minimize, maximize, close

### Search Bar

Real-time search with 300ms debouncing:

```
┌─────────────────────────────────────────────┐
│ 🔍 Search your memories...                  │
│    vacation photos                          │
│    ↓ Suggestions: beach, summer, california │
└─────────────────────────────────────────────┘
```

**Features**:
- Searches file names, paths, and tags
- Tag suggestions appear below as you type
- Press **Tab** to convert search text to a tag filter
- Press **Esc** to clear search

### Type Filters

Quick filter buttons:

| Filter | Icon | Shows |
|--------|------|-------|
| All | 🔍 | All files |
| Images | 🖼️ | JPG, PNG, GIF, WebP, HEIC, etc. |
| Videos | 🎬 | MP4, MOV, AVI, MKV, WebM |
| Audio | 🎵 | MP3, WAV, FLAC, AAC, OGG |
| Code | 💻 | Rust, Python, JS, Go, etc. |
| Docs | 📄 | PDF, DOCX, XLSX, TXT, MD |

**Usage**: Click a filter to toggle it. Active filters are highlighted.

### Sort Options

Dropdown menu with sorting options:

- **Newest First**: Sort by modified date (descending)
- **Oldest First**: Sort by modified date (ascending)
- **Name A-Z**: Alphabetical by filename
- **Name Z-A**: Reverse alphabetical
- **Largest First**: Sort by file size (descending)
- **Smallest First**: Sort by file size (ascending)

---

## View Modes

### Grid View

Default view showing thumbnail cards.

```
┌─────────────────────────────────────────────────────┐
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐    │
│  │ [IMG]  │  │ [IMG]  │  │ [IMG]  │  │ [IMG]  │    │
│  │        │  │        │  │        │  │        │    │
│  │photo.jpg│ │doc.pdf │ │code.rs │ │song.mp3│    │
│  │ 2.3 MB │  │ 156 KB │  │ 4 KB   │  │ 8.1 MB │    │
│  │beach    │  │work    │  │rust    │  │music   │    │
│  └────────┘  └────────┘  └────────┘  └────────┘    │
└─────────────────────────────────────────────────────┘
```

**Card contents**:
- Thumbnail (for images/videos)
- File type icon (for other files)
- Filename (truncated if long)
- File size
- Tags (first few tags)

**Interaction**:
- **Click**: Open detail panel
- **Double-click**: Open file with default app
- **Right-click**: Context menu (coming soon)

### List View

Compact table view.

```
┌────────────────────────────────────────────────────────────┐
│ Name         │ Type      │ Size    │ Modified   │ Tags     │
├──────────────┼───────────┼─────────┼────────────┼──────────┤
│ photo.jpg    │ 🖼️ Image  │ 2.3 MB  │ 2024-12-01 │ beach    │
│ document.pdf │ 📄 Doc    │ 156 KB  │ 2024-11-28 │ work     │
│ script.rs    │ 💻 Rust   │ 4 KB    │ 2024-12-05 │ code     │
│ song.mp3     │ 🎵 Audio  │ 8.1 MB  │ 2024-11-15 │ music    │
└────────────────────────────────────────────────────────────┘
```

**Columns**:
- Name with icon
- File type
- Size (formatted)
- Last modified date
- Tags (comma-separated)

**Interaction**:
- **Click row**: Open detail panel
- **Double-click row**: Open file
- **Click header**: Sort by that column (coming soon)

---

## Detail Panel

Click any file to open the detail panel.

```
┌────────────────────────────────────────┐
│  Photo_2024.jpg                    ✕  │
├────────────────────────────────────────┤
│  ┌──────────────────────────────────┐  │
│  │                                  │  │
│  │        [Thumbnail Preview]       │  │
│  │                                  │  │
│  └──────────────────────────────────┘  │
│                                        │
│  📷 Image • 2.3 MB                     │
│  📅 Modified: Dec 1, 2024, 3:45 PM     │
│  📁 /Users/john/Pictures/2024/         │
│                                        │
│  🏷️ Tags:                              │
│  ┌─────┐ ┌────────┐ ┌──────────┐      │
│  │beach│ │summer  │ │california│      │
│  └─────┘ └────────┘ └──────────┘      │
│                                        │
│  + Add tag...                          │
│                                        │
│  [Open File] [Reveal in Finder]        │
│                                        │
│  Metadata:                             │
│  • Dimensions: 4032 × 3024             │
│  • Camera: iPhone 14 Pro               │
│  • ISO: 64 • f/1.78 • 1/120s           │
│  • GPS: 34.0522°N, 118.2437°W          │
└────────────────────────────────────────┘
```

### Panel Sections

**Header**:
- Filename
- Close button (X)

**Preview**:
- Image thumbnail (256x256)
- Video thumbnail (first frame)
- File type icon (for non-visual files)

**Basic Info**:
- File type with icon
- File size (formatted)
- Last modified date/time
- Full file path (click to copy)

**Tags**:
- Existing tags as colored pills
  - User tags: Blue
  - AI tags: Magenta with ✨
  - System tags: Gray
- "Add tag" input field
- Press Enter to add a tag

**Actions**:
- **Open File**: Open with default application
- **Reveal in Finder**: Show in file manager
- **Delete**: Remove from index (coming soon)

**Metadata** (if available):
- **Images**: Dimensions, camera, EXIF data, GPS
- **Videos**: Duration, resolution, codec
- **Audio**: Duration, bitrate, artist, album
- **Code**: Language, lines, imports
- **Documents**: Page count, author, format

---

## Tagging

### Adding Tags

**Method 1: Detail Panel**
1. Click a file to open detail panel
2. Click "+ Add tag..." input
3. Type tag name
4. Press **Enter** to add

**Method 2: Bulk Tagging** (coming soon)
1. Select multiple files (Ctrl+Click or Shift+Click)
2. Click "Bulk Tag" button
3. Enter tag names
4. Apply to all selected

### Tag Sources

Tags have different colors based on their source:

| Source | Color | Icon | Example |
|--------|-------|------|---------|
| User | Blue | - | `work` |
| AI | Magenta | ✨ | `beach✨` |
| System | Gray | - | `image` |
| Imported | Green | - | `legacy` |

### Tag Filtering

**In Search Bar**:
1. Type to search
2. Tag suggestions appear below
3. Press **Tab** to add tag as filter
4. Press **Tab** again for next suggestion

**Active Tag Filters**:
- Shown as pills below search bar
- Click **X** on pill to remove filter
- Click pill to toggle include/exclude

---

## Keyboard Shortcuts

### Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘K` or `Ctrl+K` | Focus search bar |
| `⌘F` or `Ctrl+F` | Focus search bar |
| `Esc` | Clear search / Close detail panel |
| `⌘W` or `Ctrl+W` | Close window |
| `⌘Q` or `Ctrl+Q` | Quit application |

### Search & Navigation

| Shortcut | Action |
|----------|--------|
| `Tab` | Add tag suggestion as filter |
| `Enter` | Execute search |
| `↑` `↓` | Navigate results (coming soon) |
| `⌘↵` | Open selected file (coming soon) |

### View Controls

| Shortcut | Action |
|----------|--------|
| `⌘1` | Show all files |
| `⌘2` | Show images only |
| `⌘3` | Show videos only |
| `⌘4` | Show audio only |
| `⌘5` | Show code only |
| `⌘6` | Show documents only |

### File Actions

| Shortcut | Action |
|----------|--------|
| `Space` | Quick preview (coming soon) |
| `⌘O` | Open file |
| `⌘R` | Reveal in Finder |
| `⌘T` | Add tag (in detail panel) |
| `⌘Backspace` | Delete from index (coming soon) |

---

## Features

### Real-time Search

Search updates automatically as you type with 300ms debouncing.

**How it works**:
1. User types in search bar
2. Wait 300ms after last keystroke
3. Send search request to backend
4. Update results in UI
5. Show loading indicator during search

**Performance**:
- Searches 100K+ files in <50ms
- Uses SQLite FTS5 for full-text search
- Client-side filtering for types and tags

### Thumbnail Generation

Hippo generates thumbnails for visual files:

**Images**:
- 256x256 JPEG thumbnails
- Cached on disk for fast loading
- Smart invalidation when file changes

**Videos**:
- Extracts first frame using `ffmpeg`
- Graceful fallback to icon if `ffmpeg` unavailable
- Cached like images

**Cache location**:
- macOS: `~/Library/Caches/Hippo/thumbnails/`
- Linux: `~/.cache/Hippo/thumbnails/`
- Windows: `%LOCALAPPDATA%\Hippo\cache\thumbnails\`

### Source Management

Manage indexed folders from the UI.

**Add Source**:
1. Click "Add Source" button
2. Select folder in native dialog
3. Indexing starts automatically
4. Progress shown in UI (updates every 2s for 40s)

**Remove Source**:
1. Open Sources panel (coming soon)
2. Click "Remove" next to source
3. Choose to keep or delete memories
4. Confirm removal

**Re-index Source**:
1. Open Sources panel
2. Click "Re-index" next to source
3. Wait for completion

**Auto-refresh**:
- UI refreshes every 2 seconds during indexing
- Shows count of indexed files
- Stops refreshing after 40 seconds

---

## Settings

### Current Settings

Settings are stored in SQLite and include:
- Indexed folder paths
- Window size and position (OS managed)
- View mode preference (persisted in localStorage)

### Planned Settings

Coming soon:
- Dark/light mode toggle
- Thumbnail size
- Search result limit
- Auto-watch folders on startup
- Exclude patterns (e.g., `node_modules`)
- AI provider selection (Ollama vs Claude)

---

## Performance Tips

### Indexing Performance

**Start small**:
- Index one folder at a time
- Start with smaller folders (Documents)
- Then add larger ones (Pictures)

**Exclude unnecessary files**:
- Avoid indexing `node_modules`, `.git`, etc.
- Use `.hippoignore` file (coming soon)

**Close other apps**:
- Free up CPU for indexing
- Indexing is CPU-intensive

### Search Performance

**Use specific queries**:
- `vacation 2024` is better than `photo`
- Narrow results with type filters
- Use tag filters for precision

**Limit results**:
- Default limit is 1000 files
- Adjust in settings (coming soon)

### Memory Usage

**Typical usage**:
- Idle: ~100MB
- Indexing: ~200-500MB
- Searching: ~150MB

**Reduce memory**:
- Close unused sources
- Clear thumbnail cache
- Restart app periodically

---

## Troubleshooting

### App Won't Start

**macOS**: Check security settings
```bash
# Allow app to run
xattr -cr /path/to/Hippo.app
```

**Linux**: Check permissions
```bash
chmod +x hippo-tauri
```

**Windows**: Run as administrator if needed

### Slow Search

**Check database size**:
```bash
# macOS
du -sh ~/Library/Application\ Support/Hippo/hippo.db
```

**Optimize database**:
- Reset index: `hippo forget --force`
- Vacuum: Automatically done on startup

### Missing Thumbnails

**Check cache**:
```bash
# macOS
ls ~/Library/Caches/Hippo/thumbnails/
```

**Regenerate**:
- Re-index the source
- Or delete cache and re-open files

### Files Not Appearing

**Check if folder is indexed**:
1. Look in Sources panel
2. Verify path is correct

**Re-index source**:
1. Remove source
2. Add source again

**Check file type**:
- Only supported file types are indexed
- See [Architecture](architecture#supported-file-types)

---

## Tauri Commands

For developers: Available IPC commands.

| Command | Parameters | Returns | Purpose |
|---------|------------|---------|---------|
| `initialize` | - | Status | Initialize Hippo instance |
| `search` | query, tags | SearchResults | Search files |
| `add_source` | sourceType, path | Status | Add folder to index |
| `remove_source` | path, deleteFiles | Status | Remove source |
| `reindex_source` | path | Status | Re-index folder |
| `get_sources` | - | Source[] | List indexed folders |
| `get_stats` | - | Stats | Get index statistics |
| `get_tags` | - | Tag[] | List all tags |
| `add_tag` | memoryId, tag | Status | Add tag to file |
| `bulk_add_tag` | memoryIds, tag | Status | Add tag to multiple files |
| `bulk_delete` | memoryIds | Status | Delete multiple files |
| `reset_index` | - | Status | Clear all data |
| `open_file` | path | Status | Open with default app |
| `open_in_finder` | path | Status | Reveal in file manager |

**Usage example**:

```javascript
// JavaScript in UI
const results = await window.__TAURI__.core.invoke('search', {
  query: 'vacation',
  tags: ['beach', 'summer']
});

console.log(results); // SearchResults object
```

See [API Reference](api) for full documentation.

---

## Advanced Usage

### Custom Styling

The UI is in a single HTML file: `hippo-tauri/ui/dist/index.html`

Modify CSS in the `<style>` section to customize:
- Colors
- Fonts
- Layout
- Animations

### Extending Commands

Add new Tauri commands:

1. Define command in `hippo-tauri/src/main.rs`:
```rust
#[tauri::command]
async fn my_command(state: State<'_, AppState>) -> Result<String, String> {
    // Your code here
    Ok("Success".to_string())
}
```

2. Register in builder:
```rust
.invoke_handler(tauri::generate_handler![
    initialize,
    search,
    // ... existing commands
    my_command,
])
```

3. Call from UI:
```javascript
const result = await invoke('my_command');
```

---

## Next Steps

- [CLI Guide](cli-guide) - Command-line interface
- [API Reference](api) - Programmatic access
- [Architecture](architecture) - How it works
- [Contributing](contributing) - Help improve Hippo

---

Have questions? [Open an issue](https://github.com/greplabs/hippo/issues) on GitHub.
