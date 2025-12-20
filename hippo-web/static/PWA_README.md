# Hippo Progressive Web App (PWA)

Complete PWA implementation for Hippo, enabling native-like mobile app experience.

## Features

### Core PWA Features
- **Installable**: Can be installed on any device (mobile, tablet, desktop)
- **Offline Support**: Works without internet connection using service workers
- **App-like Experience**: Standalone mode hides browser UI
- **Fast Loading**: Cached assets for instant loading
- **Background Sync**: Queues operations when offline, syncs when back online
- **Push Notifications**: Ready for notification support (stub implementation)

### Caching Strategies
- **Static Assets**: Cache-first strategy for CSS, JS, fonts
- **Images**: Cache-first with expiration (7 days)
- **API Calls**: Network-first with offline fallback
- **Dynamic Content**: Stale-while-revalidate for optimal UX

### Mobile Optimizations
- **Responsive Design**: Adapts to any screen size
- **Touch Optimized**: Mobile-friendly interactions
- **Install Prompts**: Smart installation banners
- **Share Target**: Receive files shared from other apps
- **File Handler**: Open files directly with Hippo

## Quick Start

### 1. Generate Icons

```bash
cd static/icons
./generate-icons.sh
```

**Requirements**: Install one of these tools:
- `librsvg`: `brew install librsvg` (macOS) or `sudo apt install librsvg2-bin` (Linux)
- `ImageMagick`: `brew install imagemagick` or `sudo apt install imagemagick`
- `Inkscape`: Download from https://inkscape.org/

### 2. Serve the PWA

Run the hippo-web server:

```bash
cd hippo-web
cargo run
# Server runs on http://localhost:3000
```

Or use a simple static server:

```bash
cd static
python3 -m http.server 8000
# Visit http://localhost:8000
```

### 3. Test PWA Features

1. Open Chrome DevTools → Application tab
2. Check "Manifest" to verify manifest.json is loaded
3. Check "Service Workers" to see if sw.js is registered
4. Test offline mode:
   - Open Network tab
   - Check "Offline" checkbox
   - Reload the page
   - Should show cached content or offline page

### 4. Install the PWA

**Desktop (Chrome/Edge):**
- Click the install icon in the address bar (⊕)
- Or use "Install Hippo..." from the menu

**Mobile (Android):**
- Tap the "Add to Home Screen" prompt
- Or use Chrome menu → "Install app"

**Mobile (iOS):**
- Tap Share button
- Select "Add to Home Screen"

## Files Overview

### Core PWA Files

| File | Purpose |
|------|---------|
| `manifest.json` | PWA configuration, icons, theme colors |
| `sw.js` | Service worker with caching strategies |
| `index.html` | Main app page with PWA features |
| `offline.html` | Fallback page when completely offline |
| `browserconfig.xml` | Microsoft tiles configuration |

### Icons Directory

| File | Size | Purpose |
|------|------|---------|
| `icon.svg` | Vector | Source icon (editable) |
| `icon-192.png` | 192x192 | PWA manifest icon |
| `icon-512.png` | 512x512 | PWA splash screen |
| `apple-touch-icon.png` | 180x180 | iOS home screen |
| `favicon.ico` | Multi-size | Browser tab icon |
| `favicon-16.png` | 16x16 | Small favicon |
| `favicon-32.png` | 32x32 | Standard favicon |

## Lighthouse Audit

Test your PWA with Google Lighthouse:

1. Open Chrome DevTools
2. Go to Lighthouse tab
3. Select "Progressive Web App" category
4. Click "Generate report"

**Target Scores:**
- Performance: 90+
- Accessibility: 90+
- Best Practices: 90+
- SEO: 90+
- PWA: 100

### PWA Checklist

✅ Uses HTTPS (or localhost for testing)
✅ Registers a service worker
✅ Responds with 200 when offline
✅ Provides a web app manifest
✅ Has a viewport meta tag
✅ Contains icons in the manifest
✅ Themed address bar
✅ Content sized correctly for viewport
✅ Page load fast on mobile networks
✅ Redirects HTTP to HTTPS
✅ Configured for custom splash screen
✅ Sets an address bar theme color

## Service Worker Caching

### Cache Configuration

Edit `sw.js` to customize caching:

```javascript
// Version - increment to force cache update
const CACHE_VERSION = 'hippo-v1.0.0';

// Maximum cache sizes
const MAX_CACHE_SIZE = {
  'hippo-v1.0.0-dynamic': 50,
  'hippo-v1.0.0-images': 100,
  'hippo-v1.0.0-api': 30
};

// Cache expiration times
const CACHE_EXPIRATION = {
  'hippo-v1.0.0-api': 5 * 60 * 1000,        // 5 minutes
  'hippo-v1.0.0-images': 7 * 24 * 60 * 60 * 1000,  // 7 days
  'hippo-v1.0.0-dynamic': 24 * 60 * 60 * 1000      // 24 hours
};
```

