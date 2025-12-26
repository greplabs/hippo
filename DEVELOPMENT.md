# Hippo Development Guide

> Parallel development tracking for making Hippo legendary

## Branch Strategy

```
main (stable)
├── feature/phase1-visual-polish      # Icons, themes, UI refinements
├── feature/phase2-smart-tagging      # Tagging, auto-tag, organization
├── feature/phase3-advanced-search    # NLP, filters, discovery
├── feature/phase4-automation         # Rules, workflows, AI assistants
└── feature/phase5-platform           # Cross-platform, integrations
```

## Quick Commands

```bash
# Switch to a phase
git checkout feature/phase1-visual-polish

# Update from main
git fetch origin && git rebase origin/main

# Create PR for phase work
gh pr create --base main --title "feat(phase1): Description"
```

---

## Phase 1: Visual Polish & Consistency

**Branch**: `feature/phase1-visual-polish`
**Status**: 🟡 In Progress
**Priority**: HIGH

### Context & Memory

This phase focuses on making the UI beautiful and consistent. The app already works well functionally - now we make it look professional.

### Current State
- App icons: ✅ Good hippo mascot (128x128, 256x256, icns, ico)
- UI icons: ⚠️ Using emoji, need SVG icons
- Theme: ⚠️ Light mode only, dark mode CSS exists but incomplete
- Typography: ⚠️ Using system fonts, could be more refined

### Key Files
| File | Purpose |
|------|---------|
| `hippo-tauri/ui/dist/index.html` | Main UI (all CSS/JS inline) |
| `hippo-tauri/icons/` | App icons |
| `assets/` | Brand assets, SVGs |
| `docs/BRAND_GUIDE.md` | Color palette, typography |

### Tasks
- [ ] Create unified SVG icon set (24x24)
- [ ] Implement dark mode toggle
- [ ] Add CSS variables for theming
- [ ] Create loading skeletons
- [ ] Design empty state illustrations
- [ ] Refine typography scale

### Design Tokens
```css
/* Colors - from BRAND_GUIDE.md */
--color-primary: #6366F1;      /* Indigo */
--color-secondary: #8B5CF6;    /* Purple */
--color-accent: #F59E0B;       /* Amber */
--color-success: #10B981;      /* Emerald */
--color-warning: #F59E0B;      /* Amber */
--color-error: #EF4444;        /* Red */

/* Dark mode */
--color-bg-dark: #1F2937;
--color-surface-dark: #374151;
--color-text-dark: #F9FAFB;
```

---

## Phase 2: Smart Tagging & Organization

**Branch**: `feature/phase2-smart-tagging`
**Status**: 🟡 In Progress
**Priority**: HIGH

### Context & Memory

Tagging is core to Hippo's value prop. We need to make it smart, fast, and intuitive.

### Current State
- Basic tagging: ✅ Add/remove tags works
- Tag storage: ✅ SQLite with counts
- Auto-tagging: ✅ Ollama integration exists
- Hierarchical tags: ❌ Not implemented
- Tag UI: ⚠️ Basic, needs polish

### Key Files
| File | Purpose |
|------|---------|
| `hippo-core/src/models.rs` | Tag struct definition |
| `hippo-core/src/storage/mod.rs` | Tag CRUD operations |
| `hippo-core/src/ollama/mod.rs` | AI tagging |
| `hippo-core/src/organization/mod.rs` | Auto-organize logic |

### Tasks
- [ ] Implement hierarchical tag model (`parent/child`)
- [ ] Add tag colors and icons
- [ ] Create tag autocomplete API
- [ ] Build bulk tagging UI
- [ ] Improve AI auto-tagging prompts
- [ ] Add confidence scores to AI tags
- [ ] Implement smart folders

