# Hippo Web Frontend Documentation

## Overview

The Hippo Web frontend is a **responsive, mobile-first Progressive Web App (PWA)** that provides access to your Hippo file index from any modern web browser. It's built with vanilla JavaScript (no frameworks) for maximum performance and minimal bundle size.

## Architecture

### Technology Stack
- **Backend**: Rust + Axum web server
- **Frontend**: Vanilla JavaScript (ES6+)
- **Styling**: CSS3 with CSS Custom Properties (variables)
- **Layout**: CSS Grid + Flexbox
- **Offline**: Service Worker with caching strategies
- **PWA**: Web App Manifest + Service Worker

### File Structure
```
hippo-web/
├── src/
│   └── main.rs              # Axum API server
├── static/
│   ├── index.html           # Landing page
│   ├── app.html             # Main application (to be created)
│   ├── offline.html         # Offline fallback page
│   ├── sw.js                # Service worker
│   ├── manifest.json        # PWA manifest
│   ├── browserconfig.xml    # Windows tile config
│   └── icons/
│       ├── icon.svg         # SVG icon
│       ├── icon-192.png     # PWA icon 192x192
│       ├── icon-512.png     # PWA icon 512x512
│       └── generate-icons.sh
├── Cargo.toml
└── README.md
```

## Current Implementation Status

### ✅ Completed Features

#### Backend (main.rs)
- [x] Axum web server with REST API
- [x] CORS enabled for browser clients
- [x] Health check endpoint
- [x] Stats endpoint with memory counts
- [x] Search with filters (text, tags, type, sort)
- [x] Memory detail endpoint
- [x] Thumbnail serving (JPEG)
- [x] Source management (list, add, remove)
- [x] Tag management (list, add, remove)
- [x] Error handling with proper status codes
- [x] Static file serving from /static

#### Frontend Infrastructure
- [x] PWA manifest with shortcuts and file handlers
- [x] Service worker with caching strategies
- [x] Offline page fallback
- [x] Landing page (index.html)
- [x] Icons and assets
- [x] Dark mode support
- [x] Install prompts
- [x] Update notifications

#### Service Worker (sw.js)
- [x] Static asset caching
- [x] Dynamic content caching with TTL
- [x] Image caching (7 day TTL)
- [x] API response caching (5 min TTL)
- [x] Cache size limits
- [x] Cache expiration
- [x] Offline fallback
- [x] Skip waiting support
- [x] Background sync preparation

### 🔨 To Be Implemented

#### Main Application Interface (app.html)
The main application interface needs to be created with the following features:

##### Layout Components
- [ ] **Header**
  - Logo and branding
  - Global search bar with keyboard shortcut (/)
  - Dark mode toggle
  - Add source button
  - User menu (future)

- [ ] **Sidebar** (desktop) / **Mobile Menu**
  - Statistics dashboard
    - Total memories
    - Total sources
    - Total tags
    - Total size
  - Type filters
    - All Files
    - Images
    - Videos
    - Audio
    - Code
    - Documents
  - Source list
  - Tag cloud (future)
  - Actions
    - Add source
    - Refresh
    - Settings (future)

- [ ] **Main Content Area**
  - Toolbar
    - Active tag chips (removable)
    - View mode toggle (grid/list)
    - Sort dropdown
  - Memory grid/list
    - Card-based layout
    - Thumbnail previews
    - Metadata overlay
    - Lazy loading
    - Infinite scroll (future)
    - Virtual scrolling (future)
  - Empty states
  - Loading states

- [ ] **Mobile Bottom Navigation**
  - Home
  - Search
  - Add
  - Menu

- [ ] **Memory Detail Modal**
  - Large preview
  - Full metadata
  - Tags (add/remove)
  - Actions
    - Open file (if local)
    - Share
    - Favorite
    - Delete
  - Related memories (future)

##### Features to Implement
- [ ] **Search**
  - Real-time search with 300ms debounce
  - Search suggestions
  - Search history
  - Keyboard shortcuts

- [ ] **Filtering**
  - Type filters (All, Images, Videos, etc.)
  - Tag filters (include/exclude)
  - Date range filter
  - Size range filter
  - Source filter

- [ ] **Sorting**
  - Date (newest/oldest)
  - Name (A-Z/Z-A)
  - Size (ascending/descending)
  - Relevance (for searches)

- [ ] **View Modes**
  - Grid view with responsive columns
  - List view with details
  - Comfortable/Compact density

- [ ] **Tag Management**
  - Add tags to memories
  - Remove tags
  - Tag autocomplete
  - Tag colors (future)
  - Bulk tag operations (future)

- [ ] **Mobile Optimizations**
  - Touch-friendly targets (44px minimum)
  - Swipe gestures
    - Swipe right: Open menu
    - Swipe left: Close menu
  - Pull to refresh
  - Bottom sheet modals
  - Native share integration
  - Haptic feedback (if supported)

