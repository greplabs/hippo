# Hippo Web API

REST API server for Hippo - intelligent file memory system built with Axum.

## Features

- **RESTful API** - Clean REST endpoints for all Hippo functionality
- **CORS enabled** - Ready for browser-based clients
- **JSON responses** - All data in JSON format
- **Environment configuration** - Configurable via environment variables
- **Comprehensive error handling** - Proper HTTP status codes and error messages

## Installation

```bash
cd hippo-web
cargo build --release
```

## Configuration

Configure the server using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `HIPPO_HOST` | Server bind address | `127.0.0.1` |
| `HIPPO_PORT` | Server port | `3000` |
| `HIPPO_DATA_DIR` | Data directory for Hippo database | OS-specific |

## Running

### Development

```bash
cargo run
```

### Production

```bash
# Set environment variables
export HIPPO_HOST=0.0.0.0
export HIPPO_PORT=8080
export HIPPO_DATA_DIR=/var/lib/hippo

# Run the server
cargo run --release
```

### With Docker (future)

```bash
docker run -p 3000:3000 -v /path/to/data:/data hippo-web
```

## API Documentation

### Health & Status

#### GET /api/health

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

#### GET /api/stats

Get index statistics.

**Response:**
```json
{
  "total_memories": 1234,
  "by_kind": {
    "Image": 456,
    "Video": 78,
    "Code": 200
  },
  "by_source": {
    "Local": 1234
  },
  "total_size_bytes": 123456789,
  "index_size_bytes": 12345,
  "last_updated": "2025-12-20T10:30:00Z"
}
```

### Search

#### GET /api/search

Search memories with filters.

**Query Parameters:**
- `q` (optional) - Search text for semantic/keyword search
- `tags` (optional) - Comma-separated tags to filter (prefix with `-` to exclude)
- `type` (optional) - Filter by memory kind (Image, Video, Audio, Code, Document)
- `sort` (optional) - Sort order (Relevance, DateNewest, DateOldest, NameAsc, NameDesc, SizeAsc, SizeDesc)
- `limit` (optional) - Maximum results (default: 50)
- `offset` (optional) - Pagination offset (default: 0)

**Examples:**
```bash
# Simple search
GET /api/search?q=sunset

# Search with tags
GET /api/search?q=vacation&tags=beach,family,-private

# Filter by type
GET /api/search?type=Image&sort=DateNewest&limit=20

# Exclude tags
GET /api/search?tags=-private,-draft
```

**Response:**
```json
{
  "memories": [
    {
      "memory": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "path": "/Users/you/Photos/sunset.jpg",
        "kind": {
          "Image": {
            "width": 1920,
            "height": 1080,
            "format": "JPEG"
          }
        },
        "tags": ["vacation", "beach"],
        "is_favorite": false,
        "created_at": "2025-12-20T10:30:00Z",
        "modified_at": "2025-12-20T10:30:00Z",
        "indexed_at": "2025-12-20T11:00:00Z"
      },
      "score": 0.95,
      "highlights": []
    }
  ],
  "total_count": 1,
  "suggested_tags": ["vacation", "beach"],
  "clusters": []
}
```

### Memories

#### GET /api/memories/:id

Get a single memory by UUID.

