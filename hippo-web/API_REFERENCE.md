# Hippo Web API Reference

Complete REST API reference for Hippo Web API v0.1.0

## Base URL

```
http://localhost:3000/api
```

## Authentication

Currently no authentication required. In production, implement authentication middleware.

## Response Format

All responses are JSON unless specified otherwise.

### Success Response
```json
{
  "data": { ... }
}
```

### Error Response
```json
{
  "error": "Error message description"
}
```

## HTTP Status Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 404 | Not Found - Resource doesn't exist |
| 500 | Internal Server Error |

---

## Endpoints

### Health & Monitoring

#### `GET /api/health`

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

**Status Codes:** 200

---

#### `GET /api/stats`

Get index statistics.

**Response:**
```json
{
  "total_memories": 1234,
  "by_kind": {
    "Image": 456,
    "Video": 78,
    "Audio": 12,
    "Code": 200,
    "Document": 100
  },
  "by_source": {
    "Local": 1234
  },
  "total_size_bytes": 123456789,
  "index_size_bytes": 12345,
  "last_updated": "2025-12-20T10:30:00Z"
}
```

**Status Codes:** 200, 500

---

### Search

#### `GET /api/search`

Search memories with filters and pagination.

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| q | string | No | Search text (semantic/keyword) |
| tags | string | No | Comma-separated tags (prefix `-` to exclude) |
| type | string | No | Memory kind filter |
| sort | string | No | Sort order |
| limit | integer | No | Max results (default: 50) |
| offset | integer | No | Pagination offset (default: 0) |

**Valid `type` values:**
- `Image`
- `Video`
- `Audio`
- `Code`
- `Document`
- `Spreadsheet`
- `Presentation`
- `Archive`
- `Database`
- `Folder`
- `Unknown`

**Valid `sort` values:**
- `Relevance` (default)
- `DateNewest`
- `DateOldest`
- `NameAsc`
- `NameDesc`
- `SizeAsc`
- `SizeDesc`

**Examples:**

```bash
# Simple text search
GET /api/search?q=vacation

# Search with tag filters
GET /api/search?tags=important,family,-private

# Search images only, sorted by date
GET /api/search?type=Image&sort=DateNewest&limit=20

# Combined search
GET /api/search?q=beach&tags=vacation&type=Image&sort=DateNewest
```

**Response:**
```json
{
  "memories": [
    {
      "memory": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "path": "/Users/you/Photos/sunset.jpg",
        "source": {
          "Local": {
            "root_path": "/Users/you/Photos"
          }
        },
        "kind": {
          "Image": {
            "width": 1920,
            "height": 1080,
            "format": "JPEG"
          }
        },
        "metadata": {
          "size_bytes": 123456,
          "mime_type": "image/jpeg",
          "title": "sunset.jpg",
          "camera": "Canon EOS 5D",
          "location": {
            "latitude": 37.7749,
            "longitude": -122.4194
          }
        },
        "tags": [
          {
            "name": "vacation",
            "kind": "User"
          }
        ],
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
  "suggested_tags": ["vacation", "beach", "sunset"],
  "clusters": []
}
```

**Status Codes:** 200, 400, 500

---

### Memories

#### `GET /api/memories/:id`

Get a single memory by UUID.

**Path Parameters:**
- `id` - UUID of the memory

**Example:**
```bash
GET /api/memories/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "path": "/Users/you/Photos/sunset.jpg",
  "source": {
    "Local": {
      "root_path": "/Users/you/Photos"
    }
  },
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
  "tags": [],
  "is_favorite": false,
  "created_at": "2025-12-20T10:30:00Z",
  "modified_at": "2025-12-20T10:30:00Z",
  "indexed_at": "2025-12-20T11:00:00Z"
}
```

**Status Codes:** 200, 400, 404, 500

---

#### `GET /api/thumbnails/:id`

Get thumbnail image for a memory.

**Path Parameters:**
- `id` - UUID of the memory

**Example:**
```bash
GET /api/thumbnails/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
- Content-Type: `image/jpeg`
- Body: JPEG image data (256x256 pixels)

**Notes:**
- Only works for Image and Video memory types
- Thumbnails are generated on-demand and cached
- Videos extract frame at 10% duration

**Status Codes:** 200, 400, 404, 500

---

### Sources

#### `GET /api/sources`

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
    "exclude_patterns": ["*.tmp", ".DS_Store"]
  }
]
```

**Status Codes:** 200, 500

---

#### `POST /api/sources`

Add a new source to index.

**Request Body:**
```json
{
  "sourceType": "Local",
  "path": "/Users/you/Documents"
}
```

**Fields:**
- `sourceType` - Currently only "Local" is supported
- `path` - Absolute path to the directory to index

**Example:**
```bash
curl -X POST http://localhost:3000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "sourceType": "Local",
    "path": "/Users/you/Documents"
  }'
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

**Status Codes:** 200, 400, 500

---

#### `DELETE /api/sources/:path`

Remove a source from indexing.

**Path Parameters:**
- `path` - URL-encoded path of the source to remove

**Query Parameters:**
- `deleteFiles` (optional) - If "true", also delete indexed memories (default: false)

**Examples:**
```bash
# Remove source but keep indexed data
DELETE /api/sources/Users/you/Documents

