# Integration Guide: WASM Search in Tauri UI

This guide shows how to integrate the hippo-wasm module into the Tauri UI for client-side search.

## Step 1: Build the WASM Module

```bash
cd hippo-wasm
./build-wasm.sh ../hippo-tauri/ui/dist/wasm
```

This will generate:
- `hippo_wasm.js` - JavaScript bindings
- `hippo_wasm_bg.wasm` - WebAssembly binary
- `hippo_wasm_bg.wasm.d.ts` - TypeScript definitions (if enabled)

## Step 2: Update index.html

Add the WASM module to your `hippo-tauri/ui/dist/index.html`:

```html
<!-- Add at the top of your script section -->
<script type="module">
// Import WASM module
let wasmSearch = null;

async function initWasm() {
    try {
        const wasm = await import('./wasm/hippo_wasm.js');
        await wasm.default(); // Initialize
        wasmSearch = wasm;
        console.log('WASM search ready');
    } catch (err) {
        console.warn('WASM not available, falling back to server search:', err);
    }
}

// Initialize WASM on page load
initWasm();
</script>
```

## Step 3: Hybrid Search Implementation

Replace your existing search function with a hybrid approach:

```javascript
// Global cache for memories
let cachedMemories = [];

// Update cache when data changes
async function refreshData() {
    const memories = await invoke('search', {
        query: { text: null, tags: [], limit: 10000 }
    });
    cachedMemories = memories;
    render();
}

// Hybrid search function
async function handleSearch(query) {
    if (!query || query.length < 2) {
        // Show all results
        state.memories = cachedMemories;
        render();
        return;
    }

    // Try WASM search first for instant results
    if (wasmSearch && cachedMemories.length > 0) {
        try {
            // Convert to WASM format
            const wasmMemories = cachedMemories.map(m => ({
                id: m.id,
                path: m.path,
                title: m.metadata?.title,
                tags: m.tags.map(t => t.name),
                file_size: m.metadata?.file_size || 0,
                modified_at: m.modified_at,
                kind: getMemoryKindString(m.kind)
            }));

            // Perform client-side search
            const resultsJson = wasmSearch.search_local(
                JSON.stringify(wasmMemories),
                query
            );
            const results = JSON.parse(resultsJson);

            // Convert back to full Memory objects
            state.memories = results.map(r => {
                return cachedMemories.find(m => m.id === r.memory.id);
            }).filter(Boolean);

            render();
            return;
        } catch (err) {
            console.warn('WASM search failed, falling back to server:', err);
        }
    }

    // Fallback to server search
    const results = await invoke('search', {
        query: { text: query, tags: [], limit: 100 }
    });
    state.memories = results.memories;
    render();
}

// Helper function to extract kind string
function getMemoryKindString(kind) {
    if (typeof kind === 'string') return kind;
    if (kind.Image) return 'image';
    if (kind.Video) return 'video';
    if (kind.Audio) return 'audio';
    if (kind.Document) return 'document';
    if (kind.Code) return 'code';
    return 'unknown';
}
```

## Step 4: Client-Side Filtering

Use WASM for instant type filtering:

```javascript
function applyFilters() {
    let filtered = cachedMemories;

    // Use WASM for type filtering
    if (wasmSearch && state.filterType !== 'all') {
        try {
            const wasmMemories = filtered.map(toWasmMemory);
            const filteredJson = wasmSearch.filter_by_type(
                JSON.stringify(wasmMemories),
                state.filterType
            );
            const filtered = JSON.parse(filteredJson);
            state.memories = filtered.map(w =>
                cachedMemories.find(m => m.id === w.id)
            ).filter(Boolean);
        } catch (err) {
            // Fallback to JavaScript filtering
            state.memories = filtered.filter(m =>
                getMemoryKindString(m.kind) === state.filterType
            );
        }
    }

    render();
}
```

## Step 5: Client-Side Sorting

Use WASM for instant sorting:

```javascript
function applySorting() {
    if (!wasmSearch) {
        // Fallback to JavaScript sorting
        sortMemoriesJS();
        return;
    }

    try {
        const wasmMemories = state.memories.map(toWasmMemory);
        const field = getSortField(state.sortBy);
        const ascending = isAscending(state.sortBy);

        const sortedJson = wasmSearch.sort_memories(
            JSON.stringify(wasmMemories),
            field,
            ascending
        );

        const sorted = JSON.parse(sortedJson);
        state.memories = sorted.map(w =>
            cachedMemories.find(m => m.id === w.id)
        ).filter(Boolean);

        render();
    } catch (err) {
        console.warn('WASM sort failed:', err);
        sortMemoriesJS();
    }
}

function getSortField(sortBy) {
    if (sortBy.includes('name')) return 'name';
    if (sortBy.includes('size')) return 'size';
    if (sortBy.includes('date')) return 'date';
    return 'date';
}

function isAscending(sortBy) {
    return sortBy.includes('asc') || sortBy.includes('A-Z') || sortBy === 'oldest';
}
```