**Path Parameters:**
- `id` - UUID of the memory

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "path": "/Users/you/Photos/sunset.jpg",
  "kind": {
    "Image": {
      "width": 1920,
      "height": 1080,
      "format": "JPEG"
    }
  },
  "metadata": {
    "size_bytes": 123456,
    "mime_type": "image/jpeg"
  },
  "tags": ["vacation", "beach"],
  "is_favorite": false,
  "created_at": "2025-12-20T10:30:00Z",
  "modified_at": "2025-12-20T10:30:00Z",
  "indexed_at": "2025-12-20T11:00:00Z"
}
```

#### GET /api/thumbnails/:id

Get thumbnail image for a memory.

**Path Parameters:**
- `id` - UUID of the memory

**Response:**
- Content-Type: `image/jpeg`
- Body: JPEG image data (256x256)

**Note:** Only works for images and videos. Returns 404 for other types.

### Sources

#### GET /api/sources

List all configured sources.

**Response:**
```json
[
  {
    "source": {
      "Local": {
        "root_path": "/Users/you/Photos"
      }
    },
    "enabled": true,
    "sync_interval_secs": 3600,
    "last_sync": "2025-12-20T10:00:00Z",
    "include_patterns": [],
    "exclude_patterns": ["*.tmp"]
  }
]
```

#### POST /api/sources

Add a new source to index.

**Request Body:**
```json
{
  "sourceType": "Local",
  "path": "/Users/you/Documents"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Source added successfully",
  "source": {
    "Local": {
      "root_path": "/Users/you/Documents"
    }
  }
}
```

#### DELETE /api/sources/:path

Remove a source.

**Path Parameters:**
- `path` - URL-encoded path of the source

**Query Parameters:**
- `deleteFiles` (optional) - If "true", also delete indexed memories (default: false)

**Example:**
```bash
DELETE /api/sources/Users/you/Documents?deleteFiles=true
```

**Response:**
```json
{
  "success": true,
  "message": "Source removed successfully"
}
```

### Tags

#### GET /api/tags

List all tags with usage counts.

**Response:**
```json
[
  { "name": "vacation", "count": 42 },
  { "name": "work", "count": 128 },
  { "name": "family", "count": 56 }
]
```

#### POST /api/memories/:id/tags

Add a tag to a memory.

**Path Parameters:**
- `id` - UUID of the memory

**Request Body:**
```json
{
  "tag": "important"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Tag added successfully"
}
```

#### DELETE /api/memories/:id/tags/:tag

Remove a tag from a memory.

**Path Parameters:**
- `id` - UUID of the memory
- `tag` - Name of the tag to remove

**Response:**
```json
{
  "success": true,
  "message": "Tag removed successfully"
}
```

## Error Responses

All errors return a JSON object with an `error` field:

```json
{
  "error": "Memory not found"
}
```

**HTTP Status Codes:**
- `200` - Success
- `400` - Bad Request (invalid parameters)
- `404` - Not Found
- `500` - Internal Server Error

## Testing

### Unit Tests

```bash
cargo test -p hippo-web
```

### Integration Tests

The integration tests require a running server instance.

```bash
# Start the server in one terminal
HIPPO_PORT=3001 cargo run

# Run tests in another terminal
HIPPO_TEST_PORT=3001 cargo test -p hippo-web -- --ignored
```

## Development

### Project Structure

```
hippo-web/
├── Cargo.toml          # Dependencies
├── src/
│   └── main.rs         # Server implementation
├── tests/
│   └── api_tests.rs    # Integration tests
└── README.md           # This file
```

### Adding New Endpoints

1. Define the handler function in `src/main.rs`
2. Add route to the router in `main()`
3. Add request/response types as needed
4. Document with OpenAPI-style comments
5. Add integration tests in `tests/api_tests.rs`

### CORS Configuration

CORS is configured to allow all origins, methods, and headers for development. In production, you should restrict this:

```rust
CorsLayer::new()
    .allow_origin("https://your-domain.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::DELETE])
    .allow_headers([header::CONTENT_TYPE])
```

## Deployment

### Systemd Service

Create `/etc/systemd/system/hippo-web.service`:

```ini
[Unit]
Description=Hippo Web API
After=network.target

[Service]
Type=simple
User=hippo
WorkingDirectory=/opt/hippo-web
Environment="HIPPO_HOST=0.0.0.0"
Environment="HIPPO_PORT=8080"
Environment="HIPPO_DATA_DIR=/var/lib/hippo"
ExecStart=/opt/hippo-web/target/release/hippo-web
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable hippo-web
sudo systemctl start hippo-web
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name hippo.example.com;

    location /api {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Security Considerations

- **Authentication:** This API currently has no authentication. Add middleware for auth tokens in production.
- **Rate limiting:** Consider adding rate limiting for public deployments.
- **CORS:** Restrict CORS origins in production.
- **File access:** The server can access any files Hippo has indexed. Ensure proper file permissions.

## Future Enhancements

- [ ] Authentication/Authorization (JWT, API keys)
- [ ] Rate limiting
- [ ] WebSocket support for real-time updates
- [ ] GraphQL endpoint
- [ ] OpenAPI/Swagger specification file
- [ ] Prometheus metrics endpoint
- [ ] Request validation middleware
- [ ] Batch operations endpoints
- [ ] Export/Import endpoints

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- All tests pass
- Code is formatted with `rustfmt`
- New endpoints are documented
- Integration tests are added for new features