# Remove source and delete all indexed memories
DELETE /api/sources/Users/you/Documents?deleteFiles=true
```

**Response:**
```json
{
  "success": true,
  "message": "Source removed successfully"
}
```

**Status Codes:** 200, 400, 500

---

### Tags

#### `GET /api/tags`

List all tags with usage counts.

**Response:**
```json
[
  {
    "name": "vacation",
    "count": 42
  },
  {
    "name": "work",
    "count": 128
  },
  {
    "name": "family",
    "count": 56
  }
]
```

**Status Codes:** 200, 500

---

#### `POST /api/memories/:id/tags`

Add a tag to a memory.

**Path Parameters:**
- `id` - UUID of the memory

**Request Body:**
```json
{
  "tag": "important"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/memories/550e8400-e29b-41d4-a716-446655440000/tags \
  -H "Content-Type: application/json" \
  -d '{"tag": "important"}'
```

**Response:**
```json
{
  "success": true,
  "message": "Tag added successfully"
}
```

**Status Codes:** 200, 400, 404, 500

---

#### `DELETE /api/memories/:id/tags/:tag`

Remove a tag from a memory.

**Path Parameters:**
- `id` - UUID of the memory
- `tag` - Name of the tag to remove (URL-encoded)

**Example:**
```bash
DELETE /api/memories/550e8400-e29b-41d4-a716-446655440000/tags/important
```

**Response:**
```json
{
  "success": true,
  "message": "Tag removed successfully"
}
```

**Status Codes:** 200, 400, 404, 500

---

## Data Models

### Memory

```typescript
{
  id: string (UUID),
  path: string,
  source: Source,
  kind: MemoryKind,
  metadata: MemoryMetadata,
  tags: Tag[],
  embedding_id?: string,
  connections: Connection[],
  is_favorite: boolean,
  created_at: string (ISO 8601),
  modified_at: string (ISO 8601),
  indexed_at: string (ISO 8601)
}
```

### MemoryKind (variants)

```typescript
// Images
{ Image: { width: number, height: number, format: string } }

// Videos
{ Video: { duration_ms: number, format: string } }

// Audio
{ Audio: { duration_ms: number, format: string } }

// Documents
{ Document: { format: DocumentFormat, page_count?: number } }

// Code
{ Code: { language: string, lines: number } }

// Other types
{ Spreadsheet: { sheet_count: number } }
{ Presentation: { slide_count: number } }
{ Archive: { item_count: number } }
{ Database: {} }
{ Folder: {} }
{ Unknown: {} }
```

### Source

```typescript
{ Local: { root_path: string } }
{ GoogleDrive: { account_id: string } }
{ ICloud: { account_id: string } }
{ Dropbox: { account_id: string } }
{ OneDrive: { account_id: string } }
{ S3: { bucket: string, region: string } }
{ Custom: { name: string } }
```

### Tag

```typescript
{
  name: string,
  kind: "User" | "Auto" | "System"
}
```

---

## Rate Limiting

Currently no rate limiting. For production use, implement rate limiting middleware.

## CORS

CORS is enabled for all origins in development. For production:
- Restrict allowed origins
- Limit allowed methods
- Set appropriate headers

## WebSocket Support

Not currently implemented. Future enhancement for real-time updates.

---

## Client Libraries

### JavaScript/TypeScript

```typescript
class HippoClient {
  constructor(private baseUrl: string = 'http://localhost:3000/api') {}

  async search(params: SearchParams): Promise<SearchResults> {
    const query = new URLSearchParams(params);
    const response = await fetch(`${this.baseUrl}/search?${query}`);
    return response.json();
  }

  // ... other methods
}
```

### Python

See `examples/client.py` for full implementation.

```python
from hippo_client import HippoClient

client = HippoClient('http://localhost:3000/api')
results = client.search(query='vacation', tags=['beach'], limit=10)
```

### Curl

See `examples/client.sh` for examples.

---

## Changelog

### v0.1.0 (2025-12-20)

Initial release:
- Health check endpoint
- Search with filters
- Memory retrieval
- Thumbnail serving
- Source management
- Tag management
- CORS support
- Comprehensive error handling

---

## Future Endpoints (Planned)

- `POST /api/memories/:id/favorite` - Toggle favorite status
- `GET /api/collections` - List virtual collections
- `POST /api/collections` - Create collection
- `GET /api/duplicates` - Find duplicate files
- `POST /api/analyze` - AI analysis of files
- `GET /api/graph/:id` - Knowledge graph
- `POST /api/export` - Export index
- `POST /api/import` - Import index
- `GET /api/metrics` - Prometheus metrics
- `WS /api/ws` - WebSocket for real-time updates

---

For more information, see:
- [README.md](README.md) - Full documentation
- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [examples/](examples/) - Client examples
