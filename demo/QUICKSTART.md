# Hippo Demo - Quick Start

Get the Hippo demo running in 5 minutes.

## Option 1: Vercel (Easiest)

### One-Click Deploy

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/greplabs/hippo)

### Manual Deploy

```bash
# Install Vercel CLI
npm install -g vercel

# Clone the repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Deploy
vercel

# Or deploy to production
vercel --prod
```

Your demo will be live at `https://your-project.vercel.app`

## Option 2: Local Development

### Python (Simplest)

```bash
cd demo
python3 -m http.server 8080
```

Open http://localhost:8080

### Node.js

```bash
npx http-server demo -p 8080
```

Open http://localhost:8080

### PHP

```bash
cd demo
php -S localhost:8080
```

Open http://localhost:8080

## What You'll See

The demo includes:

- 247 sample files across different types (images, videos, code, documents)
- Search interface with tag filtering
- File type filters (Images, Videos, Audio, Code, Documents)
- Sample tags and sources
- Dark mode support
- Responsive design

## Customizing the Demo

### Change Sample Data

Edit JSON files in `api/`:

```bash
demo/api/
├── health.json       # Health check
├── stats.json        # Index statistics
├── sources.json      # Configured sources
├── tags.json         # Available tags
└── search.json       # Search results
```

### Change Appearance

Edit `index.html` to customize:
- Colors (search for color values in CSS)
- Branding (update title, meta tags)
- Layout (modify HTML structure)

### Connect to Real Backend

To use a live Hippo Web API instead of mock data:

1. Edit `hippo-api-wrapper.js`
2. Change the `API_BASE` constant:
   ```javascript
   const API_BASE = 'https://your-hippo-api.com/api';
   ```
3. Redeploy

## Limitations

This is a static demo with the following limitations:

- Read-only data (no file indexing)
- No file uploads
- No real-time updates
- Sample data only
- No AI features
- No file system access

## Next Steps

Want full functionality?

1. **Desktop App**: Download the full Tauri app for file system access
   ```bash
   git clone https://github.com/greplabs/hippo.git
   cd hippo/hippo-tauri
   cargo run
   ```

2. **Web Server**: Deploy the REST API backend with Docker
   ```bash
   docker-compose up -d
   ```
   See [DEPLOYMENT.md](../DEPLOYMENT.md) for details

3. **CLI**: Use the command-line interface
   ```bash
   cargo build --bin hippo
   ./target/debug/hippo --help
   ```

## Troubleshooting

### Icons Not Loading

The demo needs icon files. If icons are missing:

```bash
# Create placeholder icons (requires ImageMagick)
cd demo/icons
convert -size 192x192 xc:purple -fill white -gravity center \
  -pointsize 120 -annotate +0+0 "H" icon-192.png
```

### API Requests Failing

Check browser console (F12) for errors. Common issues:

- **CORS errors**: API endpoints must allow cross-origin requests
- **404 errors**: JSON files missing from `demo/api/`
- **Network errors**: Check your internet connection

### Dark Mode Not Working

Clear browser cache and reload:
- Chrome: Ctrl+Shift+R (Cmd+Shift+R on Mac)
- Firefox: Ctrl+F5
- Safari: Cmd+Option+R

## Support

- Issues: https://github.com/greplabs/hippo/issues
- Discussions: https://github.com/greplabs/hippo/discussions
- Documentation: See [README.md](../README.md)

## License

MIT - see [LICENSE](../LICENSE)
