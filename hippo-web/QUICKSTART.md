# Hippo Web API - Quick Start Guide

Get up and running with Hippo Web API in 5 minutes.

## Prerequisites

- Rust 1.70+ installed
- Qdrant running on localhost:6334 (optional, for vector search)

## Step 1: Build and Run

```bash
cd hippo-web
cargo run
```

The server will start on `http://127.0.0.1:3000`

## Step 2: Test the API

### Using curl

```bash
# Health check
curl http://127.0.0.1:3000/api/health

# Get stats
curl http://127.0.0.1:3000/api/stats

# Search all memories
curl http://127.0.0.1:3000/api/search?limit=10

# List tags
curl http://127.0.0.1:3000/api/tags
```

### Using the example scripts

**Bash:**
```bash
chmod +x examples/client.sh
./examples/client.sh
```

**Python:**
```bash
pip install requests
python examples/client.py
```

## Step 3: Add Your First Source

```bash
curl -X POST http://127.0.0.1:3000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "sourceType": "Local",
    "path": "/Users/you/Photos"
  }'
```

The indexer will start processing files in the background.

## Step 4: Search Your Files

```bash
# Search for images
curl "http://127.0.0.1:3000/api/search?type=Image&limit=10"

# Search with text
curl "http://127.0.0.1:3000/api/search?q=vacation&limit=10"

# Search with tags
curl "http://127.0.0.1:3000/api/search?tags=important,family&limit=10"
```

## Step 5: Work with Tags

```bash
# Get a memory ID from search results
MEMORY_ID="550e8400-e29b-41d4-a716-446655440000"

# Add a tag
curl -X POST "http://127.0.0.1:3000/api/memories/$MEMORY_ID/tags" \
  -H "Content-Type: application/json" \
  -d '{"tag": "important"}'

# Remove a tag
curl -X DELETE "http://127.0.0.1:3000/api/memories/$MEMORY_ID/tags/important"
```

## Common Use Cases

### Build a Web Dashboard

```javascript
// Fetch stats for dashboard
fetch('http://127.0.0.1:3000/api/stats')
  .then(r => r.json())
  .then(stats => {
    console.log(`Total files: ${stats.total_memories}`);
    console.log('By type:', stats.by_kind);
  });
```

### Search Interface

```javascript
// Search with filters
const params = new URLSearchParams({
  q: 'vacation',
  tags: 'beach,family',
  type: 'Image',
  sort: 'DateNewest',
  limit: 20
});

fetch(`http://127.0.0.1:3000/api/search?${params}`)
  .then(r => r.json())
  .then(results => {
    results.memories.forEach(item => {
      console.log(item.memory.path, item.score);
    });
  });
```

### Display Thumbnails

```html
<!-- In your HTML -->
<img src="http://127.0.0.1:3000/api/thumbnails/550e8400-e29b-41d4-a716-446655440000"
     alt="Thumbnail">
```

## Configuration

Set environment variables before running:

```bash
# Custom port
export HIPPO_PORT=8080

# Custom host
export HIPPO_HOST=0.0.0.0

# Custom data directory
export HIPPO_DATA_DIR=/path/to/data

cargo run
```

## Troubleshooting

### "Connection refused"
- Make sure the server is running: `cargo run`
- Check the port is correct (default: 3000)

### "Qdrant connection failed"
- Hippo can run without Qdrant, but vector search won't work
- Install Qdrant: https://qdrant.tech/documentation/quick-start/
- Or run with Docker: `docker run -p 6334:6334 qdrant/qdrant`

### No results in search
- Add a source first: `POST /api/sources`
- Wait for indexing to complete
- Check stats: `GET /api/stats`

## Next Steps

- Read the full [API Documentation](README.md)
- Explore [Integration Tests](tests/api_tests.rs)
- Build a frontend with React/Vue/Svelte
- Set up authentication middleware
- Deploy to production

## Examples

See `examples/` directory for:
- `client.sh` - Bash client examples
- `client.py` - Python client library

## Need Help?

- Check the [README](README.md) for full API documentation
- Review the [tests](tests/api_tests.rs) for usage examples
- Open an issue on GitHub

Happy organizing!
