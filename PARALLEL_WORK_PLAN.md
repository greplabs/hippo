# Parallel Development Plan: Phase 1 & 2 Features

This document contains detailed implementation plans for 4 parallel workstreams.

---

## Workstream 1: Loading States & Empty States (Phase 1)
**Branch**: `feature/loading-empty-states`
**PR Title**: `feat(ui): Add loading skeletons and empty state illustrations`

### Tasks

#### 1.1 Search Loading States
**File**: `hippo-tauri/ui/dist/index.html`

Current skeleton CSS already exists (lines ~80-100). Implement:

1. **Search skeleton grid** - When searching, show skeleton cards:
```javascript
// In performSearch() function, before calling invoke:
function showSearchSkeletons() {
  const grid = document.querySelector('.file-grid');
  grid.innerHTML = Array(12).fill(0).map(() => `
    <div class="file-card skeleton-card">
      <div class="card-preview skeleton" style="height: 140px;"></div>
      <div class="card-info">
        <div class="skeleton" style="height: 14px; width: 80%; margin-bottom: 8px;"></div>
        <div class="skeleton" style="height: 12px; width: 50%;"></div>
      </div>
    </div>
  `).join('');
}
```

2. **Add CSS for skeleton cards**:
```css
.skeleton-card {
  pointer-events: none;
  animation: fadeIn 0.3s ease;
}
.skeleton-card .card-preview {
  background: var(--bg-tertiary);
}
```

3. **Show skeleton before search, hide after results**:
- Call `showSearchSkeletons()` at start of `performSearch()`
- Replace with real results when data arrives

#### 1.2 Indexing Progress Skeleton
**File**: `hippo-tauri/ui/dist/index.html`

1. **Add indexing overlay** - Show when indexing is active:
```html
<div id="indexing-overlay" class="indexing-overlay" style="display: none;">
  <div class="indexing-progress">
    <div class="indexing-spinner"></div>
    <div class="indexing-info">
      <div class="indexing-title">Indexing files...</div>
      <div class="indexing-stats">
        <span class="indexed-count">0</span> / <span class="total-count">0</span> files
      </div>
      <div class="indexing-bar">
        <div class="indexing-bar-fill"></div>
      </div>
      <div class="indexing-eta">Estimating time...</div>
    </div>
  </div>
</div>
```

2. **Add CSS for indexing overlay**:
```css
.indexing-overlay {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 1000;
  animation: fadeInUp 0.3s ease;
}
.indexing-progress {
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 12px;
  padding: 16px 20px;
  box-shadow: var(--shadow-lg);
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 280px;
}
.indexing-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-primary);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.indexing-bar {
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 8px;
}
.indexing-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
  width: 0%;
  transition: width 0.3s ease;
}
```

3. **Poll indexing progress**:
```javascript
let indexingInterval = null;

async function startIndexingProgress() {
  const overlay = document.getElementById('indexing-overlay');
  overlay.style.display = 'block';

  indexingInterval = setInterval(async () => {
    const progress = await window.__TAURI__.invoke('get_indexing_progress');
    updateIndexingUI(progress);
    if (progress.is_complete) {
      stopIndexingProgress();
    }
  }, 500);
}

function updateIndexingUI(progress) {
  document.querySelector('.indexed-count').textContent = progress.processed || 0;
  document.querySelector('.total-count').textContent = progress.total || 0;
  const percent = progress.total ? (progress.processed / progress.total * 100) : 0;
  document.querySelector('.indexing-bar-fill').style.width = `${percent}%`;
  if (progress.eta_seconds) {
    document.querySelector('.indexing-eta').textContent = `~${Math.ceil(progress.eta_seconds)}s remaining`;
  }
}

function stopIndexingProgress() {
  clearInterval(indexingInterval);
  const overlay = document.getElementById('indexing-overlay');
  overlay.style.display = 'none';
  refreshResults(); // Reload the file list
}
```

#### 1.3 Empty State Illustrations
**File**: `hippo-tauri/ui/dist/index.html`

