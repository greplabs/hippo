# Hippo CLI Tutorial

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

    H I P P O
    The Memory That Never Forgets
```

## Quick Start

```bash
# Build the CLI
cargo build --release -p hippo-cli

# Or run directly
cargo run -p hippo-cli -- --help
```

---

## Commands Overview

| Command | Aliases | Description |
|---------|---------|-------------|
| `chomp` | `eat`, `index`, `add` | Index a folder into Hippo's memory |
| `sniff` | `search`, `find`, `s` | Search your memories |
| `remember` | `list`, `ls` | List your memories |
| `weight` | `stats`, `info` | Show index statistics |
| `herd` | `sources`, `folders` | List all indexed sources |
| `mark` | `tag` | Add tags to files |
| `twins` | `duplicates`, `dupes` | Find duplicate files |
| `brain` | `ai`, `organize`, `auto` | AI auto-organize and tag |
| `splash` | `refresh`, `reindex` | Reindex all sources |
| `stomp` | `remove`, `rm` | Remove a source |
| `yawn` | `open`, `reveal` | Open file in Finder/Explorer |
| `wade` | `watch` | Watch for file changes |
| `den` | `config`, `home` | Show config locations |
| `forget` | `reset`, `clear` | Reset the entire index |

---

## Detailed Command Examples

### 1. `chomp` - Index Files

Index a folder to add files to Hippo's memory:

```bash
# Index your Documents folder
$ hippo chomp ~/Documents

=> Chomping on /Users/you/Documents
⠋ Indexing files...
✓ Added /Users/you/Documents to Hippo's memory
• Total memories: 1,247
```

```bash
# Index multiple folders
$ hippo chomp ~/Photos
$ hippo chomp ~/Projects
$ hippo chomp ~/Downloads
```

**Output:**
```
=> Chomping on /Users/you/Photos
⠋ Indexing files...
✓ Added /Users/you/Photos to Hippo's memory
• Total memories: 15,892
```

---

### 2. `sniff` - Search Files

Search through your indexed files:

```bash
# Basic search
$ hippo sniff "vacation"

=> Sniffing for "vacation"

🔍 12 memories found:

[1] 🖼️  beach_sunset.jpg (2.4 MB)
    /Users/you/Photos/2024/vacation/beach_sunset.jpg
    vacation, beach, sunset✨

[2] 🖼️  family_dinner.jpg (1.8 MB)
    /Users/you/Photos/2024/vacation/family_dinner.jpg
    vacation, family

[3] 📄 travel_itinerary.pdf (156.2 KB)
    /Users/you/Documents/Travel/travel_itinerary.pdf
    vacation, travel, planning
```

```bash
# Search with tag filters
$ hippo sniff "project" --tags work --tags 2024

# Limit results
$ hippo sniff "photos" --limit 50
```

---

### 3. `remember` - List Memories

View your indexed files:

```bash
$ hippo remember

=> Remembering...

📚 20 memories:

[1] 💻 main.rs (4.2 KB)
    /Users/you/Projects/hippo/src/main.rs
    rust, code

[2] 🖼️  screenshot.png (892.3 KB)
    /Users/you/Desktop/screenshot.png
    screenshot

[3] 📄 README.md (12.1 KB)
    /Users/you/Projects/hippo/README.md
    documentation, markdown
```

```bash
# Show more results
$ hippo remember --limit 100

