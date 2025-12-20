# Hippo PWA Quick Reference

One-page reference for common PWA tasks and debugging.

## Quick Commands

```bash
# Generate icons from SVG
cd icons && ./generate-icons.sh

# Verify PWA setup
./verify-pwa.sh

# Start development server
python3 -m http.server 8000

# Test PWA features
# Visit: http://localhost:8000/pwa-test.html
```

## File Structure

```
static/
├── manifest.json          # PWA config
├── sw.js                  # Service worker
├── index.html             # Main app
├── offline.html           # Offline page
├── pwa-utils.js           # Utilities
├── pwa-test.html          # Test page
├── SETUP.md               # Full setup guide
└── icons/
    ├── icon.svg           # Source icon
    └── generate-icons.sh  # Icon generator
```

## Service Worker Cache Control

### Force Cache Update
```javascript
// Increment version in sw.js
const CACHE_VERSION = 'hippo-v1.0.1'; // Changed from v1.0.0

// Clear old caches
caches.keys().then(keys => {
  keys.forEach(key => caches.delete(key));
});

// Or via browser console
await navigator.serviceWorker.getRegistrations()
  .then(regs => regs.forEach(reg => reg.unregister()));
```

### Check Cache Contents
```javascript
// List all caches
const cacheNames = await caches.keys();
console.log('Caches:', cacheNames);

// View cache contents
const cache = await caches.open('hippo-v1.0.0-static');
const keys = await cache.keys();
console.log('Cached URLs:', keys.map(k => k.url));
```

### Manual Cache Operations
```javascript
// Add to cache
const cache = await caches.open('my-cache');
await cache.add('/some-url');

// Remove from cache
await cache.delete('/some-url');

// Clear specific cache
await caches.delete('hippo-v1.0.0-static');
```

## Install Prompt Control

### Trigger Install Prompt
```javascript
// Import utilities
import { setupInstallPrompt } from './pwa-utils.js';

// Setup controller
const installPrompt = setupInstallPrompt();

// Show prompt
installPrompt.on('show', () => {
  console.log('Prompt available');
});

// Trigger installation
await installPrompt.show();
```

### Reset Install Prompt
```javascript
// Clear dismissal
localStorage.removeItem('pwa-install-dismissed');
localStorage.removeItem('pwa-install-dismissed-time');

// Reload page to see prompt again
window.location.reload();
```

## Offline Detection

### Simple Check
```javascript
// Check online status
if (navigator.onLine) {
  console.log('Online');
} else {
  console.log('Offline');
}

// Listen for changes
window.addEventListener('online', () => {
  console.log('Back online');
});

window.addEventListener('offline', () => {
  console.log('Gone offline');
});
```

### With PWA Utils
```javascript
import { setupOfflineDetection } from './pwa-utils.js';

const offline = setupOfflineDetection(
  () => console.log('Online'),
  () => console.log('Offline')
);

// Check current status
console.log(offline.isOnline());

// Cleanup
offline.cleanup();
```

## Notifications

### Request Permission
```javascript
const permission = await Notification.requestPermission();
console.log('Permission:', permission); // 'granted', 'denied', or 'default'
```

### Show Notification
```javascript
import { showNotification } from './pwa-utils.js';

await showNotification('Title', {
  body: 'Notification body text',
  icon: '/icons/icon-192.png',
  badge: '/icons/icon-192.png',
  tag: 'unique-tag',
  vibrate: [200, 100, 200],
  actions: [
    { action: 'open', title: 'Open' },
    { action: 'close', title: 'Close' }
  ]
});
```

## Share API

### Basic Share
```javascript
import { shareContent, canShare } from './pwa-utils.js';

if (canShare()) {
  await shareContent({
    title: 'Title',
    text: 'Text to share',
    url: window.location.href
  });
}
```

### Share Files
```javascript
const file = new File(['content'], 'file.txt', { type: 'text/plain' });

if (navigator.canShare && navigator.canShare({ files: [file] })) {
  await navigator.share({
    title: 'Sharing a file',
    files: [file]
  });
}
```

## Performance Monitoring

### Get Metrics
```javascript
import { getPerformanceMetrics, logPerformanceMetrics } from './pwa-utils.js';

// Get metrics object
const metrics = getPerformanceMetrics();
console.log('Load time:', metrics.loadTime);

// Or log all metrics
logPerformanceMetrics();
```

### Network Information
```javascript
import { getNetworkInfo, isSlowNetwork } from './pwa-utils.js';

const info = getNetworkInfo();
console.log('Connection type:', info.effectiveType); // '4g', '3g', etc.
console.log('Downlink:', info.downlink, 'Mbps');
console.log('RTT:', info.rtt, 'ms');

if (isSlowNetwork()) {
  console.log('Slow network detected');
  // Load lower quality images, etc.
}
```

## Debugging Commands

### Chrome DevTools Console

```javascript
// Check if running as PWA
console.log('Is PWA:', window.matchMedia('(display-mode: standalone)').matches);

// Get service worker registration
const reg = await navigator.serviceWorker.getRegistration();
console.log('SW state:', reg?.active?.state);

// Force service worker update
await reg?.update();

// List all registrations
const regs = await navigator.serviceWorker.getRegistrations();
console.log('Registrations:', regs.length);

// Unregister all
regs.forEach(r => r.unregister());

// Clear all storage
localStorage.clear();
sessionStorage.clear();
await caches.keys().then(keys => keys.forEach(k => caches.delete(k)));
if ('indexedDB' in window) {
  // Note: Must enumerate databases manually
}
```