1. **Create empty state component**:
```html
<div id="empty-state" class="empty-state" style="display: none;">
  <div class="empty-illustration">
    <!-- Hippo illustration SVG -->
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none">
      <!-- Cute hippo SVG - simplified -->
      <circle cx="60" cy="60" r="50" fill="currentColor" opacity="0.1"/>
      <ellipse cx="60" cy="65" rx="35" ry="30" fill="currentColor" opacity="0.15"/>
      <!-- Eyes -->
      <circle cx="45" cy="50" r="6" fill="currentColor" opacity="0.4"/>
      <circle cx="75" cy="50" r="6" fill="currentColor" opacity="0.4"/>
      <!-- Snout -->
      <ellipse cx="60" cy="70" rx="20" ry="12" fill="currentColor" opacity="0.2"/>
      <circle cx="52" cy="70" r="3" fill="currentColor" opacity="0.3"/>
      <circle cx="68" cy="70" r="3" fill="currentColor" opacity="0.3"/>
    </svg>
  </div>
  <h3 class="empty-title">No files yet</h3>
  <p class="empty-description">Add a folder to start organizing your files</p>
  <button class="empty-action" onclick="document.querySelector('.add-btn').click()">
    <span class="icon">➕</span> Add Folder
  </button>
</div>
```

2. **Add empty state for search with no results**:
```html
<div id="no-results-state" class="empty-state" style="display: none;">
  <div class="empty-illustration">
    <svg width="100" height="100" viewBox="0 0 100 100" fill="none">
      <circle cx="40" cy="40" r="30" stroke="currentColor" stroke-width="6" opacity="0.2"/>
      <line x1="62" y1="62" x2="85" y2="85" stroke="currentColor" stroke-width="6" stroke-linecap="round" opacity="0.2"/>
      <text x="40" y="45" text-anchor="middle" fill="currentColor" opacity="0.3" font-size="24">?</text>
    </svg>
  </div>
  <h3 class="empty-title">No results found</h3>
  <p class="empty-description">Try a different search term or filter</p>
</div>
```

3. **Add CSS for empty states**:
```css
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 40px;
  text-align: center;
  animation: fadeInUp 0.4s ease;
}
.empty-illustration {
  color: var(--text-muted);
  margin-bottom: 20px;
}
.empty-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}
.empty-description {
  font-size: 14px;
  color: var(--text-tertiary);
  max-width: 280px;
  margin-bottom: 20px;
}
.empty-action {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: var(--accent-primary);
  color: white;
  border: none;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}
.empty-action:hover {
  background: var(--accent-secondary);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}
```

4. **Show/hide empty states based on data**:
```javascript
function updateEmptyStates(memories, searchQuery) {
  const emptyState = document.getElementById('empty-state');
  const noResultsState = document.getElementById('no-results-state');
  const fileGrid = document.querySelector('.file-grid');

  if (memories.length === 0) {
    fileGrid.style.display = 'none';
    if (searchQuery) {
      emptyState.style.display = 'none';
      noResultsState.style.display = 'flex';
    } else {
      emptyState.style.display = 'flex';
      noResultsState.style.display = 'none';
    }
  } else {
    fileGrid.style.display = 'grid';
    emptyState.style.display = 'none';
    noResultsState.style.display = 'none';
  }
}
```

#### 1.4 Better Thumbnail Placeholders
**File**: `hippo-tauri/ui/dist/index.html`

1. **Type-specific placeholder icons**:
```javascript
function getTypePlaceholder(kind) {
  const icons = {
    'Image': `<svg>...</svg>`, // Image icon
    'Video': `<svg>...</svg>`, // Video icon
    'Audio': `<svg>...</svg>`, // Audio waveform icon
    'Document': `<svg>...</svg>`, // Document icon
    'Code': `<svg>...</svg>`, // Code brackets icon
    'Archive': `<svg>...</svg>`, // Zip icon
    'Folder': `<svg>...</svg>`, // Folder icon
  };
  return icons[kind] || icons['Document'];
}
```

2. **Loading state for thumbnails**:
```css
.card-preview.loading {
  background: linear-gradient(90deg, var(--bg-tertiary) 25%, var(--bg-hover) 50%, var(--bg-tertiary) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}
.card-preview.loading::after {
  content: '';
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
```

### Testing
1. Start app with no sources - verify empty state shows
2. Add a large folder - verify indexing progress shows
3. Search for non-existent term - verify no results state
4. Search with debounce - verify skeleton appears during loading

---

## Workstream 2: Tag Colors & Visual Enhancements (Phase 2)
**Branch**: `feature/tag-colors`
**PR Title**: `feat(tags): Add tag colors with picker and visual improvements`

### Backend Changes

#### 2.1 Storage: Persist Tag Colors
**File**: `hippo-core/src/storage/mod.rs`

1. **Update tags table schema** (if not already):
```sql
-- Add color column to tags table
ALTER TABLE tags ADD COLUMN color TEXT;
```