### Data Model Updates
```rust
// Enhanced Tag model
pub struct Tag {
    pub name: String,
    pub parent: Option<String>,      // Hierarchical
    pub color: Option<String>,       // Hex color
    pub icon: Option<String>,        // SVG icon name
    pub kind: TagKind,
    pub confidence: u8,              // 0-100 for AI tags
    pub created_at: DateTime<Utc>,
}

// Smart Folder
pub struct SmartFolder {
    pub id: Uuid,
    pub name: String,
    pub query: SearchQuery,          // Saved search
    pub icon: Option<String>,
    pub color: Option<String>,
}
```

---

## Phase 3: Advanced Search & Discovery

**Branch**: `feature/phase3-advanced-search`
**Status**: 🟡 In Progress
**Priority**: HIGH

### Context & Memory

Search is Hippo's superpower. We need natural language, operators, and discovery features.

### Current State
- Text search: ✅ Fast SQL-based
- Semantic search: ✅ Qdrant vectors
- Fuzzy search: ✅ Levenshtein
- NLP parsing: ⚠️ Basic date detection
- Operators: ❌ Not implemented
- Saved searches: ❌ Not implemented

### Key Files
| File | Purpose |
|------|---------|
| `hippo-core/src/search/mod.rs` | Search engine |
| `hippo-core/src/search/advanced_filter.rs` | Filter logic |
| `hippo-core/src/models.rs` | SearchQuery struct |

### Tasks
- [ ] Implement search operators (AND, OR, NOT)
- [ ] Add quoted exact match
- [ ] Build NLP date parser ("last week", "in 2024")
- [ ] Create date range picker component
- [ ] Add file size filters
- [ ] Implement saved searches
- [ ] Build search history
- [ ] Add "similar files" button
- [ ] Create timeline view

### Search Syntax
```
# Operators
photos AND vacation        # Both terms
photos OR videos           # Either term
photos NOT selfie          # Exclude term
"exact phrase"             # Exact match

# Filters
type:image                 # File type
size:>10mb                 # Size filter
date:2024                  # Year
date:last-week             # Relative
tag:vacation               # Has tag
-tag:work                  # Exclude tag

# Combined
type:image date:last-month tag:family NOT screenshot
```

---

## Phase 4: Smart Automation

**Branch**: `feature/phase4-automation`
**Status**: 🟡 In Progress
**Priority**: MEDIUM

### Context & Memory

Automation makes Hippo work for you. Rules, workflows, and AI assistants.

### Current State
- File watcher: ✅ Real-time with notify crate
- Background indexing: ✅ Works
- Scheduled tasks: ❌ Not implemented
- Rules engine: ❌ Not implemented
- AI streaming: ❌ Not implemented

### Key Files
| File | Purpose |
|------|---------|
| `hippo-core/src/watcher/mod.rs` | File watching |
| `hippo-core/src/indexer/mod.rs` | Indexing logic |
| `hippo-core/src/ollama/mod.rs` | AI integration |
| `hippo-core/src/organization/mod.rs` | Organization |

### Tasks
- [ ] Design automation rule schema
- [ ] Implement rule evaluation engine
- [ ] Add scheduled task runner
- [ ] Create rule builder UI
- [ ] Implement streaming AI responses
- [ ] Add natural language commands
- [ ] Build notification system

### Rule Schema
```rust
pub struct AutomationRule {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub trigger: RuleTrigger,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

pub enum RuleTrigger {
    FileCreated,
    FileModified,
    Schedule(CronExpr),
    Manual,
}

pub enum Action {
    AddTag(String),
    MoveToFolder(PathBuf),
    RunAiAnalysis,
    SendNotification(String),
    RunCommand(String),
}
```

---

## Phase 5: Platform & Integrations

**Branch**: `feature/phase5-platform`
**Status**: 🟡 In Progress
**Priority**: MEDIUM

### Context & Memory

Cross-platform support and integrations to make Hippo universally useful.

### Current State
- macOS: ✅ Full support, app built
- Windows: ⚠️ Should work, not tested
- Linux: ⚠️ Should work, not tested
- System tray: ❌ Not implemented
- Global hotkey: ❌ Not implemented
- Cloud sync: ❌ Not implemented