- [ ] **Progressive Enhancement**
  - Works without JS (basic HTML table)
  - Enhanced with JS
  - Offline mode indicator
  - Sync status
  - Upload queue (future)

##### State Management
```javascript
const state = {
  // Data
  memories: [],              // All fetched memories
  filteredMemories: [],      // After filters applied
  selectedMemory: null,      // Currently selected
  stats: {},                 // Index statistics
  sources: [],               // Configured sources
  tags: [],                  // Available tags

  // UI State
  activeTags: [],            // Active tag filters
  activeTypeFilter: 'all',   // Current type filter
  viewMode: 'grid',          // 'grid' or 'list'
  sortBy: 'DateNewest',      // Current sort
  searchQuery: '',           // Search text

  // Status
  loading: false,
  error: null,
  offline: !navigator.onLine,
  darkMode: false,

  // Pagination
  page: 0,
  pageSize: 50,
  hasMore: true,
};
```

## Implementation Guide

### Step 1: Create app.html

Create `/Users/punitmishra/Downloads/hippov20/hippo-web/static/app.html` with:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=5.0">
  <title>Hippo - The Memory That Never Forgets</title>

  <!-- PWA Meta -->
  <link rel="manifest" href="/manifest.json">
  <meta name="theme-color" content="#6366f1">
  <link rel="icon" type="image/svg+xml" href="/icons/icon.svg">

  <!-- Inline CSS for performance -->
  <style>
    /* CSS variables for theming */
    :root {
      --color-bg: #f8f8f7;
      --color-bg-elevated: #ffffff;
      /* ... more variables */
    }

    [data-theme="dark"] {
      --color-bg: #0f0e0d;
      /* ... dark theme */
    }

    /* Layout styles */
    /* Component styles */
    /* Responsive breakpoints */
  </style>
</head>
<body>
  <!-- App structure -->
  <div class="app">
    <header><!-- Header --></header>
    <main>
      <aside><!-- Sidebar --></aside>
      <div class="content"><!-- Main content --></div>
    </main>
    <nav class="mobile-nav"><!-- Mobile nav --></nav>
  </div>

  <!-- Modals -->
  <!-- Scripts -->
  <script>
    // State management
    // API functions
    // UI rendering
    // Event handlers
    // Initialization
  </script>
</body>
</html>
```

### Step 2: Implement Core JavaScript

#### API Integration
```javascript
const API_BASE = '/api';

async function apiCall(endpoint, options = {}) {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || 'API error');
  }

  return response.json();
}

async function searchMemories(query, tags, sort, limit = 50) {
  const params = new URLSearchParams({
    ...(query && { q: query }),
    ...(tags.length && { tags: tags.join(',') }),
    ...(sort && { sort }),
    limit: limit.toString(),
  });

  return apiCall(`/search?${params}`);
}

async function fetchStats() {
  return apiCall('/stats');
}

async function addTag(memoryId, tag) {
  return apiCall(`/memories/${memoryId}/tags`, {
    method: 'POST',
    body: JSON.stringify({ tag }),
  });
}
```

#### State Management
```javascript
function setState(updates) {
  Object.assign(state, updates);
  render();
}

function applyFilters() {
  let filtered = state.memories;

  // Type filter
  if (state.activeTypeFilter !== 'all') {
    filtered = filtered.filter(m =>
      m.kind.kind.toLowerCase() === state.activeTypeFilter
    );
  }

  setState({ filteredMemories: filtered });
}
```

#### Rendering
```javascript
function render() {
  renderHeader();
  renderSidebar();
  renderMemories();
  renderMobileNav();
}

function renderMemories() {
  const container = document.getElementById('memories');
  const isGrid = state.viewMode === 'grid';

  container.className = isGrid ? 'memories-grid' : 'memories-list';
  container.innerHTML = state.filteredMemories
    .map(memory => renderMemoryCard(memory))
    .join('');
}

function renderMemoryCard(memory) {
  const icon = getKindIcon(memory.kind.kind);
  const name = getFileName(memory.path);

  return `
    <div class="memory-card" data-id="${memory.id}">
      <div class="memory-thumbnail">${icon}</div>
      <div class="memory-info">
        <div class="memory-name">${name}</div>
        <div class="memory-meta">...</div>
      </div>
    </div>
  `;
}
```

### Step 3: Add Mobile Optimizations

#### Touch Events
```javascript
// Pull to refresh
let pullStart = 0;
let pullDistance = 0;

container.addEventListener('touchstart', e => {
  pullStart = e.touches[0].screenY;
});