2. **Update tag storage methods**:
```rust
pub async fn set_tag_color(&self, tag_name: &str, color: &str) -> Result<()> {
    let conn = self.conn.lock().await;
    conn.execute(
        "UPDATE tags SET color = ?1 WHERE name = ?2",
        params![color, tag_name],
    )?;
    Ok(())
}

pub async fn get_tag_with_color(&self, tag_name: &str) -> Result<Option<(String, u64, Option<String>)>> {
    let conn = self.conn.lock().await;
    let mut stmt = conn.prepare("SELECT name, count, color FROM tags WHERE name = ?1")?;
    let result = stmt.query_row(params![tag_name], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).optional()?;
    Ok(result)
}

pub async fn list_tags_with_colors(&self) -> Result<Vec<(String, u64, Option<String>)>> {
    let conn = self.conn.lock().await;
    let mut stmt = conn.prepare("SELECT name, count, color FROM tags ORDER BY count DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    let tags: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    Ok(tags)
}
```

#### 2.2 Tauri Commands
**File**: `hippo-tauri/src/main.rs`

1. **Add set_tag_color command**:
```rust
#[tauri::command]
async fn set_tag_color(
    tag_name: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Not initialized")?;

    hippo.storage().set_tag_color(&tag_name, &color).await
        .map_err(|e| e.to_string())?;

    Ok(format!("Set color {} for tag {}", color, tag_name))
}
```

2. **Update get_tags to return colors**:
```rust
#[tauri::command]
async fn get_tags(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Not initialized")?;

    let tags = hippo.storage().list_tags_with_colors().await
        .map_err(|e| e.to_string())?;

    let tag_objects: Vec<serde_json::Value> = tags.iter().map(|(name, count, color)| {
        serde_json::json!({
            "name": name,
            "count": count,
            "color": color
        })
    }).collect();

    Ok(serde_json::json!(tag_objects))
}
```

### Frontend Changes

#### 2.3 Tag Color Picker Component
**File**: `hippo-tauri/ui/dist/index.html`

1. **Predefined color palette**:
```javascript
const TAG_COLORS = [
  { name: 'Red', value: '#ef4444' },
  { name: 'Orange', value: '#f97316' },
  { name: 'Amber', value: '#f59e0b' },
  { name: 'Yellow', value: '#eab308' },
  { name: 'Lime', value: '#84cc16' },
  { name: 'Green', value: '#22c55e' },
  { name: 'Emerald', value: '#10b981' },
  { name: 'Teal', value: '#14b8a6' },
  { name: 'Cyan', value: '#06b6d4' },
  { name: 'Sky', value: '#0ea5e9' },
  { name: 'Blue', value: '#3b82f6' },
  { name: 'Indigo', value: '#6366f1' },
  { name: 'Violet', value: '#8b5cf6' },
  { name: 'Purple', value: '#a855f7' },
  { name: 'Fuchsia', value: '#d946ef' },
  { name: 'Pink', value: '#ec4899' },
  { name: 'Rose', value: '#f43f5e' },
  { name: 'Gray', value: '#6b7280' },
];
```

2. **Color picker dropdown**:
```html
<div class="color-picker-dropdown" id="tag-color-picker" style="display: none;">
  <div class="color-picker-header">Choose color</div>
  <div class="color-grid">
    <!-- Colors rendered via JS -->
  </div>
  <button class="color-clear-btn">Remove color</button>
</div>
```

3. **CSS for color picker**:
```css
.color-picker-dropdown {
  position: absolute;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 12px;
  padding: 12px;
  box-shadow: var(--shadow-lg);
  z-index: 1000;
  min-width: 200px;
}
.color-picker-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  margin-bottom: 10px;
}
.color-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 6px;
}
.color-swatch {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.15s ease;
}
.color-swatch:hover {
  transform: scale(1.15);
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}
.color-swatch.selected {
  border-color: var(--text-primary);
}
.color-clear-btn {
  width: 100%;
  margin-top: 10px;
  padding: 8px;
  background: transparent;
  border: 1px dashed var(--border-primary);
  border-radius: 8px;
  color: var(--text-tertiary);
  font-size: 12px;
  cursor: pointer;
}
.color-clear-btn:hover {
  background: var(--bg-hover);
}
```

