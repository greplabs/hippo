# Hippo PWA Setup Guide

Complete setup instructions for the Progressive Web App implementation.

## Quick Start (5 Minutes)

### 1. Generate Icons

```bash
cd hippo-web/static/icons
chmod +x generate-icons.sh
./generate-icons.sh
```

This requires one of:
- **librsvg**: `brew install librsvg` (macOS) or `sudo apt install librsvg2-bin` (Linux)
- **ImageMagick**: `brew install imagemagick` or `sudo apt install imagemagick`
- **Inkscape**: Download from https://inkscape.org/

### 2. Start the Server

**Option A: Use the Hippo Web API server**
```bash
cd hippo-web
cargo run
# Server runs on http://localhost:3000
# Visit http://localhost:3000
```

**Option B: Use a simple static server**
```bash
cd hippo-web/static
python3 -m http.server 8000
# Visit http://localhost:8000
```

### 3. Test PWA Features

Open Chrome and visit: `http://localhost:8000/pwa-test.html`

This test page will verify:
- Service worker registration
- Cache functionality
- Install prompt
- Offline detection
- Notifications
- Share API
- Performance metrics

### 4. Install the PWA

**Desktop:**
1. Click the install icon (⊕) in the address bar
2. Or use Chrome menu → "Install Hippo..."

**Mobile:**
1. Visit the site in Chrome/Safari
2. Tap "Add to Home Screen" prompt
3. Or use browser menu → "Install app"

## Files Created

### Core PWA Files

```
hippo-web/static/
├── manifest.json           # PWA configuration & metadata
├── sw.js                   # Service worker (caching logic)
├── index.html              # Main PWA page with features
├── offline.html            # Offline fallback page
├── browserconfig.xml       # Microsoft tiles config
├── pwa-utils.js            # Reusable PWA utilities
├── pwa-test.html           # Testing/debugging page
├── PWA_README.md           # Detailed documentation
├── SETUP.md                # This file
└── icons/
    ├── icon.svg            # Source icon (customizable)
    ├── generate-icons.sh   # Icon generation script
    └── README.md           # Icon documentation
```

### Icons to Generate

After running `generate-icons.sh`, you'll have:
- `icon-192.png` - 192x192 PWA icon
- `icon-512.png` - 512x512 PWA splash screen
- `apple-touch-icon.png` - 180x180 iOS home screen
- `favicon.ico` - Multi-size browser favicon
- `favicon-16.png` - 16x16 favicon
- `favicon-32.png` - 32x32 favicon

## PWA Features Implemented

### ✅ Core Features
- **Installable** - Can be installed on any device
- **Offline Support** - Works without internet via service worker
- **App-like UI** - Standalone mode hides browser chrome
- **Fast Loading** - Aggressive caching for instant loads

### ✅ Caching Strategies
- **Static Assets** - Cache-first for CSS/JS/fonts
- **Images** - Cache-first with 7-day expiration
- **API Calls** - Network-first with offline fallback
- **Dynamic Content** - Stale-while-revalidate

### ✅ Mobile Features
- **Install Prompt** - Smart banner for installation
- **Offline Banner** - Shows connection status
- **Update Banner** - Notifies when updates available
- **Share Target** - Receive files from other apps (configured)
- **File Handler** - Open files with Hippo (configured)

### ✅ Performance
- **Cache Management** - Automatic cache trimming
- **Background Sync** - Queue operations for later (stub)
- **Push Notifications** - Ready for notifications (stub)
- **Performance Monitoring** - Built-in metrics tracking

## Testing Checklist

### Service Worker
- [ ] Service worker registers successfully
- [ ] Static assets cached on install
- [ ] API responses cached correctly
- [ ] Offline mode works (Network tab → Offline)
- [ ] Cache updates when CACHE_VERSION changes
- [ ] Old caches cleaned up on activate

### Install Prompt
- [ ] Install prompt appears (after 5 seconds)
- [ ] Install button works
- [ ] Dismiss button works (hides for 7 days)
- [ ] App can be installed on home screen
- [ ] Installed app opens in standalone mode
- [ ] App icon shows correctly

### Offline Features
- [ ] Offline banner appears when disconnected
- [ ] Online banner shows when reconnected
- [ ] Cached content accessible offline
- [ ] Offline page shown for uncached routes
- [ ] Background sync registers (if supported)

