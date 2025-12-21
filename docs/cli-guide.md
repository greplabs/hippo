---
layout: default
title: CLI Guide
nav_order: 3
description: "Complete reference for Hippo CLI commands - chomp, sniff, twins, brain, and more"
---

# CLI Guide
{: .no_toc }

Master the Hippo command-line interface with hippo-themed commands
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Hippo's CLI uses fun, hippo-themed command names that make file management enjoyable. Each command has both a primary name and aliases for convenience.

```bash
$ hippo --help
Hippo - The Memory That Never Forgets

Usage: hippo <COMMAND>

Commands:
  chomp      Chomp on some files - index a folder
  sniff      Sniff around - search your memories
  remember   Remember things - list your memories
  weight     Check your weight - show index statistics
  herd       Gather the herd - list all sources
  mark       Mark your territory - add tags to files
  twins      Find your twins - detect duplicate files
  brain      Use your brain - AI auto-organize files
  splash     Take a big splash - refresh/reindex all sources
  stomp      Stomp it out - remove a source
  yawn       Open wide - reveal file in finder
  wade       Wade in the water - watch for file changes
  den        Hippo's home - show config locations
  forget     Start fresh - reset the entire index
  help       Print this message or the help of a given subcommand(s)
```

---

## Core Commands

### chomp - Index Files

Add a folder to Hippo's memory.

**Aliases**: `eat`, `index`, `add`

```bash
hippo chomp <PATH>
```

**Examples**:

```bash
# Index your Documents folder
hippo chomp ~/Documents

# Index multiple folders (run separately)
hippo chomp ~/Pictures
hippo chomp ~/Downloads
hippo chomp ~/Desktop

# Index with absolute path
hippo chomp /Users/john/Projects
```

**What it does**:
- Recursively scans the folder
- Extracts metadata from each file
- Stores information in SQLite database
- Generates thumbnails for images and videos
- Parses code files for imports/exports

**Supported file types**: 70+ extensions including:
- Images: jpg, png, gif, webp, heic, raw
- Videos: mp4, mov, avi, mkv, webm
- Audio: mp3, wav, flac, aac, ogg
- Documents: pdf, docx, xlsx, pptx, txt, md
- Code: rs, py, js, go, ts, java, cpp

---

### sniff - Search Files

Search your indexed files.

**Aliases**: `search`, `find`, `s`

```bash
hippo sniff [OPTIONS] <QUERY>
```

**Options**:
- `-t, --tags <TAGS>`: Filter by tags
- `-l, --limit <LIMIT>`: Limit results (default: 20)

**Examples**:

```bash
# Simple search
hippo sniff "vacation"

# Search with tag filter
hippo sniff "photo" --tags beach summer

# Search with custom limit
hippo sniff "report" --limit 50

# Search for specific file types (use tags)
hippo sniff "meeting" --tags video
```

**Search behavior**:
- Searches file names, paths, and tags
- Uses substring matching (case-insensitive)
- Results sorted by relevance
- Displays with colored output and icons

**Output format**:

```
=> Sniffing for "vacation"

🔍 12 memories found:

[1] 📷 IMG_1234.jpg (2.3 MB)
    /Users/john/Pictures/2024/vacation/IMG_1234.jpg
    beach, summer, california

[2] 🎬 video_sunset.mp4 (45.1 MB)
    /Users/john/Videos/vacation/video_sunset.mp4
    beach, sunset
```

---

### remember - List Files

List recently indexed files.

**Aliases**: `list`, `ls`

```bash
hippo remember [OPTIONS]
```

**Options**:
- `-l, --limit <LIMIT>`: Number of files to show (default: 20)
- `-t, --kind <KIND>`: Filter by file type (image, video, audio, code, document)

**Examples**:

```bash
# Show last 20 files
hippo remember

# Show last 50 files
hippo remember --limit 50

# Show only images
hippo remember --kind image

# Show only code files
hippo remember --kind code
```

---

## Source Management

### herd - List Sources

Show all indexed folders.

**Aliases**: `sources`, `folders`

```bash
hippo herd
```

**Output**:

```
=> Gathering the herd...

3 sources:

┌────────────────────────────────────┬───────┐
│ Path                               │ Files │
├────────────────────────────────────┼───────┤
│ /Users/john/Documents              │ -     │
│ /Users/john/Pictures               │ -     │
│ /Users/john/Downloads              │ -     │
└────────────────────────────────────┴───────┘
```

---

### splash - Refresh Index

Re-index all sources to pick up new/changed files.

**Aliases**: `refresh`, `reindex`

```bash
hippo splash
```

**What it does**:
- Re-scans all configured folders
- Updates existing files
- Adds new files
- Removes deleted files from index

**Example**:

```bash
=> Making a big splash - reindexing all sources...

⠋ Reindexing /Users/john/Documents...
✓ Done: /Users/john/Documents

⠋ Reindexing /Users/john/Pictures...
✓ Done: /Users/john/Pictures

✓ All sources refreshed!
```

---

### stomp - Remove Source

Remove a folder from the index.

**Aliases**: `remove`, `rm`

```bash
hippo stomp [OPTIONS] <PATH>
```

**Options**:
- `--delete-memories`: Also delete the file records (not the actual files)

**Examples**:

```bash
# Remove source (keeps memory records)
hippo stomp ~/Downloads

# Remove source and delete records
hippo stomp ~/Downloads --delete-memories
```

**Confirmation prompt**:

```
=> Stomping out /Users/john/Downloads
? Are you sure you want to remove this source? (y/n)
```

---

## Tagging

### mark - Add Tags

Add tags to files.

**Aliases**: `tag`

```bash
hippo mark <TARGET> <TAGS>...
```

**Examples**:

```bash
# Tag by filename
hippo mark "report.pdf" work important

# Tag by search query (tags first result)
hippo mark "vacation" beach summer 2024

# Multiple tags
hippo mark "photo" family reunion california
```

**Tag sources**:
- User tags: Created by you (blue)
- AI tags: Generated by AI (magenta with ✨)
- System tags: Auto-generated (dimmed)

---

## Advanced Features

### twins - Find Duplicates

Detect duplicate files using hash comparison.

**Aliases**: `duplicates`, `dupes`

```bash
hippo twins [OPTIONS]
```

**Options**:
- `-m, --min-size <BYTES>`: Minimum file size to check (default: 1024)

**Examples**:

```bash
# Find all duplicates
hippo twins

# Only check files > 1MB
hippo twins --min-size 1048576

# Only check files > 100KB
hippo twins --min-size 102400
```

**How it works**:
1. Filters files by minimum size
2. Groups files by size
3. Computes SHA-256 hash for each file
4. Groups identical hashes
5. Reports duplicate groups

**Output**:

```
=> Looking for twins (duplicates)...

⠹ Scanning and hashing files...

Found 5 duplicate groups (12 duplicate files):

  💾 234.5 MB wasted by duplicates

Group 1: 3 files × 45.2 MB = 90.4 MB wasted
  Hash: a3f5e8c9d2b1...
  - IMG_1234.jpg
    /Users/john/Pictures/IMG_1234.jpg
  - IMG_1234 copy.jpg
    /Users/john/Desktop/IMG_1234 copy.jpg
  - IMG_1234 (1).jpg
    /Users/john/Downloads/IMG_1234 (1).jpg

Tip: Review duplicates carefully before deleting. Keep the file in the best location.
```

---

### brain - AI Organization

Use AI to automatically tag and organize files.

**Aliases**: `ai`, `organize`, `auto`

```bash
hippo brain [OPTIONS]
```

**Options**:
- `-a, --api-key <KEY>`: Anthropic API key (or set `ANTHROPIC_API_KEY` env var)
- `--dry-run`: Analyze only, don't apply changes

**Examples**:

```bash
# Use with environment variable
export ANTHROPIC_API_KEY=sk-ant-...
hippo brain

# Use with API key argument
hippo brain --api-key sk-ant-...

# Preview without applying
hippo brain --dry-run
```

**What it does**:
1. Finds files with few or no tags (< 3 tags)
2. Sends file metadata to Claude API
3. Receives tag suggestions with confidence scores
4. Applies tags with confidence ≥ 60%

**Output**:

```
=> Activating Hippo's brain...

Found 25 files could use better tagging

Analyzing 10 files with Claude AI...

[1/10] Analyzing vacation_photo.jpg... beach (95%), sunset (88%), california (75%)
    A beautiful sunset photo taken at the beach

[2/10] Analyzing meeting_notes.txt... work (90%), meeting (85%), important (70%)
    Notes from quarterly planning meeting

...

✓ Added 42 tags to files!
```

**Cost consideration**: Each API call costs ~$0.001-0.01 depending on file metadata size.

---

### wade - Watch for Changes

Monitor folders and auto-update the index when files change.

**Aliases**: `watch`

```bash
hippo wade [PATHS]...
```

**Examples**:

```bash
# Watch all indexed sources
hippo wade

# Watch specific paths
hippo wade ~/Documents ~/Downloads

# Watch single path
hippo wade ~/Desktop
```

**What it monitors**:
- New files created
- Files modified
- Files deleted
- Files renamed/moved

**Output**:

```
=> Wading in the water - watching for changes...

  ✓ /Users/john/Documents
  ✓ /Users/john/Pictures

Watching 2 path(s) for changes...
Press Ctrl+C to stop

  → 3 new file(s) indexed
  → 1 file(s) removed
```

**Note**: Press `Ctrl+C` to stop watching.

---

## Information Commands

### weight - Show Statistics

Display index statistics.