4. **Show color picker on tag right-click**:
```javascript
function showTagColorPicker(tagName, event) {
  event.preventDefault();
  const picker = document.getElementById('tag-color-picker');
  picker.style.display = 'block';
  picker.style.top = `${event.clientY}px`;
  picker.style.left = `${event.clientX}px`;
  picker.dataset.tagName = tagName;

  // Render colors
  const grid = picker.querySelector('.color-grid');
  grid.innerHTML = TAG_COLORS.map(c => `
    <div class="color-swatch"
         style="background: ${c.value}"
         data-color="${c.value}"
         title="${c.name}">
    </div>
  `).join('');
}

// Handle color selection
document.getElementById('tag-color-picker').addEventListener('click', async (e) => {
  if (e.target.classList.contains('color-swatch')) {
    const tagName = e.currentTarget.dataset.tagName;
    const color = e.target.dataset.color;
    await window.__TAURI__.invoke('set_tag_color', { tagName, color });
    refreshTags();
    e.currentTarget.style.display = 'none';
  }
});
```

#### 2.4 Display Colored Tags
**File**: `hippo-tauri/ui/dist/index.html`

1. **Update tag rendering everywhere**:
```javascript
function renderTag(tag) {
  const colorStyle = tag.color
    ? `background: ${tag.color}; color: white;`
    : '';
  return `
    <span class="tag"
          style="${colorStyle}"
          data-tag="${tag.name}"
          oncontextmenu="showTagColorPicker('${tag.name}', event)">
      ${tag.name}
      ${tag.count ? `<span class="tag-count">${tag.count}</span>` : ''}
    </span>
  `;
}
```

2. **Update sidebar tags**:
```javascript
async function refreshTags() {
  const tags = await window.__TAURI__.invoke('get_tags');
  const container = document.querySelector('.tag-list');
  container.innerHTML = tags.slice(0, 10).map(t => renderTag(t)).join('');
}
```

3. **Update detail panel tags**:
```javascript
function renderDetailTags(memory) {
  return memory.tags.map(t => `
    <span class="detail-tag"
          style="${t.color ? `border-color: ${t.color}; color: ${t.color}` : ''}">
      ${t.name}
    </span>
  `).join('');
}
```

### Testing
1. Right-click a tag → Color picker appears
2. Select a color → Tag updates everywhere
3. Remove color → Tag returns to default
4. Verify colors persist after app restart

---

## Workstream 3: Tag Autocomplete & Bulk Tagging (Phase 2)
**Branch**: `feature/bulk-tagging`
**PR Title**: `feat(tags): Add tag autocomplete and bulk tagging UI`

### Frontend: Tag Autocomplete

#### 3.1 Autocomplete Component
**File**: `hippo-tauri/ui/dist/index.html`

1. **Autocomplete dropdown HTML**:
```html
<div class="tag-autocomplete" id="tag-autocomplete" style="display: none;">
  <div class="autocomplete-list"></div>
</div>
```

2. **CSS for autocomplete**:
```css
.tag-autocomplete {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  box-shadow: var(--shadow-md);
  max-height: 200px;
  overflow-y: auto;
  z-index: 100;
}
.autocomplete-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  cursor: pointer;
  transition: background 0.1s;
}
.autocomplete-item:hover,
.autocomplete-item.selected {
  background: var(--bg-hover);
}
.autocomplete-item .tag-name {
  font-size: 13px;
  color: var(--text-primary);
}
.autocomplete-item .tag-count {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
}
.autocomplete-item .tag-color {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 8px;
}
.autocomplete-create {
  padding: 8px 12px;
  border-top: 1px solid var(--border-secondary);
  color: var(--accent-primary);
  font-size: 12px;
  cursor: pointer;
}
.autocomplete-create:hover {
  background: var(--bg-hover);
}
```