# Filter by type (coming soon)
$ hippo remember --kind image
$ hippo remember --kind code
```

---

### 4. `weight` - Index Statistics

Check how much Hippo remembers:

```bash
$ hippo weight

    .-"""-.
   /        \
  /_        _\
 // \      / \\
 |\__\    /__/|
  \    ||    /
   \        /
    \  __  /
     '.__.'

    HIPPO 🦛
    The Memory That Never Forgets

=> Checking Hippo's weight...

Index Statistics
  Total Memories: 15,892
  Total Sources: 4
```

---

### 5. `herd` - List Sources

See which folders are indexed:

```bash
$ hippo herd

=> Gathering the herd...

4 sources:

+----------------------------------------+-------+
| Path                                   | Files |
+----------------------------------------+-------+
| /Users/you/Documents                   | -     |
| /Users/you/Photos                      | -     |
| /Users/you/Projects                    | -     |
| /Users/you/Downloads                   | -     |
+----------------------------------------+-------+
```

---

### 6. `mark` - Add Tags

Tag your files for better organization:

```bash
$ hippo mark beach_sunset.jpg vacation summer favorite

=> Marking beach_sunset.jpg with tags
✓ Added tag: vacation
✓ Added tag: summer
✓ Added tag: favorite
```

```bash
# Tag by search
$ hippo mark "project report" work important 2024
```

---

### 7. `twins` - Find Duplicates

Discover duplicate files wasting disk space:

```bash
$ hippo twins

=> Looking for twins (duplicates)...
⠋ Scanning and hashing files...

Found 23 duplicate groups (89 duplicate files):

  💾 1.2 GB wasted by duplicates

Group 1: 3 files × 45.2 MB = 90.4 MB wasted
  Hash: 8a7f3b2c9d1e4f5a
  - vacation_photo.jpg
    /Users/you/Photos/Backup/vacation_photo.jpg
  - vacation_photo.jpg
    /Users/you/Photos/2024/vacation_photo.jpg
  - vacation_photo_copy.jpg
    /Users/you/Desktop/vacation_photo_copy.jpg

Group 2: 2 files × 12.8 MB = 12.8 MB wasted
  Hash: 3c4d5e6f7a8b9c0d
  - presentation.pptx
    /Users/you/Documents/Work/presentation.pptx
  - presentation_backup.pptx
    /Users/you/Documents/Backup/presentation_backup.pptx

Tip: Review duplicates carefully before deleting. Keep the file in the best location.
```

```bash
# Only check files larger than 1MB
$ hippo twins --min-size 1048576
```

---

### 8. `brain` - AI Auto-Organize

Let AI analyze and tag your files (requires Anthropic API key):

```bash
$ export ANTHROPIC_API_KEY="your-key-here"
$ hippo brain

=> Activating Hippo's brain...

Found 47 files could use better tagging

Analyzing 10 files with Claude AI...

[1/10] Analyzing quarterly_report.pdf... finance (85%), report (92%), Q3 (78%)
    Financial report for Q3 2024 with revenue analysis
[2/10] Analyzing team_photo.jpg... team (90%), office (75%), event (82%)
    Group photo at company event
[3/10] Analyzing server_config.yaml... devops (95%), kubernetes (88%), config (90%)
    Kubernetes deployment configuration

✓ Added 28 tags to files!
```

```bash
# Dry run - see suggestions without applying
$ hippo brain --dry-run

# With API key inline
$ hippo brain --api-key "sk-ant-..."
```

---

### 9. `splash` - Reindex Sources

Refresh all indexed sources:

```bash
$ hippo splash

=> Making a big splash - reindexing all sources...
⠋ Reindexing /Users/you/Documents...
Done: /Users/you/Documents
⠋ Reindexing /Users/you/Photos...
Done: /Users/you/Photos
✓ All sources refreshed!
```

---

### 10. `stomp` - Remove Source

Remove a folder from the index:

```bash
$ hippo stomp ~/Downloads

=> Stomping out /Users/you/Downloads
? Are you sure you want to remove this source? (y/N) y
✓ Source removed!
```

```bash
# Also delete the memories (not actual files)
$ hippo stomp ~/Downloads --delete-memories
```

---

### 11. `yawn` - Reveal in Finder

Open a file in Finder/Explorer:

```bash
$ hippo yawn "quarterly report"

• Opening /Users/you/Documents/quarterly_report.pdf...
```

---

### 12. `wade` - Watch for Changes

Monitor folders for real-time indexing:

```bash
$ hippo wade

=> Wading in the water - watching for changes...
  ✓ /Users/you/Documents
  ✓ /Users/you/Photos

Watching 2 path(s) for changes...
Press Ctrl+C to stop

  → 3 new file(s) indexed
  → 1 file(s) removed
  → 2 new file(s) indexed
^C

Stopping watchers...
✓ Stopped watching
```

```bash
# Watch specific paths
$ hippo wade ~/Documents ~/Projects
```

---

### 13. `den` - Show Config Locations

Find where Hippo stores its data:

```bash
$ hippo den

    .-"""-.
   /        \
  /_        _\
 // \      / \\
 |\__\    /__/|
  \    ||    /
   \        /
    \  __  /
     '.__.'

    HIPPO 🦛
    The Memory That Never Forgets