**Aliases**: `stats`, `info`

```bash
hippo weight
```

**Output**:

```
    .-"""-.
   /        \
  /_        _\
 // \      / \\
 |\__\    /__/|
  \    ||    /
   \        /
    \  __  /
     '.__.'

 H I P P O 🦛
 The Memory That Never Forgets

=> Checking Hippo's weight...

Index Statistics
  Total Memories: 137,339
  Total Sources: 3
```

---

### den - Show Locations

Show configuration and data file locations.

**Aliases**: `config`, `home`

```bash
hippo den
```

**Output (macOS)**:

```
    .-"""-.
   /        \
  /_        _\
 // \      / \\
 |\__\    /__/|
  \    ||    /
   \        /
    \  __  /
     '.__.'

 H I P P O 🦛
 The Memory That Never Forgets

=> Hippo's Den

Locations:
  Data: /Users/john/Library/Application Support/Hippo
  Config: /Users/john/Library/Application Support/Hippo
  Cache: /Users/john/Library/Caches/Hippo

Database:
  /Users/john/Library/Application Support/Hippo/hippo.db
```

---

### yawn - Open File

Reveal a file in Finder/Explorer.

**Aliases**: `open`, `reveal`

```bash
hippo yawn <TARGET>
```

**Examples**:

```bash
# Open by filename
hippo yawn "report.pdf"

# Open by search query (opens first result)
hippo yawn "vacation photo"
```

**Behavior**:
- **macOS**: Opens Finder and selects the file
- **Linux**: Opens file manager to parent directory
- **Windows**: Opens Explorer and selects the file

---

## Maintenance Commands

### forget - Reset Index

Delete all indexed data and start fresh.

**Aliases**: `reset`, `clear`

```bash
hippo forget [OPTIONS]
```

**Options**:
- `--force`: Skip confirmation prompt

**Examples**:

```bash
# Reset with confirmation
hippo forget

# Reset without confirmation
hippo forget --force
```

**Warning**: This deletes:
- All file records
- All tags
- All source configurations
- Thumbnail cache

**Note**: Original files are NOT deleted, only the index.

---

## Tips and Tricks

### Combining Commands

```bash
# Index, then search
hippo chomp ~/Documents && hippo sniff "report"

# Find duplicates, then review
hippo twins > duplicates.txt && cat duplicates.txt

# Continuous indexing with watch
hippo chomp ~/Downloads && hippo wade ~/Downloads
```

### Shell Aliases

Add to your `.bashrc` or `.zshrc`:

```bash
alias hs='hippo sniff'
alias hc='hippo chomp'
alias hw='hippo weight'
alias hl='hippo remember'
```

### Piping Output

```bash
# Search and save results
hippo sniff "photo" > photos.txt

# Count results
hippo sniff "code" | wc -l

# Filter further
hippo remember | grep ".rs"
```

### Scripting

```bash
#!/bin/bash
# Backup important files found by Hippo

hippo sniff "important" --tags backup | \
  grep -oE "/Users/.*" | \
  xargs -I {} cp {} ~/Backups/
```

### Performance Tips

```bash
# Index large folders in background
hippo chomp ~/BigFolder &

# Limit search results for speed
hippo sniff "query" --limit 10

# Watch specific folders only
hippo wade ~/Documents  # instead of all sources
```

---

## Command Reference Table

| Command | Aliases | Purpose | Key Options |
|---------|---------|---------|-------------|
| `chomp` | eat, index, add | Index a folder | path |
| `sniff` | search, find, s | Search files | --tags, --limit |
| `remember` | list, ls | List recent files | --limit, --kind |
| `weight` | stats, info | Show statistics | - |
| `herd` | sources, folders | List sources | - |
| `mark` | tag | Add tags | target, tags |
| `twins` | duplicates, dupes | Find duplicates | --min-size |
| `brain` | ai, organize | AI tagging | --api-key, --dry-run |
| `splash` | refresh, reindex | Refresh all | - |
| `stomp` | remove, rm | Remove source | --delete-memories |
| `yawn` | open, reveal | Open file | target |
| `wade` | watch | Watch changes | paths |
| `den` | config, home | Show locations | - |
| `forget` | reset, clear | Reset index | --force |

---

## Exit Codes

Hippo uses standard exit codes:

- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `130`: Interrupted (Ctrl+C)

Check exit code in scripts:

```bash
if hippo chomp ~/Documents; then
  echo "Indexing succeeded"
else
  echo "Indexing failed with code $?"
fi
```

---

## Next Steps

- [Desktop App Guide](desktop-app) - Learn the graphical interface
- [API Reference](api) - Programmatic access
- [Architecture](architecture) - Understand internals
- [Contributing](contributing) - Help improve Hippo

---

Need help? Run `hippo --help` or [open an issue](https://github.com/greplabs/hippo/issues).