### Caching Strategies

**Network First** (for API calls):
1. Try network request
2. If successful, cache and return
3. If failed, return cached version
4. If no cache, return error

**Cache First** (for images):
1. Check cache
2. If cached and not expired, return
3. Otherwise, fetch from network
4. Cache the result

**Stale While Revalidate** (for dynamic content):
1. Return cached version immediately
2. Fetch new version in background
3. Update cache for next time

### Manual Cache Control

From the browser console:

```javascript
// List all caches
caches.keys().then(console.log);

// Clear all caches
caches.keys().then(keys => {
  keys.forEach(key => caches.delete(key));
});

// Clear specific cache
caches.delete('hippo-v1.0.0-api');

// Force service worker update
navigator.serviceWorker.getRegistrations().then(regs => {
  regs.forEach(reg => reg.update());
});
```

## Offline Features

### Offline Detection

The app automatically detects online/offline status and shows a banner:

```javascript
// Listen for online/offline events
window.addEventListener('online', () => {
  console.log('Back online!');
  // Sync pending operations
});

window.addEventListener('offline', () => {
  console.log('Gone offline');
  // Show offline UI
});

// Check current status
if (navigator.onLine) {
  console.log('Currently online');
}
```

### Background Sync

Queue operations for later syncing:

```javascript
// Register a sync event
navigator.serviceWorker.ready.then(registration => {
  return registration.sync.register('sync-searches');
});

// Service worker will trigger when back online
// See sw.js for implementation
```

### Offline Data Storage

Use IndexedDB for offline data:

```javascript
// Open database
const request = indexedDB.open('HippoDB', 1);

request.onsuccess = (event) => {
  const db = event.target.result;
  // Use database
};

// Store search query for later
function queueSearch(query) {
  // Store in IndexedDB
  // Service worker will sync when online
}
```

## Install Prompt

### Customize Install Prompt

Edit `index.html` to customize the install prompt:

```javascript
// Trigger conditions
const dismissed = localStorage.getItem('pwa-install-dismissed');
if (!dismissed) {
  // Show after 5 seconds
  setTimeout(() => {
    installPrompt.classList.add('show');
  }, 5000);
}

// Dismiss for 7 days
installDismiss.addEventListener('click', () => {
  installPrompt.classList.remove('show');
  localStorage.setItem('pwa-install-dismissed', 'true');

  setTimeout(() => {
    localStorage.removeItem('pwa-install-dismissed');
  }, 7 * 24 * 60 * 60 * 1000);
});
```

### Track Installation

```javascript
window.addEventListener('appinstalled', () => {
  console.log('PWA was installed');

  // Track with analytics
  if (window.analytics) {
    window.analytics.track('PWA Installed');
  }
});
```

## Customization

### Update App Metadata

Edit `manifest.json`:

```json
{
  "name": "Your App Name",
  "short_name": "App",
  "description": "Your description",
  "theme_color": "#6366f1",
  "background_color": "#f8f8f7",
  "start_url": "/",
  "display": "standalone"
}
```

### Customize Icons

1. Edit `icons/icon.svg` with your design
2. Run `./generate-icons.sh` to regenerate PNGs
3. Update theme colors in manifest.json

### Add Custom Routes

In `sw.js`, add caching for new routes:

```javascript
self.addEventListener('fetch', (event) => {
  const url = new URL(request.url);

  // Custom route handling
  if (url.pathname.startsWith('/custom-route/')) {
    event.respondWith(customCachingStrategy(request));
  }
});
```

## Integration with Hippo REST API

The PWA can work with the hippo-web REST API:

### API Configuration

```javascript
const API_BASE = 'http://localhost:3000/api';

async function searchMemories(query) {
  try {
    const response = await fetch(`${API_BASE}/search?q=${query}`);
    const data = await response.json();
    return data.memories;
  } catch (error) {
    // Check cache for offline support
    const cachedResponse = await caches.match(`${API_BASE}/search?q=${query}`);
    if (cachedResponse) {
      return await cachedResponse.json();
    }
    throw error;
  }
}
```

### Offline Queue

Queue API calls when offline:

```javascript
async function addTag(memoryId, tag) {
  if (!navigator.onLine) {
    // Queue for later
    await queueOperation({
      type: 'add-tag',
      memoryId,
      tag,
      timestamp: Date.now()
    });
    return;
  }

  // Execute immediately
  await fetch(`${API_BASE}/memories/${memoryId}/tags`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tag })
  });
}
```

