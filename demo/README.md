# Hippo Web Demo

This directory contains a deployable web demo of Hippo that works with static JSON files.

## What's included

- **index.html** - Full Hippo web UI (PWA-enabled)
- **hippo-api-wrapper.js** - API compatibility layer for demo mode
- **api/** - Mock API responses with sample data
- **icons/** - App icons and assets

## How it works

The demo uses a JavaScript wrapper (`hippo-api-wrapper.js`) that:
- Detects if running in Tauri desktop mode or web demo mode
- Provides a compatible `invoke()` function that fetches from static JSON files
- Displays a banner indicating this is a demo version
- Falls back gracefully when API endpoints are unavailable

## Deployment Options

### Vercel (Recommended)

From the project root:

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy
vercel

# Or deploy for production
vercel --prod
```

The `vercel.json` configuration is already set up to serve the demo directory.

### Netlify

Create a `netlify.toml` in the project root:

```toml
[build]
  publish = "demo"
  command = "echo 'No build needed'"

[[redirects]]
  from = "/api/*"
  to = "/api/:splat"
  status = 200
```

Then:

```bash
netlify deploy --dir=demo --prod
```

### Static Web Server

Any static file server will work:

```bash
# Python
cd demo
python3 -m http.server 8080

# Node.js
npx http-server demo -p 8080

# PHP
cd demo
php -S localhost:8080
```

## Customization

### Update Sample Data

Edit the JSON files in `api/` to customize the demo:

- `api/stats.json` - Index statistics
- `api/sources.json` - Configured sources
- `api/tags.json` - Available tags
- `api/search.json` - Sample search results

### Connect to Real Backend

To connect to a live Hippo Web API server, modify `hippo-api-wrapper.js`:

```javascript
const API_BASE = 'https://your-hippo-server.com/api';
```

## Features

The demo includes:
- Search interface
- Tag filtering
- File type filters
- Sample data with images, videos, code, and documents
- Dark mode support
- Responsive design
- PWA capabilities (installable)

## Limitations

As a static demo, the following features are disabled:

- File indexing (read-only sample data)
- Adding/removing sources
- Uploading files
- Real-time file system access
- Thumbnail generation
- Opening files in system apps

## License

MIT - see LICENSE file in the project root
