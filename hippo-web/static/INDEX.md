# Hippo PWA - File Index

Complete index of all Progressive Web App files and their purposes.

## Quick Navigation

- [Getting Started](#getting-started)
- [Core Files](#core-files)
- [Documentation](#documentation)
- [Tools & Testing](#tools--testing)
- [Icons](#icons)

---

## Getting Started

**Start here if you're new:**

1. Read: [`SETUP.md`](./SETUP.md) - Complete setup guide (5 minutes)
2. Run: `./verify-pwa.sh` - Verify your setup
3. Generate: `cd icons && ./generate-icons.sh` - Create icon files
4. Test: Open `pwa-test.html` in browser

**Quick reference:**
- [`PWA_QUICKREF.md`](./PWA_QUICKREF.md) - One-page cheat sheet

---

## Core Files

### Essential PWA Files

| File | Lines | Purpose | Documentation |
|------|-------|---------|---------------|
| **manifest.json** | 96 | PWA configuration, icons, theme colors | [Spec](https://developer.mozilla.org/en-US/docs/Web/Manifest) |
| **sw.js** | 550+ | Service worker with caching strategies | [PWA_README.md](./PWA_README.md#service-worker-caching) |
| **index.html** | 450+ | Main PWA page with all features | Inline comments |
| **offline.html** | 150+ | Offline fallback page | [SETUP.md](./SETUP.md#offline-features) |
| **browserconfig.xml** | 10 | Microsoft Edge/Windows tile config | [Microsoft Docs](https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/dn320426(v=vs.85)) |
| **pwa-utils.js** | 800+ | Reusable PWA utility functions | Inline JSDoc |

### Purpose of Each File

#### manifest.json
Web app manifest that defines how the PWA appears when installed:
- App name, description, icons
- Theme colors and background color
- Display mode (standalone)
- Share target for receiving files
- File handlers for opening files
- Shortcuts for quick actions

**Used by:** Browser install prompt, home screen installation, OS integration

#### sw.js
Service worker that enables offline functionality:
- Caches static assets on install
- Intercepts network requests
- Implements caching strategies (network-first, cache-first, stale-while-revalidate)
- Manages cache expiration and trimming
- Handles background sync (stub)
- Processes push notifications (stub)

**Strategies implemented:**
- Static assets: Cache-first
- Images: Cache-first with 7-day expiration
- API calls: Network-first with offline fallback
- Dynamic content: Stale-while-revalidate

#### index.html
Main application entry point with PWA features:
- Complete PWA meta tags (Apple, Android, Microsoft)
- Service worker registration and update handling
- Install prompt with smart timing (shows after 5 seconds)
- Offline/online detection with banner
- Update notification banner
- Dark mode support based on system preference
- Responsive mobile-first design
- Feature showcase grid

**PWA Features:**
- Auto-registers service worker
- Shows install prompt
- Detects online/offline status
- Notifies of updates
- Handles installation tracking

#### offline.html
Beautiful fallback page shown when offline:
- Gradient purple background
- Connection status indicator with real-time updates
- List of features available offline
- Auto-retry when connection restored
- Auto-reload when back online

**Shown when:**
- User navigates to uncached page while offline
- Service worker has no cached response

#### browserconfig.xml
Configuration for Microsoft Edge and Windows:
- Tile colors for Windows Start menu
- Icon paths for Windows tiles
- Branded experience on Windows

#### pwa-utils.js
ES6 module with reusable PWA functions:

**Service Worker:**
- `registerServiceWorker()` - Register and handle updates
- `updateServiceWorker()` - Force update and reload
- `clearAllCaches()` - Clear all caches
- `sendMessageToSW()` - Send messages to service worker

**Install Prompt:**
- `setupInstallPrompt()` - Handle beforeinstallprompt
- Methods: show(), dismiss(), isAvailable()
- Events: 'show', 'install', 'dismiss'

**Offline Detection:**
- `setupOfflineDetection()` - Monitor connection
- `createOfflineBanner()` - Create UI banner

**Background Sync:**
- `queueBackgroundSync()` - Queue operations
- `getSyncQueue()` - Get pending operations
- `clearSyncQueue()` - Clear queue

**Notifications:**
- `requestNotificationPermission()` - Request permission
- `showNotification()` - Show notification

**Share API:**
- `canShare()` - Check if supported
- `shareContent()` - Share content/files

**Performance:**
- `getPerformanceMetrics()` - Get load metrics
- `logPerformanceMetrics()` - Log to console

**Utilities:**
- `isInstalledPWA()` - Check if running as PWA
- `isTauriApp()` - Check if running in Tauri
- `detectPlatform()` - Detect iOS/Android/Desktop
- `getNetworkInfo()` - Get connection info
- `isSlowNetwork()` - Check if slow connection

**Initialization:**
- `initializePWA()` - Setup all PWA features

---

## Documentation

### Complete Guides

| File | Size | Audience | Content |
|------|------|----------|---------|
| **SETUP.md** | 400+ lines | Beginners | Step-by-step setup, testing, deployment |
| **PWA_README.md** | 550+ lines | Developers | Comprehensive technical documentation |
| **PWA_QUICKREF.md** | 350+ lines | Everyone | Quick reference for common tasks |
| **PWA_IMPLEMENTATION_SUMMARY.md** | 500+ lines | Project leads | Complete implementation overview |
| **INDEX.md** | This file | Everyone | Navigation and file reference |

### What to Read When

**First time setup:**
1. INDEX.md (this file) - Understand structure
2. SETUP.md - Follow setup steps
3. PWA_QUICKREF.md - Bookmark for reference

**Development:**
- PWA_QUICKREF.md - Code snippets and examples
- PWA_README.md - Deep dives on features
- Inline code comments - Implementation details

**Debugging:**
- PWA_QUICKREF.md - Common issues section
- SETUP.md - Troubleshooting guide
- pwa-test.html - Feature testing

**Deployment:**
- SETUP.md - Deployment section
- PWA_README.md - Production considerations
- PWA_IMPLEMENTATION_SUMMARY.md - Checklist

---

## Tools & Testing

### Development Tools

| File | Purpose | How to Use |
|------|---------|------------|
| **pwa-test.html** | Feature testing & debugging | Open in browser, click test buttons |
| **verify-pwa.sh** | Setup verification | Run: `./verify-pwa.sh` |

### pwa-test.html Features

Interactive testing page with:

**Environment Info:**
- Platform detection (iOS/Android/Desktop)
- Install status (PWA vs Browser)
- Connection type (Online/Offline/4G/3G)
- Network information

**Feature Detection:**
- Service Workers support
- Push Notifications support
- Background Sync support
- Web Share API support
- Cache API support
- IndexedDB support
- Fetch API support
- Web App Manifest presence
- HTTPS/localhost check

**Service Worker Tests:**
- Register service worker
- Check cache contents
- Clear all caches
- View cache statistics

**Install Prompt Tests:**
- Show install prompt
- Dismiss install prompt
- Check if prompt available

**Offline Tests:**
- Test offline mode
- Queue offline operations
- Monitor connection status

**Notification Tests:**
- Request permission
- Show test notification
- View permission status

**Share API Tests:**
- Check if supported
- Test sharing content

**Performance Tests:**
- Display load metrics
- Show TTFB, DOM load, render time
- Network timing breakdown

**Console Log:**
- Real-time log viewer
- Color-coded messages
- Timestamp for each entry

### verify-pwa.sh Features

Automated verification script that checks:

**Files:**
- All required PWA files exist
- Documentation files present
- Icon source files exist

**Configuration:**
- manifest.json is valid JSON
- Service worker has required event handlers
- Icons script is executable

**Tools:**
- Icon generation tools installed (librsvg/ImageMagick/Inkscape)
- Optional tools detected

**Output:**
- Color-coded results (green/yellow/red)
- Error count and warning count
- Helpful suggestions for fixes

---

## Icons

### Icon Files

| File | Type | Purpose |
|------|------|---------|
| **icon.svg** | Source | Editable vector icon |
| **generate-icons.sh** | Script | Generates all PNG icons |
| **README.md** | Docs | Icon documentation |

### Generated Icons (after running script)

| File | Size | Purpose | Used By |
|------|------|---------|---------|
| icon-192.png | 192x192 | PWA manifest icon | Android, Chrome |
| icon-512.png | 512x512 | PWA splash screen | Android, Chrome |
| apple-touch-icon.png | 180x180 | iOS home screen | iOS Safari |
| favicon.ico | Multi-size | Browser tab | All browsers |
| favicon-16.png | 16x16 | Small favicon | Browser tabs |
| favicon-32.png | 32x32 | Standard favicon | Browser tabs |

### Icon Generation

**Requirements:**
One of these tools:
- librsvg (rsvg-convert) - Recommended
- ImageMagick (convert)
- Inkscape

**Installation:**
```bash
# macOS
brew install librsvg

# Linux (Ubuntu/Debian)
sudo apt install librsvg2-bin

# Linux (Fedora)
sudo dnf install librsvg2-tools
```

**Usage:**
```bash
cd icons
./generate-icons.sh
```

**Customization:**
1. Edit `icon.svg` with your design
2. Run `./generate-icons.sh`
3. Verify generated PNGs
4. Update theme colors in manifest.json

---

## File Dependencies

### What Depends on What

```
index.html
├── manifest.json (linked via <link rel="manifest">)
├── sw.js (registered via JavaScript)
├── pwa-utils.js (imported as ES6 module)
└── icons/* (referenced in manifest)

sw.js
├── manifest.json (caches as static asset)
├── index.html (caches as static asset)
└── offline.html (serves when offline)

manifest.json
└── icons/* (icon paths)

pwa-test.html
└── pwa-utils.js (imports functions)

verify-pwa.sh
└── All files (checks existence)
```

### Load Order

1. Browser requests `index.html`
2. HTML loads and parses `manifest.json`
3. HTML registers service worker (`sw.js`)
4. Service worker caches static assets
5. HTML imports `pwa-utils.js` (if used)
6. PWA features initialize

---

## Size Overview

**Estimated file sizes:**
- manifest.json: ~4 KB
- sw.js: ~20 KB
- index.html: ~15 KB
- offline.html: ~5 KB
- browserconfig.xml: <1 KB
- pwa-utils.js: ~30 KB
- pwa-test.html: ~15 KB

**Generated icons:**
- icon-192.png: ~8-15 KB
- icon-512.png: ~20-40 KB
- favicon.ico: ~5-10 KB

**Total PWA bundle:** ~90-120 KB (excluding documentation)
**With icons:** ~120-180 KB
**Documentation:** ~150 KB (not loaded by app)

---

## Browser Caching

Files are cached by service worker in different caches:

**Static Cache (permanent):**
- /
- /index.html
- /manifest.json
- /icons/icon-192.png
- /icons/icon-512.png

**Dynamic Cache (up to 50 items):**
- User-visited pages
- 24-hour expiration

**Image Cache (up to 100 items):**
- Thumbnails, photos
- 7-day expiration

**API Cache (up to 30 items):**
- API responses
- 5-minute expiration

---

## Integration Examples

### With Hippo REST API

```javascript
// In your app code
import { isInstalledPWA, setupOfflineDetection } from './pwa-utils.js';

const API_BASE = 'http://localhost:3000/api';

async function searchWithOfflineSupport(query) {
  try {
    const response = await fetch(`${API_BASE}/search?q=${query}`);
    return await response.json();
  } catch (error) {
    // Check cache
    const cached = await caches.match(`${API_BASE}/search?q=${query}`);
    if (cached) {
      return await cached.json();
    }
    throw new Error('Offline and no cached results');
  }
}
```

### With Tauri Desktop App

```javascript
// Conditional service worker registration
if (!window.__TAURI__ && 'serviceWorker' in navigator) {
  // Only register in browser, not Tauri
  await registerServiceWorker('/sw.js');
}

// Platform-specific API calls
import { isTauriApp } from './pwa-utils.js';

if (isTauriApp()) {
  // Use Tauri commands
  const results = await window.__TAURI__.invoke('search', { query });
} else {
  // Use web APIs
  const results = await searchWithOfflineSupport(query);
}
```

---

## Maintenance

### Regular Tasks

**Weekly:**
- Check for service worker updates
- Monitor cache sizes
- Review error logs

**Monthly:**
- Update dependencies
- Review Lighthouse scores
- Test on new browser versions

**As Needed:**
- Update icons when branding changes
- Increment CACHE_VERSION when assets change
- Update manifest.json for new features

### Version Management

**Service Worker Versions:**
```javascript
// In sw.js
const CACHE_VERSION = 'hippo-v1.0.0';

// Increment when:
// - Static assets change
// - Caching strategy changes
// - Bug fixes in service worker
```

**Manifest Updates:**
- Update `manifest.json` version in comments
- Test install prompt after changes
- Verify icons still valid

---

## Support & Resources

### Getting Help

1. **Check Documentation:**
   - SETUP.md for setup issues
   - PWA_QUICKREF.md for code examples
   - PWA_README.md for technical details

2. **Use Test Tools:**
   - Run `./verify-pwa.sh`
   - Open `pwa-test.html` in browser
   - Check browser console

3. **Debug Tools:**
   - Chrome DevTools → Application tab
   - Service Workers panel
   - Cache Storage panel
   - Manifest panel

4. **Run Lighthouse:**
   - Chrome DevTools → Lighthouse tab
   - Check PWA audit results
   - Follow recommendations

### External Resources

- [MDN: Progressive Web Apps](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)
- [web.dev: PWA](https://web.dev/progressive-web-apps/)
- [Chrome DevTools](https://developer.chrome.com/docs/devtools/)
- [Lighthouse](https://developers.google.com/web/tools/lighthouse)
- [PWA Builder](https://www.pwabuilder.com/)
- [Can I Use](https://caniuse.com/) - Browser support

---

## License

All PWA files are part of the Hippo project.
See root LICENSE file for details.

---

## Quick Links

- **Setup**: [SETUP.md](./SETUP.md)
- **Docs**: [PWA_README.md](./PWA_README.md)
- **Reference**: [PWA_QUICKREF.md](./PWA_QUICKREF.md)
- **Summary**: [PWA_IMPLEMENTATION_SUMMARY.md](./PWA_IMPLEMENTATION_SUMMARY.md)
- **Test**: [pwa-test.html](./pwa-test.html)
- **Icons**: [icons/README.md](./icons/README.md)

---

*Last Updated: 2024-12-20*
*PWA Version: 1.0.0*