container.addEventListener('touchmove', e => {
  pullDistance = e.touches[0].screenY - pullStart;
  if (pullDistance > 0 && container.scrollTop === 0) {
    // Show refresh indicator
  }
});

container.addEventListener('touchend', () => {
  if (pullDistance > 80) {
    refreshData();
  }
  pullDistance = 0;
});
```

#### Responsive Layout
```css
@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    transform: translateX(-100%);
    transition: transform 0.3s;
  }

  .sidebar.open {
    transform: translateX(0);
  }

  .mobile-nav {
    display: flex;
  }
}
```

### Step 4: Add Keyboard Shortcuts

```javascript
document.addEventListener('keydown', e => {
  // Focus search with '/'
  if (e.key === '/' && !isInputFocused()) {
    e.preventDefault();
    document.getElementById('search').focus();
  }

  // Close modal with Escape
  if (e.key === 'Escape') {
    closeModal();
    closeSidebar();
  }

  // Toggle dark mode
  if ((e.metaKey || e.ctrlKey) && e.key === 'd') {
    e.preventDefault();
    toggleDarkMode();
  }
});
```

### Step 5: Update Service Worker Routes

Add app.html to cached assets:

```javascript
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/app.html',  // Add this
  '/manifest.json',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
];
```

### Step 6: Update Landing Page

Modify index.html to link to app:

```html
<a href="/app.html" class="cta-button">Launch App</a>
```

## Design Guidelines

### Mobile-First Approach
1. Design for mobile screens first (320px+)
2. Progressively enhance for larger screens
3. Use responsive units (rem, %, vw/vh)
4. Touch targets minimum 44px
5. Test on real devices

### Accessibility
- ARIA labels for interactive elements
- Keyboard navigation support
- Focus indicators
- Screen reader friendly
- Semantic HTML
- Color contrast WCAG AA minimum

### Performance
- Lazy load images
- Debounce search (300ms)
- Virtual scrolling for large lists
- Code splitting (future)
- Compress images
- Minimize reflows

### Visual Design
- Follow system theme preference
- Consistent spacing scale
- Clear visual hierarchy
- Smooth animations (60fps)
- Loading states for all async operations
- Error states with recovery actions

## Testing Checklist

### Desktop Browsers
- [ ] Chrome/Edge (latest)
- [ ] Firefox (latest)
- [ ] Safari (latest)
- [ ] Resize window behavior
- [ ] Keyboard shortcuts
- [ ] Dark mode toggle

### Mobile Browsers
- [ ] iOS Safari
- [ ] Chrome for Android
- [ ] Samsung Internet
- [ ] Touch gestures work
- [ ] Pull to refresh
- [ ] Bottom nav positioning
- [ ] Mobile menu transitions
- [ ] Virtual keyboard handling

### PWA Features
- [ ] Install prompt appears
- [ ] App installs successfully
- [ ] Offline mode works
- [ ] Service worker caching
- [ ] Update notifications
- [ ] Icon appears correctly
- [ ] Splash screen (mobile)

### Responsive Breakpoints
- [ ] 320px (small mobile)
- [ ] 375px (standard mobile)
- [ ] 768px (tablet)
- [ ] 1024px (desktop)
- [ ] 1440px+ (large desktop)

### Performance
- [ ] Lighthouse score 90+
- [ ] First Contentful Paint < 1.5s
- [ ] Time to Interactive < 3.5s
- [ ] Smooth animations (no jank)
- [ ] Fast search response

## Deployment

### Build for Production
```bash
# Build Rust backend
cd hippo-web
cargo build --release

# Optimize assets (future)
# npm run build
```

### Serve
```bash
# Development
cargo run

# Production
export HIPPO_HOST=0.0.0.0
export HIPPO_PORT=8080
./target/release/hippo-web
```

### Docker (Future)
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/hippo-web /usr/local/bin/
COPY --from=builder /app/static /app/static
WORKDIR /app
CMD ["hippo-web"]
```

## Future Enhancements

### Near Term
- [ ] Virtual scrolling for performance
- [ ] Advanced search syntax
- [ ] Bulk operations
- [ ] Favorites/starred items
- [ ] Collections/albums
- [ ] File upload

### Long Term
- [ ] Real-time updates (WebSocket)
- [ ] Collaborative features
- [ ] Mobile apps (React Native/Flutter)
- [ ] Desktop app (Tauri already exists)
- [ ] AI features integration
- [ ] Advanced analytics

## Resources

- [MDN: Progressive Web Apps](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)
- [web.dev: PWA Checklist](https://web.dev/pwa-checklist/)
- [CSS Grid Guide](https://css-tricks.com/snippets/css/complete-guide-grid/)
- [Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [Web App Manifest](https://developer.mozilla.org/en-US/docs/Web/Manifest)

## License

MIT - See LICENSE file in repository root