## Step 6: Fuzzy Autocomplete

Use WASM for tag suggestions:

```javascript
function getSuggestedTags(input) {
    if (!wasmSearch) return [];

    const allTags = [...new Set(cachedMemories.flatMap(m => m.tags.map(t => t.name)))];

    // Use fuzzy matching for suggestions
    const scored = allTags.map(tag => ({
        tag,
        score: wasmSearch.fuzzy_match(input.toLowerCase(), tag.toLowerCase())
    }))
    .filter(s => s.score > 0.5)
    .sort((a, b) => b.score - a.score)
    .slice(0, 10);

    return scored.map(s => s.tag);
}
```

## Step 7: Performance Monitoring

Add performance tracking:

```javascript
async function handleSearch(query) {
    const startTime = performance.now();

    // ... search logic ...

    const endTime = performance.now();
    const duration = endTime - startTime;

    console.log(`Search completed in ${duration.toFixed(2)}ms`);

    // Show in UI (optional)
    document.getElementById('search-time').textContent =
        `${duration < 1 ? '<1' : Math.round(duration)}ms`;
}
```

## Step 8: Offline Support

Cache results in localStorage for offline use:

```javascript
// Save to localStorage after fetching
async function refreshData() {
    const memories = await invoke('search', {
        query: { text: null, tags: [], limit: 10000 }
    });

    cachedMemories = memories;
    localStorage.setItem('hippo_cache', JSON.stringify({
        memories,
        timestamp: Date.now()
    }));

    render();
}

// Load from localStorage on startup
function loadFromCache() {
    const cached = localStorage.getItem('hippo_cache');
    if (cached) {
        const { memories, timestamp } = JSON.parse(cached);

        // Use cache if less than 1 hour old
        if (Date.now() - timestamp < 3600000) {
            cachedMemories = memories;
            render();
            return true;
        }
    }
    return false;
}

// Initialize
if (!loadFromCache()) {
    refreshData();
}
```

## Step 9: Progressive Enhancement

Graceful degradation when WASM is not available:

```javascript
function isWasmAvailable() {
    return typeof WebAssembly === 'object' && wasmSearch !== null;
}

async function performSearch(query) {
    if (isWasmAvailable()) {
        return searchWithWasm(query);
    } else {
        return searchWithServer(query);
    }
}

// Show indicator in UI
function updateSearchIndicator() {
    const indicator = document.getElementById('search-mode');
    if (isWasmAvailable()) {
        indicator.textContent = '⚡ Client-side search';
        indicator.className = 'fast';
    } else {
        indicator.textContent = '🌐 Server search';
        indicator.className = 'server';
    }
}
```

## Benefits

1. **Instant Results**: No network latency for cached data
2. **Offline Capability**: Search works without server connection
3. **Reduced Server Load**: Most searches happen client-side
4. **Better UX**: Sub-millisecond response times
5. **Typo Tolerance**: Fuzzy matching handles misspellings
6. **Progressive Enhancement**: Falls back to server gracefully

## Performance Expectations

- **WASM Search**: 1-10ms for 1000 items
- **Server Search**: 50-200ms (network + database)
- **Initial Load**: +50-100KB (WASM module, gzipped)
- **Memory**: +2-5MB (cached memories in browser)

## Browser Support

All modern browsers with WebAssembly support:
- Chrome 57+ (95% of users)
- Firefox 52+ (98% of users)
- Safari 11+ (97% of users)
- Edge 16+ (99% of users)

## Troubleshooting

### WASM Module Not Loading

Check browser console for errors. Common issues:
- File path incorrect (check `/wasm/hippo_wasm.js`)
- CORS policy (serve from same origin)
- WASM file not built (run `./build-wasm.sh`)

### Poor Performance

If WASM is slower than expected:
- Check bundle size (`ls -lh wasm/*.wasm`)
- Ensure wasm-opt ran (should be <500KB gzipped)
- Profile with browser DevTools
- Reduce cached memory count (<10,000 items)

### Memory Issues

If browser uses too much memory:
- Limit cache size (use pagination)
- Clear old cache periodically
- Use IndexedDB instead of localStorage for large datasets