### Notifications
- [ ] Permission can be requested
- [ ] Test notification shows correctly
- [ ] Notification icon/badge displays
- [ ] Notification click handlers work

### Share API
- [ ] Share button appears (if supported)
- [ ] Share dialog opens correctly
- [ ] Content shares successfully

### Performance
- [ ] Page loads in < 2 seconds
- [ ] Lighthouse PWA score = 100
- [ ] All assets cached properly
- [ ] No console errors

## Lighthouse Audit

Run a Lighthouse audit to verify PWA quality:

1. Open Chrome DevTools (F12)
2. Go to "Lighthouse" tab
3. Select:
   - ✅ Progressive Web App
   - ✅ Performance
   - ✅ Accessibility
   - ✅ Best Practices
   - ✅ SEO
4. Click "Generate report"

**Target Scores:**
- PWA: **100** (Required)
- Performance: **90+**
- Accessibility: **90+**
- Best Practices: **90+**
- SEO: **90+**

### Common Lighthouse Issues

**Issue: "Does not register a service worker"**
- Solution: Ensure sw.js is accessible at `/sw.js`
- Check: Service worker registration code runs

**Issue: "Does not respond with 200 when offline"**
- Solution: Verify service worker caches index.html
- Test: Enable offline mode and reload

**Issue: "Web app manifest does not meet installability requirements"**
- Solution: Check manifest.json has all required fields
- Verify: Icons exist at specified paths

**Issue: "Is not configured for a custom splash screen"**
- Solution: Add 512x512 icon to manifest
- Ensure: background_color and theme_color set

## Customization

### Update Branding

**1. Edit manifest.json:**
```json
{
  "name": "Your App Name",
  "short_name": "YourApp",
  "description": "Your description",
  "theme_color": "#yourcolor",
  "background_color": "#yourcolor"
}
```

**2. Create custom icons:**
```bash
# Edit icons/icon.svg with your design
cd icons
# Then regenerate PNGs
./generate-icons.sh
```

**3. Update meta tags in HTML:**
```html
<meta name="theme-color" content="#yourcolor">
<title>Your App Name</title>
<meta name="description" content="Your description">
```

### Modify Caching

**Edit sw.js to customize caching:**

```javascript
// Change version to force cache update
const CACHE_VERSION = 'hippo-v2.0.0';

// Adjust maximum cache sizes
const MAX_CACHE_SIZE = {
  'hippo-v2.0.0-dynamic': 100,  // Increase from 50
  'hippo-v2.0.0-images': 200,   // Increase from 100
};

// Change expiration times
const CACHE_EXPIRATION = {
  'hippo-v2.0.0-api': 10 * 60 * 1000,  // 10 minutes
  'hippo-v2.0.0-images': 30 * 24 * 60 * 60 * 1000,  // 30 days
};

// Add custom routes
if (url.pathname.startsWith('/custom/')) {
  event.respondWith(yourCustomStrategy(request));
}
```

### Add Analytics

```javascript
// In index.html or pwa-utils.js
window.addEventListener('appinstalled', () => {
  // Track installation
  gtag('event', 'pwa_install', {
    event_category: 'engagement',
    event_label: 'PWA Installed'
  });
});
```

## Integration with Hippo Tauri

To use PWA features in the Tauri desktop app:

### 1. Copy PWA Files

```bash
cp -r hippo-web/static/* hippo-tauri/ui/dist/
```

### 2. Conditional Service Worker

Edit index.html to skip SW registration in Tauri:

```javascript
if (!window.__TAURI__ && 'serviceWorker' in navigator) {
  // Only register in browser, not Tauri
  navigator.serviceWorker.register('/sw.js');
}
```

### 3. Use PWA Utils

```javascript
import { isInstalledPWA, isTauriApp } from './pwa-utils.js';

if (isTauriApp()) {
  // Use Tauri APIs
  const { invoke } = window.__TAURI__;
  await invoke('search', { query });
} else {
  // Use web APIs
  await fetch('/api/search?q=' + query);
}
```

## Deployment

### GitHub Pages

1. Push to GitHub
2. Settings → Pages → Enable
3. Select branch and `/hippo-web/static` folder
4. Access at `https://yourusername.github.io/hippo`