3. **Autocomplete JavaScript**:
```javascript
class TagAutocomplete {
  constructor(inputElement, onSelect) {
    this.input = inputElement;
    this.onSelect = onSelect;
    this.dropdown = document.getElementById('tag-autocomplete');
    this.allTags = [];
    this.selectedIndex = -1;

    this.input.addEventListener('input', () => this.onInput());
    this.input.addEventListener('keydown', (e) => this.onKeydown(e));
    this.input.addEventListener('blur', () => setTimeout(() => this.hide(), 150));
  }

  async loadTags() {
    this.allTags = await window.__TAURI__.invoke('get_tags');
  }

  onInput() {
    const query = this.input.value.toLowerCase();
    if (!query) {
      this.hide();
      return;
    }

    // Filter and sort by frequency
    const matches = this.allTags
      .filter(t => t.name.toLowerCase().includes(query))
      .sort((a, b) => {
        // Prioritize starts-with matches
        const aStarts = a.name.toLowerCase().startsWith(query);
        const bStarts = b.name.toLowerCase().startsWith(query);
        if (aStarts && !bStarts) return -1;
        if (!aStarts && bStarts) return 1;
        // Then sort by count
        return b.count - a.count;
      })
      .slice(0, 8);

    this.render(matches, query);
  }

  render(matches, query) {
    const list = this.dropdown.querySelector('.autocomplete-list');
    const exactMatch = matches.some(t => t.name.toLowerCase() === query);

    list.innerHTML = matches.map((t, i) => `
      <div class="autocomplete-item ${i === this.selectedIndex ? 'selected' : ''}"
           data-tag="${t.name}">
        ${t.color ? `<span class="tag-color" style="background: ${t.color}"></span>` : ''}
        <span class="tag-name">${this.highlight(t.name, query)}</span>
        <span class="tag-count">${t.count}</span>
      </div>
    `).join('');

    // Show "create new" option if no exact match
    if (!exactMatch && query.length >= 2) {
      list.innerHTML += `
        <div class="autocomplete-create" data-create="${query}">
          + Create tag "${query}"
        </div>
      `;
    }

    this.dropdown.style.display = 'block';

    // Handle clicks
    list.querySelectorAll('.autocomplete-item').forEach(item => {
      item.addEventListener('click', () => {
        this.onSelect(item.dataset.tag);
        this.hide();
      });
    });

    const createBtn = list.querySelector('.autocomplete-create');
    if (createBtn) {
      createBtn.addEventListener('click', () => {
        this.onSelect(createBtn.dataset.create);
        this.hide();
      });
    }
  }

  highlight(text, query) {
    const idx = text.toLowerCase().indexOf(query);
    if (idx === -1) return text;
    return text.slice(0, idx) +
           `<strong>${text.slice(idx, idx + query.length)}</strong>` +
           text.slice(idx + query.length);
  }

  onKeydown(e) {
    const items = this.dropdown.querySelectorAll('.autocomplete-item');

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      this.selectedIndex = Math.min(this.selectedIndex + 1, items.length - 1);
      this.updateSelection();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
      this.updateSelection();
    } else if (e.key === 'Enter' && this.selectedIndex >= 0) {
      e.preventDefault();
      const item = items[this.selectedIndex];
      if (item) {
        this.onSelect(item.dataset.tag);
        this.hide();
      }
    } else if (e.key === 'Escape') {
      this.hide();
    }
  }

  updateSelection() {
    const items = this.dropdown.querySelectorAll('.autocomplete-item');
    items.forEach((item, i) => {
      item.classList.toggle('selected', i === this.selectedIndex);
    });
  }

  hide() {
    this.dropdown.style.display = 'none';
    this.selectedIndex = -1;
  }
}
```

### Frontend: Bulk Tagging UI

#### 3.2 Multi-Select Mode
**File**: `hippo-tauri/ui/dist/index.html`

1. **Track selected files**:
```javascript
let selectedMemories = new Set();
let isMultiSelectMode = false;

function toggleMultiSelect() {
  isMultiSelectMode = !isMultiSelectMode;
  document.body.classList.toggle('multi-select-mode', isMultiSelectMode);
  if (!isMultiSelectMode) {
    selectedMemories.clear();
    updateSelectionUI();
  }
}

function toggleMemorySelection(memoryId, event) {
  if (!isMultiSelectMode) {
    if (event.shiftKey || event.metaKey || event.ctrlKey) {
      toggleMultiSelect();
    } else {
      return; // Normal click, show detail panel
    }
  }

  if (selectedMemories.has(memoryId)) {
    selectedMemories.delete(memoryId);
  } else {
    selectedMemories.add(memoryId);
  }

  updateSelectionUI();
}

function updateSelectionUI() {
  // Update cards
  document.querySelectorAll('.file-card').forEach(card => {
    const id = card.dataset.memoryId;
    card.classList.toggle('selected', selectedMemories.has(id));
  });

  // Update bulk action bar
  const bar = document.getElementById('bulk-action-bar');
  if (selectedMemories.size > 0) {
    bar.style.display = 'flex';
    bar.querySelector('.selection-count').textContent =
      `${selectedMemories.size} file${selectedMemories.size > 1 ? 's' : ''} selected`;
  } else {
    bar.style.display = 'none';
  }
}
```