### Service Worker Console

```javascript
// Send message to SW
navigator.serviceWorker.controller.postMessage({
  type: 'CLEAR_CACHE'
});

// Send message with response
const response = await new Promise((resolve) => {
  const channel = new MessageChannel();
  channel.port1.onmessage = (event) => resolve(event.data);
  navigator.serviceWorker.controller.postMessage(
    { type: 'GET_STATUS' },
    [channel.port2]
  );
});
```

## Common Issues & Fixes

### Service Worker Not Updating

**Problem:** Changes to SW not taking effect
**Fix:**
1. Increment `CACHE_VERSION` in sw.js
2. Hard refresh: Ctrl+Shift+R (Windows) / Cmd+Shift+R (Mac)
3. DevTools → Application → Service Workers → Update
4. Check "Update on reload" checkbox

### Install Prompt Not Showing

**Problem:** No install banner appears
**Fix:**
1. Clear: `localStorage.removeItem('pwa-install-dismissed')`
2. Test in Incognito mode
3. Verify manifest.json is valid
4. Check all icons exist
5. Must be on HTTPS (or localhost)

### Offline Mode Not Working

**Problem:** App doesn't work offline
**Fix:**
1. Verify SW is registered and activated
2. Check DevTools → Application → Cache Storage
3. Ensure index.html is cached
4. Test: DevTools → Network → Offline checkbox

### Cache Not Clearing

**Problem:** Old content still showing
**Fix:**
```javascript
// Nuclear option: clear everything
await caches.keys().then(keys =>
  Promise.all(keys.map(k => caches.delete(k)))
);
await navigator.serviceWorker.getRegistrations()
  .then(regs => Promise.all(regs.map(r => r.unregister())));
localStorage.clear();
window.location.reload(true);
```

## Manifest.json Fields

```json
{
  "name": "Full App Name",           // Required
  "short_name": "App",                // Required
  "start_url": "/",                   // Required
  "display": "standalone",            // Required
  "background_color": "#ffffff",      // Required
  "theme_color": "#6366f1",          // Required
  "description": "App description",   // Recommended
  "icons": [                          // Required (192 & 512)
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    }
  ],
  "categories": ["productivity"],     // Optional
  "shortcuts": [],                    // Optional
  "share_target": {},                 // Optional
  "file_handlers": []                 // Optional
}
```

## Service Worker Events

### Install
```javascript
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open('my-cache')
      .then(cache => cache.addAll(['/index.html']))
      .then(() => self.skipWaiting())
  );
});
```

### Activate
```javascript
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then(keys => Promise.all(
        keys.filter(k => k !== 'my-cache')
          .map(k => caches.delete(k))
      ))
      .then(() => self.clients.claim())
  );
});
```

### Fetch
```javascript
self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request)
      .then(response => response || fetch(event.request))
  );
});
```

## Browser Support Check

```javascript
// Service Workers
const hasSW = 'serviceWorker' in navigator;

// Push Notifications
const hasPush = 'Notification' in window;

// Background Sync
const hasSync = 'sync' in ServiceWorkerRegistration.prototype;

// Web Share
const hasShare = 'share' in navigator;

// Cache API
const hasCache = 'caches' in window;

// IndexedDB
const hasIDB = 'indexedDB' in window;

// Summary
console.log({
  serviceWorker: hasSW,
  notifications: hasPush,
  backgroundSync: hasSync,
  share: hasShare,
  cache: hasCache,
  indexedDB: hasIDB
});
```

## Testing URLs

- **Main App**: http://localhost:8000/
- **Test Page**: http://localhost:8000/pwa-test.html
- **Offline Page**: http://localhost:8000/offline.html
- **Manifest**: http://localhost:8000/manifest.json
- **Service Worker**: http://localhost:8000/sw.js

## DevTools Shortcuts

- **Application Tab**: Chrome DevTools → Application
- **Service Workers**: Application → Service Workers
- **Cache Storage**: Application → Cache Storage
- **Manifest**: Application → Manifest
- **Lighthouse**: DevTools → Lighthouse
- **Network Offline**: Network → Offline checkbox

## Lighthouse PWA Checklist

Essential for PWA badge:
- ✅ Served over HTTPS
- ✅ Registers a service worker
- ✅ Responds with 200 when offline
- ✅ Has a web app manifest
- ✅ Contains valid icons (192x192 & 512x512)
- ✅ Sets viewport meta tag
- ✅ Themed address bar
- ✅ Content sized correctly

## Resources

- **Full Setup**: See SETUP.md
- **Documentation**: See PWA_README.md
- **Icons Guide**: See icons/README.md
- **Test Page**: Open pwa-test.html in browser

## Version Info

- PWA Utils: ES6 Modules
- Service Worker: Version controlled via CACHE_VERSION
- Browser Requirements: Modern browsers (Chrome 80+, Safari 14+, Firefox 90+)
