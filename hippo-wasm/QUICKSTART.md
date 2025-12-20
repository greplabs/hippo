# Quick Start: hippo-wasm

Get started with client-side search in 5 minutes.

## Install

```bash
# 1. Install wasm-pack (one time)
cargo install wasm-pack

# 2. Build the WASM module
cd hippo-wasm
./build-wasm.sh
```

## Test

Open `example.html` in your browser:

```bash
# Start a local server
python3 -m http.server 8000

# Open http://localhost:8000/example.html
```

You should see:
- Green border at top (WASM loaded)
- 4 interactive demos
- No console errors

## Integrate

Add to your HTML:

```html
<script type="module">
  import init, { search_local } from './wasm/hippo_wasm.js';

  // Initialize
  await init();

  // Use
  const memories = [
    {
      id: "1",
      path: "/photo.jpg",
      title: "My Photo",
      tags: ["vacation"],
      file_size: 1024000,
      modified_at: "2025-01-15T10:00:00Z",
      kind: "image"
    }
  ];

  const results = search_local(
    JSON.stringify(memories),
    "vacation"
  );

  console.log(JSON.parse(results));
</script>
```

## Common Use Cases

### 1. Instant Search

```javascript
// Debounced search input
let searchTimeout;
searchInput.addEventListener('input', (e) => {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    const results = search_local(cachedMemories, e.target.value);
    displayResults(JSON.parse(results));
  }, 100);
});
```

### 2. Fuzzy Autocomplete

```javascript
import { fuzzy_match } from './wasm/hippo_wasm.js';

function getSuggestions(input, options) {
  return options
    .map(opt => ({ opt, score: fuzzy_match(input, opt) }))
    .filter(({ score }) => score > 0.5)
    .sort((a, b) => b.score - a.score)
    .slice(0, 10)
    .map(({ opt }) => opt);
}
```

### 3. Client-side Filtering

```javascript
import { filter_by_type } from './wasm/hippo_wasm.js';

typeSelect.addEventListener('change', (e) => {
  const filtered = filter_by_type(
    JSON.stringify(allMemories),
    e.target.value
  );
  displayResults(JSON.parse(filtered));
});
```

### 4. Offline Search

```javascript
// Cache on page load
const memories = await fetch('/api/memories').then(r => r.json());
localStorage.setItem('cache', JSON.stringify(memories));

// Use cache when offline
if (!navigator.onLine) {
  const cached = localStorage.getItem('cache');
  const results = search_local(cached, query);
  displayResults(JSON.parse(results));
}
```

## Tips

- **Cache aggressively**: Store results in memory, not localStorage
- **Debounce input**: Wait 100-300ms before searching
- **Limit results**: Use `.slice(0, 100)` to avoid rendering lag
- **Fallback gracefully**: Always have server search as backup
- **Monitor size**: Keep WASM <100KB gzipped for fast load

## Next Steps

- Read [INTEGRATION.md](./INTEGRATION.md) for Tauri integration
- See [VERIFICATION.md](./VERIFICATION.md) for testing
- Check [README.md](./README.md) for full API docs
- Browse [example.html](./example.html) for live demo

## Troubleshooting

**WASM not loading?**
- Check file path is correct (`./wasm/hippo_wasm.js`)
- Serve from HTTP server (file:// won't work)
- Check browser console for errors

**Searches too slow?**
- Limit cached memories to <10,000 items
- Use debouncing (wait before search)
- Profile with browser DevTools

**Too large?**
- Check gzipped size: `gzip -c file.wasm | wc -c`
- Should be <100KB (500KB max)
- Rebuild with optimizations enabled

## Support

- GitHub Issues: Report bugs
- Discussions: Ask questions
- Docs: See full documentation