### Key Files
| File | Purpose |
|------|---------|
| `hippo-tauri/tauri.conf.json` | App config |
| `hippo-tauri/src/main.rs` | Tauri commands |
| `hippo-core/src/sources/mod.rs` | Source connectors |

### Tasks
- [ ] Test Windows build
- [ ] Test Linux build
- [ ] Add system tray mode
- [ ] Implement global hotkey
- [ ] Add menu bar mini-mode
- [ ] Design cloud sync architecture
- [ ] Implement Google Drive connector

---

## Code Quality Standards

### Performance Guidelines

```rust
// DO: Use async for I/O operations
async fn load_file(path: &Path) -> Result<Vec<u8>> {
    tokio::fs::read(path).await.map_err(Into::into)
}

// DO: Use channels for background work
let (tx, rx) = mpsc::channel(100);
tokio::spawn(async move {
    while let Some(task) = rx.recv().await {
        process(task).await;
    }
});

// DO: Use caching for expensive operations
static CACHE: Lazy<Cache<String, Value>> = Lazy::new(|| {
    Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300))
        .build()
});

// DO: Batch database operations
async fn insert_batch(items: Vec<Item>) -> Result<()> {
    let mut tx = db.begin().await?;
    for item in items {
        tx.execute("INSERT INTO items ...", &[&item]).await?;
    }
    tx.commit().await
}
```

### Error Handling

```rust
// Use thiserror for library errors
#[derive(Debug, thiserror::Error)]
pub enum HippoError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("AI service unavailable")]
    AiUnavailable,
}

// Propagate with context
async fn index_file(path: &Path) -> Result<Memory> {
    let content = tokio::fs::read(path)
        .await
        .map_err(|e| HippoError::Io {
            path: path.to_owned(),
            source: e
        })?;
    // ...
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_returns_results() {
        let storage = Storage::in_memory().await.unwrap();
        storage.insert_test_data().await;

        let results = storage.search("test").await.unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_tag_parsing() {
        let tag = Tag::parse("photos/vacation/beach").unwrap();
        assert_eq!(tag.name, "beach");
        assert_eq!(tag.parent, Some("photos/vacation".to_string()));
    }
}
```

---

## Git Workflow

### Commit Messages

```
feat(phase1): Add dark mode toggle
fix(search): Handle empty query gracefully
perf(indexer): Batch database inserts
docs: Update architecture diagram
test(storage): Add tag hierarchy tests
chore: Update dependencies
```

### PR Template

```markdown
## Summary
Brief description of changes

## Phase
- [ ] Phase 1: Visual Polish
- [ ] Phase 2: Smart Tagging
- [ ] Phase 3: Advanced Search
- [ ] Phase 4: Automation
- [ ] Phase 5: Platform

## Changes
- Change 1
- Change 2

## Testing
- [ ] Unit tests pass
- [ ] Manual testing done
- [ ] Performance verified
```

---

## Context Preservation

### Session Handoff Template

When ending a session, update this section in CLAUDE.md:

```markdown
### Session Handoff

**Date**: YYYY-MM-DD
**Phases Worked On**: Phase 1, Phase 2

**What Was Done**:
- Implemented feature X
- Fixed bug Y

**In Progress**:
- Working on Z (70% complete)
- Branch: feature/phase1-visual-polish

**Next Steps**:
1. Complete Z
2. Start A

**Blockers**:
- None / Describe blocker

**Key Decisions Made**:
- Chose approach X because Y
```

---

## Quick Reference

### Build Commands

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt --all
```

### File Locations

| What | Where |
|------|-------|
| App icons | `hippo-tauri/icons/` |
| UI code | `hippo-tauri/ui/dist/index.html` |
| Core library | `hippo-core/src/` |
| CLI | `hippo-cli/src/` |
| Tests | `hippo-core/tests/` |
| Docs | `docs/` |
| Brand assets | `assets/` |

---

*Last updated: December 25, 2025*