### Vercel

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
cd hippo-web/static
vercel
```

### Netlify

1. Connect GitHub repository
2. Build command: (none)
3. Publish directory: `hippo-web/static`
4. Deploy!

### Custom Domain

**Requirements:**
- HTTPS (required for service workers)
- Valid SSL certificate

**Steps:**
1. Configure DNS to point to your server
2. Setup SSL (Let's Encrypt recommended)
3. Update manifest.json URLs
4. Test PWA functionality

**nginx config example:**
```nginx
server {
    listen 443 ssl http2;
    server_name hippo.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    root /var/www/hippo-web/static;
    index index.html;

    # Service worker must be served from root
    location = /sw.js {
        add_header Service-Worker-Allowed /;
        add_header Cache-Control "no-cache, no-store, must-revalidate";
    }

    # Manifest
    location = /manifest.json {
        add_header Cache-Control "public, max-age=3600";
    }

    # Static assets
    location ~* \.(css|js|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf)$ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    # API proxy (optional)
    location /api/ {
        proxy_pass http://localhost:3000;
    }
}
```

## Troubleshooting

### Service Worker Not Working

**Symptoms:** SW doesn't register, offline mode fails
**Solutions:**
1. Must use HTTPS (or localhost for testing)
2. Check browser console for errors
3. Verify sw.js path is correct (must be `/sw.js`)
4. Clear browser cache (Ctrl+Shift+Delete)
5. Unregister old service workers:
   ```javascript
   navigator.serviceWorker.getRegistrations().then(regs => {
     regs.forEach(reg => reg.unregister());
   });
   ```

### Install Prompt Not Showing

**Symptoms:** No install prompt appears
**Solutions:**
1. Check manifest.json is valid (use PWA Builder validator)
2. Verify all icon files exist
3. Must be served over HTTPS
4. Clear localStorage: `localStorage.removeItem('pwa-install-dismissed')`
5. Test in Incognito mode
6. Some browsers don't support install prompts (Firefox, Safari)

### Icons Not Displaying

**Symptoms:** Broken icon images
**Solutions:**
1. Run `./generate-icons.sh` to create PNGs
2. Check file permissions
3. Verify paths in manifest.json match actual files
4. Clear browser cache
5. Check browser console for 404 errors

### Cache Not Updating

**Symptoms:** Old content shows after update
**Solutions:**
1. Increment `CACHE_VERSION` in sw.js
2. Force update: DevTools → Application → Service Workers → Update
3. Clear all caches: DevTools → Application → Storage → Clear
4. Show update banner and prompt user to reload
5. Add `skipWaiting()` in SW install event

### Offline Mode Not Working

**Symptoms:** App doesn't work offline
**Solutions:**
1. Verify service worker is activated
2. Check cached resources: DevTools → Application → Cache Storage
3. Test with DevTools → Network → Offline checkbox
4. Ensure index.html is in static cache
5. Check fetch event handler in sw.js

## Next Steps

### Recommended Enhancements

1. **Implement Background Sync**
   - Store operations in IndexedDB when offline
   - Sync when connection restored
   - Show sync status to user

2. **Add Push Notifications**
   - Request notification permission
   - Subscribe to push service
   - Handle push events in service worker
   - Show relevant notifications

3. **Optimize Performance**
   - Implement code splitting
   - Lazy load non-critical features
   - Use WebP images
   - Enable brotli compression

4. **Enhance Offline Experience**
   - Cache more content proactively
   - Show which features work offline
   - Better offline UI/UX
   - Offline search with IndexedDB

5. **Add Analytics**
   - Track PWA installations
   - Monitor offline usage
   - Measure performance metrics
   - A/B test install prompts

6. **Improve Accessibility**
   - ARIA labels
   - Keyboard navigation
   - Screen reader support
   - High contrast mode

## Resources

- **Documentation**: `PWA_README.md` - Detailed PWA documentation
- **Icons Guide**: `icons/README.md` - Icon generation and customization
- **Test Page**: `pwa-test.html` - Feature testing and debugging
- **Utilities**: `pwa-utils.js` - Reusable PWA functions

## Support

For issues or questions:
1. Check browser console for errors
2. Run Lighthouse audit
3. Test with `pwa-test.html`
4. Verify in multiple browsers
5. Check service worker registration status

## License

Same as main Hippo project (see root LICENSE file).