=> Hippo's Den

Locations:
  Data: /Users/you/Library/Application Support/Hippo
  Config: /Users/you/Library/Application Support/Hippo
  Cache: /Users/you/Library/Caches/Hippo

Database:
  /Users/you/Library/Application Support/Hippo/hippo.db
```

---

### 14. `forget` - Reset Index

Start fresh by clearing all data:

```bash
$ hippo forget

=> Forgetting everything...
? This will delete ALL indexed data. Are you sure? (y/N) y
✓ Index reset. Hippo's memory is now clean.
```

```bash
# Skip confirmation
$ hippo forget --force
```

---

## Pro Tips

### 1. Use Aliases for Speed

```bash
# Add to your ~/.zshrc or ~/.bashrc
alias hs="hippo sniff"      # Quick search
alias hl="hippo remember"   # List files
alias hi="hippo chomp"      # Index folder
alias hw="hippo weight"     # Check stats
```

### 2. Combine with Other Tools

```bash
# Find duplicates and review with fzf
hippo twins | fzf

# Search and open in VS Code
hippo sniff "config" | head -1 | xargs code

# Export search results
hippo sniff "photos" --limit 1000 > photo_list.txt
```

### 3. Automate Indexing

```bash
# Add to crontab for nightly reindex
0 2 * * * /path/to/hippo splash >> /var/log/hippo.log 2>&1

# Or use launchd on macOS
```

### 4. Watch During Development

```bash
# Keep Hippo watching while you code
hippo wade ~/Projects &
```

---

## File Type Support

Hippo recognizes 70+ file types:

| Category | Extensions |
|----------|------------|
| **Images** | jpg, jpeg, png, gif, webp, bmp, tiff, heic, raw, cr2, nef |
| **Videos** | mp4, mov, avi, mkv, webm, m4v |
| **Audio** | mp3, wav, flac, m4a, ogg, aac |
| **Documents** | pdf, doc, docx, txt, md, rtf, odt |
| **Code** | rs, py, js, ts, jsx, tsx, go, java, c, cpp, rb, php, swift |
| **Data** | json, yaml, toml, xml, csv |
| **Archives** | zip, tar, gz, 7z, rar |

---

## Troubleshooting

### "No memories found"

```bash
# Check if you have indexed any folders
hippo herd

# If empty, index a folder first
hippo chomp ~/Documents
```

### "Permission denied"

```bash
# Make sure you have read access to the folder
ls -la ~/Documents

# Try with sudo for system folders (not recommended)
```

### "Database locked"

```bash
# Only one Hippo instance can run at a time
# Close the desktop app if it's running
pkill -f hippo-tauri

# Then try your CLI command again
```

### API Key Issues

```bash
# Set the API key properly
export ANTHROPIC_API_KEY="sk-ant-your-key"

# Or pass it directly
hippo brain --api-key "sk-ant-your-key"
```

---

## What's Next?

- **Semantic Search**: `hippo sniff "things similar to vacation"` (coming soon)
- **Smart Folders**: Auto-organizing virtual folders
- **Sync**: Cloud backup with E2E encryption
- **Mobile**: iOS/Android companion app

---

<p align="center">
  <sub>🦛 Hippo CLI - Built with Rust for speed and reliability</sub>
</p>