2. **Bulk action bar HTML**:
```html
<div id="bulk-action-bar" class="bulk-action-bar" style="display: none;">
  <div class="bulk-left">
    <span class="selection-count">0 files selected</span>
    <button class="bulk-btn select-all" onclick="selectAll()">Select All</button>
    <button class="bulk-btn clear-selection" onclick="clearSelection()">Clear</button>
  </div>
  <div class="bulk-right">
    <button class="bulk-btn bulk-tag" onclick="showBulkTagModal()">
      <span class="icon">🏷️</span> Add Tags
    </button>
    <button class="bulk-btn bulk-favorite" onclick="bulkToggleFavorite()">
      <span class="icon">⭐</span> Favorite
    </button>
    <button class="bulk-btn bulk-delete danger" onclick="bulkDelete()">
      <span class="icon">🗑️</span> Remove
    </button>
  </div>
</div>
```

3. **Bulk action bar CSS**:
```css
.bulk-action-bar {
  position: fixed;
  bottom: 0;
  left: 240px; /* Sidebar width */
  right: 0;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-primary);
  padding: 12px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  z-index: 100;
  animation: fadeInUp 0.2s ease;
  box-shadow: 0 -4px 20px rgba(0,0,0,0.1);
}
.bulk-left, .bulk-right {
  display: flex;
  align-items: center;
  gap: 12px;
}
.selection-count {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.bulk-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}
.bulk-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.bulk-btn.danger:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.3);
}
.file-card.selected {
  outline: 2px solid var(--accent-primary);
  outline-offset: 2px;
}
.file-card.selected::before {
  content: '✓';
  position: absolute;
  top: 8px;
  left: 8px;
  width: 20px;
  height: 20px;
  background: var(--accent-primary);
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  z-index: 10;
}
```

4. **Bulk tag modal**:
```html
<div id="bulk-tag-modal" class="modal-overlay" style="display: none;">
  <div class="modal-content">
    <div class="modal-header">
      <h3>Add Tags to <span id="bulk-tag-count">0</span> Files</h3>
      <button class="modal-close" onclick="closeBulkTagModal()">×</button>
    </div>
    <div class="modal-body">
      <div class="tag-input-container" style="position: relative;">
        <input type="text" id="bulk-tag-input" placeholder="Type tag name..." class="tag-input">
        <!-- Autocomplete dropdown will appear here -->
      </div>
      <div class="selected-tags" id="bulk-selected-tags"></div>
      <div class="common-tags">
        <div class="common-tags-label">Common tags:</div>
        <div class="common-tags-list" id="common-tags-list"></div>
      </div>
    </div>
    <div class="modal-footer">
      <button class="modal-btn cancel" onclick="closeBulkTagModal()">Cancel</button>
      <button class="modal-btn primary" onclick="applyBulkTags()">Apply Tags</button>
    </div>
  </div>
</div>
```

5. **Bulk tagging logic**:
```javascript
let bulkTagsToAdd = [];

function showBulkTagModal() {
  const modal = document.getElementById('bulk-tag-modal');
  document.getElementById('bulk-tag-count').textContent = selectedMemories.size;
  modal.style.display = 'flex';

  // Initialize autocomplete
  const input = document.getElementById('bulk-tag-input');
  new TagAutocomplete(input, (tag) => {
    if (!bulkTagsToAdd.includes(tag)) {
      bulkTagsToAdd.push(tag);
      renderBulkSelectedTags();
    }
    input.value = '';
  });

  // Load common tags
  loadCommonTags();
  bulkTagsToAdd = [];
  renderBulkSelectedTags();
}

function renderBulkSelectedTags() {
  const container = document.getElementById('bulk-selected-tags');
  container.innerHTML = bulkTagsToAdd.map(tag => `
    <span class="selected-tag">
      ${tag}
      <button onclick="removeBulkTag('${tag}')">×</button>
    </span>
  `).join('');
}

async function loadCommonTags() {
  const tags = await window.__TAURI__.invoke('get_tags');
  const container = document.getElementById('common-tags-list');
  container.innerHTML = tags.slice(0, 10).map(t => `
    <button class="common-tag-btn" onclick="addBulkTag('${t.name}')">
      ${t.color ? `<span class="tag-dot" style="background: ${t.color}"></span>` : ''}
      ${t.name}
    </button>
  `).join('');
}

async function applyBulkTags() {
  if (bulkTagsToAdd.length === 0) return;

  const memoryIds = Array.from(selectedMemories);

  for (const tag of bulkTagsToAdd) {
    await window.__TAURI__.invoke('bulk_add_tag', {
      memoryIds,
      tag
    });
  }

  closeBulkTagModal();
  clearSelection();
  refreshResults();
  showToast(`Added ${bulkTagsToAdd.length} tag(s) to ${memoryIds.length} file(s)`);
}

function closeBulkTagModal() {
  document.getElementById('bulk-tag-modal').style.display = 'none';
}
```

