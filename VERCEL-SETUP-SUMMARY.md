# Vercel Deployment Setup - Summary

This document summarizes the Vercel deployment setup for the Hippo demo.

## What Was Created

### Configuration Files (Project Root)

1. **vercel.json** - Vercel deployment configuration
   - Serves the `demo/` directory
   - Routes API calls to JSON files
   - Adds security headers
   - Configures rewrites for API endpoints

2. **package.json** - Node.js package metadata
   - Basic project info for Vercel
   - Scripts for local development

3. **.vercelignore** - Files to exclude from deployment
   - Source code (only deploys demo)
   - Build artifacts
   - Development files

4. **DEPLOYMENT.md** - Comprehensive deployment guide
   - Desktop app deployment
   - Web server deployment
   - Static demo deployment (Vercel, Netlify, etc.)
   - Docker deployment
   - Troubleshooting

### Demo Directory Structure

```
demo/
├── index.html                    # Main UI (copied from hippo-web/static)
├── manifest.json                 # PWA manifest
├── hippo-api-wrapper.js         # API compatibility layer
├── README.md                     # Demo documentation
├── QUICKSTART.md                # Quick start guide
├── .gitkeep                     # Keep directory in git
├── api/                         # Mock API responses
│   ├── health.json              # Health check
│   ├── stats.json               # Index statistics
│   ├── sources.json             # Configured sources
│   ├── tags.json                # Available tags
│   ├── search.json              # Sample search results
│   ├── collections.json         # Empty collections
│   ├── qdrant-stats.json        # Vector DB status
│   ├── ollama-status.json       # AI status
│   └── virtual-paths.json       # Virtual paths
└── icons/                       # App icons directory
    └── README.md                # Icon generation instructions
```

## How It Works

### 1. API Wrapper (hippo-api-wrapper.js)

The wrapper provides compatibility between Tauri's `invoke()` and REST API:

```javascript
// Auto-detects environment
const isTauri = window.__TAURI__ && window.__TAURI__.core;

// Creates unified invoke() function
window.invoke = isTauri
  ? window.__TAURI__.core.invoke
  : createWebInvoke();

// Maps commands to API endpoints
invoke('search', { query: 'test' })
  → fetch('/api/search.json?q=test')
```

### 2. Mock Data

Static JSON files in `demo/api/` provide sample data:

- **247 sample memories** across different types
- **10 tags** with usage counts
- **3 sources** (Documents, Pictures, Code)
- **Realistic metadata** (file sizes, dates, EXIF data)

### 3. UI Compatibility

The same UI works in both modes:

- **Desktop mode** (Tauri): Full file system access, real indexing
- **Web demo mode**: Static data, read-only interface
- Automatic detection and fallback

### 4. Demo Banner

In web mode, adds a banner:
- Indicates demo status
- Links to full desktop app
- Styled to match Hippo theme

## Deployment Instructions

### Vercel (Recommended)

```bash
# One-time setup
npm install -g vercel

# Deploy
vercel

# Production
vercel --prod
```

### GitHub Integration

1. Push code to GitHub
2. Connect repository to Vercel
3. Vercel auto-deploys on push

### Custom Domain

In Vercel dashboard:
1. Go to Project Settings → Domains
2. Add custom domain
3. Update DNS records

## Features in Demo

### Working Features

- Search interface
- Tag filtering (click tags or use Tab key)
- File type filters (Images, Videos, Audio, Code, Documents)
- Sort options (Newest, Oldest, Name, Size)
- Dark mode toggle
- Responsive design
- PWA capabilities (installable)
- Sample data browsing

### Limited Features

- Read-only (no file indexing)
- Static sample data
- No file uploads
- No real-time updates
- No AI features
- No file system access
- Disabled source management buttons

## Customization Guide

### Change Sample Data

Edit JSON files in `demo/api/`:

```json
// demo/api/stats.json
{
  "total_memories": 500,  // Change counts
  "by_kind": {
    "Image": 300,
    "Video": 100,
    // ...
  }
}
```

### Update Branding

Edit `demo/index.html`:

```html
<!-- Change title -->
<title>Your Brand - File Organizer</title>

<!-- Update meta tags -->
<meta property="og:title" content="Your Brand">

<!-- Modify CSS variables for colors -->
<style>
  :root {
    --primary-color: #your-color;
  }
</style>
```

### Connect to Real API