## Deployment

### GitHub Pages

1. Copy `static/` contents to your repository
2. Enable GitHub Pages in repository settings
3. Select branch and root folder
4. Access at `https://yourusername.github.io/hippo`

### Vercel/Netlify

1. Connect your repository
2. Set build command: (none - static files)
3. Set publish directory: `hippo-web/static`
4. Deploy!

### Custom Domain

1. Configure HTTPS (required for PWA)
2. Update manifest.json `start_url` and `scope`
3. Update Open Graph URLs
4. Test service worker on production domain

### Environment Variables

For production deployment, update URLs:

```javascript
// In index.html or separate config.js
const CONFIG = {
  API_URL: process.env.HIPPO_API_URL || 'http://localhost:3000',
  CDN_URL: process.env.CDN_URL || '',
  ANALYTICS_ID: process.env.ANALYTICS_ID || ''
};
```

## Security

### HTTPS Requirements

- Service workers **require HTTPS** (except localhost)
- Use Let's Encrypt for free SSL certificates
- Configure redirects from HTTP to HTTPS

### Content Security Policy

Add CSP headers to prevent XSS:

```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self';
               script-src 'self' 'unsafe-inline';
               style-src 'self' 'unsafe-inline';
               img-src 'self' data: blob:;
               connect-src 'self' https://api.hippo.com">
```

### Input Sanitization

Always sanitize user input:

```javascript
function sanitizeInput(input) {
  const div = document.createElement('div');
  div.textContent = input;
  return div.innerHTML;
}
```

## Browser Support

| Feature | Chrome | Firefox | Safari | Edge |
|---------|--------|---------|--------|------|
| Service Workers | ✅ | ✅ | ✅ | ✅ |
| Web App Manifest | ✅ | ✅ | ✅ | ✅ |
| Install Prompt | ✅ | ❌ | ❌ | ✅ |
| Background Sync | ✅ | ❌ | ❌ | ✅ |
| Push Notifications | ✅ | ✅ | ❌ | ✅ |
| Share Target | ✅ | ❌ | ❌ | ✅ |
| File Handler | ✅ | ❌ | ❌ | ✅ |

## Troubleshooting

### Service Worker Not Registering

**Problem**: Service worker doesn't register
**Solution**:
- Ensure you're using HTTPS or localhost
- Check browser console for errors
- Verify sw.js is accessible at /sw.js
- Clear browser cache and hard reload

### Install Prompt Not Showing

**Problem**: Install prompt doesn't appear
**Solution**:
- Check manifest.json is valid
- Ensure all icon files exist
- Verify service worker is registered
- Check if already dismissed in localStorage
- Try in incognito mode

### Offline Mode Not Working

**Problem**: App doesn't work offline
**Solution**:
- Verify service worker is installed and activated
- Check DevTools → Application → Service Workers
- Ensure resources are cached (check Cache Storage)
- Increment CACHE_VERSION in sw.js

### Icons Not Displaying

**Problem**: App icons don't show
**Solution**:
- Run `./generate-icons.sh` to create PNG files
- Verify icon files exist in /icons/
- Check manifest.json paths are correct
- Clear browser cache
- Validate manifest with Lighthouse

### Cache Not Updating

**Problem**: Old content still showing after update
**Solution**:
- Increment CACHE_VERSION in sw.js
- Call `registration.update()` programmatically
- Show update banner and reload page
- Use DevTools → Application → Clear storage

## Performance Tips

1. **Minimize Initial Payload**
   - Split code into chunks
   - Lazy load non-critical features
   - Use code splitting

2. **Optimize Images**
   - Use WebP format where possible
   - Implement lazy loading
   - Serve responsive images

3. **Cache Aggressively**
   - Cache all static assets
   - Use appropriate cache strategies
   - Set proper cache expiration times

4. **Preload Critical Resources**
   ```html
   <link rel="preload" href="/critical.css" as="style">
   <link rel="preload" href="/critical.js" as="script">
   ```

5. **Monitor Performance**
   ```javascript
   // Track page load time
   window.addEventListener('load', () => {
     const perfData = performance.getEntriesByType('navigation')[0];
     console.log('Load time:', perfData.loadEventEnd - perfData.fetchStart);
   });
   ```

## Resources

- [PWA Documentation](https://web.dev/progressive-web-apps/)
- [Service Workers](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [Web App Manifest](https://developer.mozilla.org/en-US/docs/Web/Manifest)
- [Workbox (PWA Library)](https://developers.google.com/web/tools/workbox)
- [PWA Builder](https://www.pwabuilder.com/)
- [Can I Use](https://caniuse.com/) - Browser support checker

## License

Same as main Hippo project (see root LICENSE file).