### Testing
1. Click/Cmd+Click to select multiple files
2. Bulk action bar appears at bottom
3. Click "Add Tags" → Modal with autocomplete
4. Type to filter tags, arrow keys to navigate
5. Apply tags → All selected files updated

---

## Workstream 4: File Type Icons & Modal Polish (Phase 1)
**Branch**: `feature/file-type-icons`
**PR Title**: `feat(ui): Add custom file type icons and polish modals`

### Custom File Type SVG Icons

#### 4.1 Icon SVG Collection
**File**: `hippo-tauri/ui/dist/index.html`

Add inline SVG icons at top of `<body>`:

```html
<svg style="display: none;">
  <defs>
    <!-- Image icon -->
    <symbol id="icon-image" viewBox="0 0 24 24">
      <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
      <circle cx="8.5" cy="8.5" r="1.5" fill="currentColor"/>
      <path d="M21 15l-5-5L5 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </symbol>

    <!-- Video icon -->
    <symbol id="icon-video" viewBox="0 0 24 24">
      <rect x="2" y="4" width="15" height="16" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M17 8l5-3v14l-5-3V8z" fill="none" stroke="currentColor" stroke-width="2"/>
    </symbol>

    <!-- Audio icon -->
    <symbol id="icon-audio" viewBox="0 0 24 24">
      <path d="M9 18V5l12-2v13" fill="none" stroke="currentColor" stroke-width="2"/>
      <circle cx="6" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="2"/>
      <circle cx="18" cy="16" r="3" fill="none" stroke="currentColor" stroke-width="2"/>
    </symbol>

    <!-- Document icon -->
    <symbol id="icon-document" viewBox="0 0 24 24">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </symbol>

    <!-- Code icon -->
    <symbol id="icon-code" viewBox="0 0 24 24">
      <path d="M16 18l6-6-6-6M8 6l-6 6 6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </symbol>

    <!-- Archive icon -->
    <symbol id="icon-archive" viewBox="0 0 24 24">
      <path d="M21 8v13H3V8M23 3H1v5h22V3z" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M10 12h4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </symbol>

    <!-- Folder icon -->
    <symbol id="icon-folder" viewBox="0 0 24 24">
      <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" fill="none" stroke="currentColor" stroke-width="2"/>
    </symbol>

    <!-- PDF icon -->
    <symbol id="icon-pdf" viewBox="0 0 24 24">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M14 2v6h6" stroke="currentColor" stroke-width="2"/>
      <text x="12" y="17" text-anchor="middle" fill="currentColor" font-size="6" font-weight="bold">PDF</text>
    </symbol>

    <!-- Spreadsheet icon -->
    <symbol id="icon-spreadsheet" viewBox="0 0 24 24">
      <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M3 9h18M3 15h18M9 3v18M15 3v18" stroke="currentColor" stroke-width="2"/>
    </symbol>

    <!-- Presentation icon -->
    <symbol id="icon-presentation" viewBox="0 0 24 24">
      <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M12 17v4M8 21h8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </symbol>

    <!-- Unknown/Generic file icon -->
    <symbol id="icon-file" viewBox="0 0 24 24">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" fill="none" stroke="currentColor" stroke-width="2"/>
      <path d="M14 2v6h6" stroke="currentColor" stroke-width="2"/>
    </symbol>
  </defs>
</svg>
```

#### 4.2 Icon Mapping Function
```javascript
function getFileTypeIcon(kind, extension) {
  // Specific extension icons
  const extensionIcons = {
    'pdf': 'pdf',
    'doc': 'document', 'docx': 'document',
    'xls': 'spreadsheet', 'xlsx': 'spreadsheet',
    'ppt': 'presentation', 'pptx': 'presentation',
    'rs': 'code', 'py': 'code', 'js': 'code', 'ts': 'code',
    'zip': 'archive', 'tar': 'archive', 'gz': 'archive', 'rar': 'archive',
  };

  if (extensionIcons[extension]) {
    return `<svg class="file-type-icon"><use href="#icon-${extensionIcons[extension]}"/></svg>`;
  }

  // Kind-based icons
  const kindIcons = {
    'Image': 'image',
    'Video': 'video',
    'Audio': 'audio',
    'Document': 'document',
    'Code': 'code',
    'Archive': 'archive',
    'Folder': 'folder',
    'Spreadsheet': 'spreadsheet',
    'Presentation': 'presentation',
  };

  const iconName = kindIcons[getKindName(kind)] || 'file';
  return `<svg class="file-type-icon"><use href="#icon-${iconName}"/></svg>`;
}

function getKindName(kind) {
  if (typeof kind === 'string') return kind;
  return Object.keys(kind)[0];
}
```