Edit `demo/hippo-api-wrapper.js`:

```javascript
const API_BASE = 'https://your-api.com/api';
```

Then redeploy.

## Testing Locally

### Quick Test

```bash
cd demo
python3 -m http.server 8080
# Open http://localhost:8080
```

### Test with Live Reload

```bash
# Using browser-sync (install first)
npm install -g browser-sync
browser-sync start --server demo --files "demo/**/*"
```

### Test API Calls

```bash
# Check mock API responses
curl http://localhost:8080/api/health.json
curl http://localhost:8080/api/stats.json
curl http://localhost:8080/api/search.json
```

## Performance Optimization

### Vercel Configuration

The `vercel.json` includes:

- **Output directory**: `demo/` for minimal deployment size
- **Security headers**: X-Frame-Options, X-Content-Type-Options, etc.
- **Clean URLs**: No trailing slashes
- **API rewrites**: Direct JSON file serving

### Caching

Vercel automatically caches:
- Static files (HTML, JS, CSS)
- JSON responses
- Images and icons

### Edge Network

Deployed to Vercel's global CDN:
- Sub-100ms response times worldwide
- Automatic HTTPS
- DDoS protection

## Maintenance

### Update Sample Data

1. Edit JSON files in `demo/api/`
2. Commit changes
3. Push to GitHub (auto-deploys if connected)

Or manually:

```bash
vercel --prod
```

### Update UI

1. Modify `demo/index.html`
2. Test locally
3. Deploy

### Monitor Usage

Check Vercel dashboard for:
- Deployment status
- Bandwidth usage
- Error rates
- Geographic distribution

## Troubleshooting

### Deployment Fails

```bash
# Check vercel.json syntax
cat vercel.json | jq .

# Check build logs
vercel logs
```

### Icons Not Loading

Add placeholder icons or copy from `hippo-web/static/icons/`:

```bash
# If you have icons
cp -r hippo-web/static/icons/* demo/icons/
```

### API Calls Failing

1. Check browser console (F12)
2. Verify JSON files exist in `demo/api/`
3. Check CORS headers in `vercel.json`

### Styling Issues

1. Clear browser cache
2. Hard reload (Ctrl+Shift+R or Cmd+Shift+R)
3. Check CSS in `demo/index.html`

## Next Steps

### For Production Use

1. **Add authentication**: Use Vercel Edge Middleware
2. **Connect real backend**: Deploy hippo-web separately
3. **Enable analytics**: Add Vercel Analytics
4. **Custom domain**: Configure in Vercel settings
5. **Environment variables**: Set in Vercel dashboard

### For Development

1. **Test different data**: Modify `demo/api/*.json`
2. **UI improvements**: Edit `demo/index.html`
3. **Add features**: Extend `hippo-api-wrapper.js`
4. **Mobile testing**: Use Vercel preview deployments

## Resources

- **Vercel Docs**: https://vercel.com/docs
- **Hippo Repo**: https://github.com/greplabs/hippo
- **Demo README**: `demo/README.md`
- **Deployment Guide**: `DEPLOYMENT.md`

## Support

- Report issues: https://github.com/greplabs/hippo/issues
- Discussions: https://github.com/greplabs/hippo/discussions
- Vercel Support: https://vercel.com/support

---

## Files Created Summary

### Root Level
- `vercel.json` - Vercel configuration
- `package.json` - npm package metadata
- `.vercelignore` - Deployment exclusions
- `DEPLOYMENT.md` - Full deployment guide
- `VERCEL-SETUP-SUMMARY.md` - This file

### Demo Directory
- `demo/index.html` - Main UI
- `demo/manifest.json` - PWA manifest
- `demo/hippo-api-wrapper.js` - API wrapper
- `demo/README.md` - Demo docs
- `demo/QUICKSTART.md` - Quick start
- `demo/.gitkeep` - Git directory marker

### Mock API
- `demo/api/health.json`
- `demo/api/stats.json`
- `demo/api/sources.json`
- `demo/api/tags.json`
- `demo/api/search.json`
- `demo/api/collections.json`
- `demo/api/qdrant-stats.json`
- `demo/api/ollama-status.json`
- `demo/api/virtual-paths.json`

### Documentation
- `demo/icons/README.md` - Icon instructions

## Updated Files

- `README.md` - Added Vercel deployment section

---

**Total**: 24 new files, 1 updated file

Ready to deploy with: `vercel --prod`
