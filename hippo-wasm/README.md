# hippo-wasm

WebAssembly bindings for the Hippo search engine. This package enables client-side fuzzy matching, semantic scoring, and local search to run directly in the browser.

## Features

- **Fuzzy Matching**: Levenshtein distance-based fuzzy text matching
- **Semantic Scoring**: Cosine similarity for embedding vectors
- **Local Search**: Client-side search with scoring and highlights
- **Filtering**: Type-based filtering of memories
- **Sorting**: Client-side sorting by name, date, size
- **Statistics**: Aggregate statistics over memory collections

## Building

### Prerequisites

Install `wasm-pack`:
```bash
cargo install wasm-pack
```

### Build

Run the build script:
```bash
cd hippo-wasm
./build-wasm.sh
```

Or specify a custom output directory:
```bash
./build-wasm.sh /path/to/output
```

This will:
1. Compile the Rust code to WebAssembly
2. Generate JavaScript bindings
3. Optimize the WASM binary for size
4. Output to `../hippo-tauri/ui/dist/wasm` by default

### Manual Build

```bash
wasm-pack build --target web --out-dir ./pkg --release
```

## Usage

### Basic Integration

```html
<!DOCTYPE html>
<html>
<head>
    <title>Hippo WASM Demo</title>
</head>
<body>
    <script type="module">
        // Import the WASM module
        import init, {
            fuzzy_match,
            semantic_score,
            search_local
        } from './wasm/hippo_wasm.js';

        // Initialize the WASM module
        await init();

        // Now you can use the functions
        const score = fuzzy_match("hello", "helo");
        console.log("Fuzzy match score:", score);
    </script>
</body>
</html>
```

### Search Example

```javascript
import init, { search_local } from './wasm/hippo_wasm.js';

await init();

// Your memories data (from API or cache)
const memories = [
    {
        id: "1",
        path: "/photos/beach.jpg",
        title: "Beach Vacation",
        tags: ["beach", "summer"],
        file_size: 1024000,
        modified_at: "2025-01-15T10:00:00Z",
        kind: "image"
    },
    // ... more memories
];

// Perform client-side search
const memoriesJson = JSON.stringify(memories);
const resultsJson = search_local(memoriesJson, "beach");
const results = JSON.parse(resultsJson);

console.log("Search results:", results);
// Results include score and highlights for each match
```

### Fuzzy Matching

```javascript
import { fuzzy_match } from './wasm/hippo_wasm.js';

// Returns 0.0 to 1.0, where 1.0 is exact match
const score1 = fuzzy_match("hello", "hello");     // 1.0
const score2 = fuzzy_match("hello", "helo");      // 0.8
const score3 = fuzzy_match("hello", "world");     // 0.0
```

### Semantic Similarity

```javascript
import { semantic_score } from './wasm/hippo_wasm.js';

const embedding1 = new Float32Array([0.1, 0.2, 0.3, 0.4]);
const embedding2 = new Float32Array([0.15, 0.25, 0.35, 0.45]);

// Returns -1.0 to 1.0 (cosine similarity)
const similarity = semantic_score(embedding1, embedding2);
```

### Filtering and Sorting

```javascript
import { filter_by_type, sort_memories } from './wasm/hippo_wasm.js';

// Filter by type
const imagesJson = filter_by_type(memoriesJson, "image");
const images = JSON.parse(imagesJson);

// Sort by name (ascending)
const sortedJson = sort_memories(memoriesJson, "name", true);
const sorted = JSON.parse(sortedJson);

// Sort by size (descending)
const sortedJson2 = sort_memories(memoriesJson, "size", false);
```

## API Reference

### `fuzzy_match(query: string, text: string) -> number`
Calculate fuzzy match score using Levenshtein distance.
- Returns: 0.0 to 1.0 (1.0 = exact match)

### `semantic_score(embedding1: Float32Array, embedding2: Float32Array) -> number`
Calculate cosine similarity between two embedding vectors.
- Returns: -1.0 to 1.0 (1.0 = identical direction)

### `search_local(memories_json: string, query: string) -> string`
Search memories with fuzzy matching and scoring.
- Input: JSON array of WasmMemory objects
- Returns: JSON array of WasmSearchResult objects

### `filter_by_type(memories_json: string, kind: string) -> string`
Filter memories by type.
- kind: "image", "video", "audio", "document", "code", etc.
- Returns: JSON array of filtered memories

### `sort_memories(memories_json: string, field: string, ascending: boolean) -> string`
Sort memories by field.
- field: "name", "size", or "date"
- Returns: JSON array of sorted memories

### `get_stats(memories_json: string) -> string`
Get statistics about memories.
- Returns: JSON object with counts and totals

## Type Definitions

### WasmMemory
```typescript
interface WasmMemory {
    id: string;
    path: string;
    title?: string;
    tags: string[];
    file_size: number;
    modified_at: string;  // ISO 8601
    kind: string;
}
```

### WasmSearchResult
```typescript
interface WasmSearchResult {
    memory: WasmMemory;
    score: number;
    highlights: WasmHighlight[];
}
```

### WasmHighlight
```typescript
interface WasmHighlight {
    field: string;
    snippet: string;
}
```

## Performance

- **Bundle Size**: Target under 500KB gzipped
- **Optimizations**:
  - Link-time optimization (LTO)
  - Size optimization (`opt-level = "z"`)
  - wasm-opt post-processing
  - Debug symbols stripped

## Browser Compatibility

Works in all modern browsers that support WebAssembly:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## Use Cases

1. **Offline Search**: Search cached memories without server round-trips
2. **Instant Filtering**: Client-side filtering for real-time results
3. **Fuzzy Autocomplete**: Type-ahead with typo tolerance
4. **Client-side Ranking**: Re-rank server results with custom scoring
5. **Progressive Enhancement**: Fallback to server API if WASM fails

## Example: Hybrid Search

Combine server and client-side search for best results:

```javascript
// 1. Fetch initial results from server
const serverResults = await fetch('/api/search?q=beach').then(r => r.json());

// 2. Cache results in localStorage
localStorage.setItem('cached_memories', JSON.stringify(serverResults));

// 3. Use WASM for instant client-side re-ranking
const cachedMemories = localStorage.getItem('cached_memories');
const localResults = search_local(cachedMemories, query);

// 4. Display instantly, then update from server in background
displayResults(localResults);

// Later: update cache when server responds
fetchServerResults(query).then(updateCache);
```

## License

MIT