#### 4.3 Icon Styling
```css
.file-type-icon {
  width: 48px;
  height: 48px;
  color: var(--text-muted);
  opacity: 0.6;
}
.card-preview .file-type-icon {
  width: 40px;
  height: 40px;
}
/* Color by type */
.file-type-icon.image { color: #22c55e; }
.file-type-icon.video { color: #ef4444; }
.file-type-icon.audio { color: #f97316; }
.file-type-icon.document { color: #3b82f6; }
.file-type-icon.code { color: #8b5cf6; }
.file-type-icon.archive { color: #eab308; }
.file-type-icon.pdf { color: #dc2626; }
```

### Modal Polish

#### 4.4 Update Modal Styles
```css
/* Base modal styles */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.4);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  animation: fadeIn 0.15s ease;
}
.modal-content {
  background: var(--bg-secondary);
  border-radius: 16px;
  width: 90%;
  max-width: 480px;
  max-height: 85vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25);
  animation: fadeInScale 0.2s ease;
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-primary);
}
.modal-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}
.modal-close {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 8px;
  font-size: 20px;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all 0.15s ease;
}
.modal-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.modal-body {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--border-primary);
  background: var(--bg-tertiary);
}
.modal-btn {
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}
.modal-btn.cancel {
  background: transparent;
  border: 1px solid var(--border-primary);
  color: var(--text-secondary);
}
.modal-btn.cancel:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.modal-btn.primary {
  background: var(--accent-primary);
  border: none;
  color: white;
}
.modal-btn.primary:hover {
  background: var(--accent-secondary);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}
.modal-btn.danger {
  background: #ef4444;
  border: none;
  color: white;
}
.modal-btn.danger:hover {
  background: #dc2626;
}

/* Input styling in modals */
.modal-input {
  width: 100%;
  padding: 12px 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 10px;
  font-size: 14px;
  color: var(--text-primary);
  transition: all 0.15s ease;
}
.modal-input:focus {
  outline: none;
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}
.modal-input::placeholder {
  color: var(--text-muted);
}

/* Toast notifications */
.toast {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 12px;
  padding: 12px 20px;
  box-shadow: var(--shadow-lg);
  display: flex;
  align-items: center;
  gap: 10px;
  z-index: 10001;
  animation: fadeInUp 0.3s ease;
}
.toast.success { border-left: 4px solid #22c55e; }
.toast.error { border-left: 4px solid #ef4444; }
.toast.info { border-left: 4px solid #3b82f6; }
```

#### 4.5 Toast Notification System
```javascript
function showToast(message, type = 'info', duration = 3000) {
  const existing = document.querySelector('.toast');
  if (existing) existing.remove();

  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.innerHTML = `
    <span class="toast-icon">${type === 'success' ? '✓' : type === 'error' ? '✕' : 'ℹ'}</span>
    <span class="toast-message">${message}</span>
  `;
  document.body.appendChild(toast);

  setTimeout(() => {
    toast.style.animation = 'fadeOut 0.3s ease forwards';
    setTimeout(() => toast.remove(), 300);
  }, duration);
}
```

### Testing
1. Verify icons show correctly for each file type
2. Check icon colors match file types
3. Open various modals - verify polish
4. Trigger toast notifications - verify styling
5. Test in dark mode

---

## Git Workflow for Each Agent

Each agent should follow this workflow:

```bash
# 1. Create feature branch from main
git checkout main
git pull origin main
git checkout -b feature/<branch-name>

# 2. Make changes
# ... edit files ...

# 3. Test changes
cargo build --workspace
cargo test --workspace

# 4. Commit with conventional commits
git add .
git commit -m "feat(scope): description

🤖 Generated with Claude Code

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

# 5. Push and create PR
git push -u origin feature/<branch-name>
gh pr create --title "feat(scope): Title" --body "$(cat <<'EOF'
## Summary
- Bullet point 1
- Bullet point 2

## Changes
- File 1: description
- File 2: description

## Test Plan
- [ ] Test case 1
- [ ] Test case 2

🤖 Generated with Claude Code
EOF
)"
```

---

## Coordination Notes

- **No file conflicts**: Each workstream touches different files
- **Workstream 2** (Tag Colors) may need to merge before Workstream 3 (Bulk Tagging) to use color data
- All workstreams can be developed in parallel
- Run `cargo build && cargo test` before committing
- UI changes are in a single file (`index.html`) so coordinate carefully

