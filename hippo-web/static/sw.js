/**
 * Hippo Service Worker
 * Provides offline support, caching, and background sync
 */

const CACHE_VERSION = 'hippo-v1.0.0';
const STATIC_CACHE = `${CACHE_VERSION}-static`;
const DYNAMIC_CACHE = `${CACHE_VERSION}-dynamic`;
const IMAGE_CACHE = `${CACHE_VERSION}-images`;
const API_CACHE = `${CACHE_VERSION}-api`;

// Assets to cache immediately on install
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/manifest.json',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/icons/icon.svg',
  // Add any CSS, JS, or font files here
];

// Maximum cache sizes
const MAX_CACHE_SIZE = {
  [DYNAMIC_CACHE]: 50,
  [IMAGE_CACHE]: 100,
  [API_CACHE]: 30
};

// Cache expiration times (in milliseconds)
const CACHE_EXPIRATION = {
  [API_CACHE]: 5 * 60 * 1000, // 5 minutes for API responses
  [IMAGE_CACHE]: 7 * 24 * 60 * 60 * 1000, // 7 days for images
  [DYNAMIC_CACHE]: 24 * 60 * 60 * 1000 // 24 hours for dynamic content
};

/**
 * Install event - cache static assets
 */
self.addEventListener('install', (event) => {
  console.log('[SW] Installing service worker...');

  event.waitUntil(
    caches.open(STATIC_CACHE)
      .then((cache) => {
        console.log('[SW] Caching static assets');
        return cache.addAll(STATIC_ASSETS);
      })
      .then(() => {
        console.log('[SW] Skip waiting');
        return self.skipWaiting();
      })
      .catch((error) => {
        console.error('[SW] Installation failed:', error);
      })
  );
});

/**
 * Activate event - clean up old caches
 */
self.addEventListener('activate', (event) => {
  console.log('[SW] Activating service worker...');

  event.waitUntil(
    caches.keys()
      .then((cacheNames) => {
        return Promise.all(
          cacheNames
            .filter((cacheName) => {
              // Delete caches that don't match current version
              return cacheName.startsWith('hippo-') &&
                     !cacheName.startsWith(CACHE_VERSION);
            })
            .map((cacheName) => {
              console.log('[SW] Deleting old cache:', cacheName);
              return caches.delete(cacheName);
            })
        );
      })
      .then(() => {
        console.log('[SW] Claiming clients');
        return self.clients.claim();
      })
  );
});

/**
 * Fetch event - implement caching strategies
 */
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests
  if (request.method !== 'GET') {
    return;
  }

  // Skip chrome-extension and other non-http(s) requests
  if (!url.protocol.startsWith('http')) {
    return;
  }

  // Route to appropriate caching strategy
  if (isApiRequest(url)) {
    event.respondWith(networkFirstStrategy(request, API_CACHE));
  } else if (isImageRequest(url)) {
    event.respondWith(cacheFirstStrategy(request, IMAGE_CACHE));
  } else if (isStaticAsset(url)) {
    event.respondWith(cacheFirstStrategy(request, STATIC_CACHE));
  } else {
    event.respondWith(staleWhileRevalidateStrategy(request, DYNAMIC_CACHE));
  }
});

/**
 * Network-first strategy (for API calls)
 * Try network first, fall back to cache if offline
 */
async function networkFirstStrategy(request, cacheName) {
  try {
    const networkResponse = await fetch(request);

    // Only cache successful responses
    if (networkResponse.ok) {
      const cache = await caches.open(cacheName);
      // Clone the response before caching
      cache.put(request, networkResponse.clone());

      // Trim cache if needed
      trimCache(cacheName, MAX_CACHE_SIZE[cacheName]);
    }

    return networkResponse;
  } catch (error) {
    console.log('[SW] Network failed, trying cache:', request.url);

    const cachedResponse = await caches.match(request);
    if (cachedResponse) {
      return cachedResponse;
    }

    // Return offline page or error response
    return new Response(JSON.stringify({
      error: 'Offline',
      message: 'You are currently offline. Please check your connection.'
    }), {
      status: 503,
      statusText: 'Service Unavailable',
      headers: { 'Content-Type': 'application/json' }
    });
  }
}

/**
 * Cache-first strategy (for images and static assets)
 * Try cache first, fall back to network
 */
async function cacheFirstStrategy(request, cacheName) {
  const cachedResponse = await caches.match(request);

  if (cachedResponse) {
    // Check if cache is expired
    const cacheDate = new Date(cachedResponse.headers.get('date'));
    const now = Date.now();
    const expiration = CACHE_EXPIRATION[cacheName];

    if (expiration && (now - cacheDate.getTime()) < expiration) {
      return cachedResponse;
    }
  }

  try {
    const networkResponse = await fetch(request);

    if (networkResponse.ok) {
      const cache = await caches.open(cacheName);
      cache.put(request, networkResponse.clone());
      trimCache(cacheName, MAX_CACHE_SIZE[cacheName]);
    }

    return networkResponse;
  } catch (error) {
    // If we have a cached response (even if expired), return it
    if (cachedResponse) {
      return cachedResponse;
    }

    // Return placeholder for images
    if (isImageRequest(new URL(request.url))) {
      return getPlaceholderImage();
    }

    throw error;
  }
}

/**
 * Stale-while-revalidate strategy
 * Return cached response immediately, update cache in background
 */
async function staleWhileRevalidateStrategy(request, cacheName) {
  const cachedResponse = await caches.match(request);

  const networkFetch = fetch(request)
    .then((networkResponse) => {
      if (networkResponse.ok) {
        caches.open(cacheName).then((cache) => {
          cache.put(request, networkResponse.clone());
          trimCache(cacheName, MAX_CACHE_SIZE[cacheName]);
        });
      }
      return networkResponse;
    })
    .catch(() => {
      // Network failed, but we might have cache
      return cachedResponse;
    });

  // Return cached response immediately if available
  return cachedResponse || networkFetch;
}

/**
 * Background sync event
 * Handle queued operations when back online
 */
self.addEventListener('sync', (event) => {
  console.log('[SW] Background sync triggered:', event.tag);

  if (event.tag === 'sync-searches') {
    event.waitUntil(syncPendingSearches());
  } else if (event.tag === 'sync-tags') {
    event.waitUntil(syncPendingTags());
  }
});

/**
 * Push notification event (stub for future implementation)
 */
self.addEventListener('push', (event) => {
  console.log('[SW] Push notification received');

  const data = event.data ? event.data.json() : {};
  const title = data.title || 'Hippo';
  const options = {
    body: data.body || 'New update available',
    icon: '/icons/icon-192.png',
    badge: '/icons/badge-72.png',
    vibrate: [200, 100, 200],
    data: data.url || '/',
    actions: [
      { action: 'open', title: 'Open' },
      { action: 'close', title: 'Close' }
    ]
  };

  event.waitUntil(
    self.registration.showNotification(title, options)
  );
});

/**
 * Notification click event
 */
self.addEventListener('notificationclick', (event) => {
  console.log('[SW] Notification clicked:', event.action);

  event.notification.close();

  if (event.action === 'open' || !event.action) {
    const url = event.notification.data || '/';
    event.waitUntil(
      clients.openWindow(url)
    );
  }
});

/**
 * Message event - handle messages from the app
 */
self.addEventListener('message', (event) => {
  console.log('[SW] Message received:', event.data);

  if (event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  } else if (event.data.type === 'CLEAR_CACHE') {
    event.waitUntil(clearAllCaches());
  } else if (event.data.type === 'CACHE_URLS') {
    event.waitUntil(cacheUrls(event.data.urls));
  }
});

/**
 * Helper: Check if request is an API call
 */
function isApiRequest(url) {
  return url.pathname.startsWith('/api/') ||
         url.pathname.includes('invoke') ||
         url.searchParams.has('__tauriModule');
}

/**
 * Helper: Check if request is for an image
 */
function isImageRequest(url) {
  const imageExtensions = ['.jpg', '.jpeg', '.png', '.gif', '.webp', '.svg', '.ico'];
  const pathname = url.pathname.toLowerCase();
  return imageExtensions.some(ext => pathname.endsWith(ext)) ||
         url.pathname.startsWith('/thumbnails/');
}

/**
 * Helper: Check if request is for a static asset
 */
function isStaticAsset(url) {
  const staticExtensions = ['.css', '.js', '.woff', '.woff2', '.ttf', '.eot'];
  const pathname = url.pathname.toLowerCase();
  return staticExtensions.some(ext => pathname.endsWith(ext));
}

/**
 * Helper: Trim cache to maximum size
 */
async function trimCache(cacheName, maxItems) {
  if (!maxItems) return;

  const cache = await caches.open(cacheName);
  const keys = await cache.keys();

  if (keys.length > maxItems) {
    // Delete oldest items (FIFO)
    const deleteCount = keys.length - maxItems;
    for (let i = 0; i < deleteCount; i++) {
      await cache.delete(keys[i]);
    }
    console.log(`[SW] Trimmed ${deleteCount} items from ${cacheName}`);
  }
}

/**
 * Helper: Clear all caches
 */
async function clearAllCaches() {
  const cacheNames = await caches.keys();
  return Promise.all(
    cacheNames.map(cacheName => caches.delete(cacheName))
  );
}

/**
 * Helper: Cache specific URLs
 */
async function cacheUrls(urls) {
  const cache = await caches.open(DYNAMIC_CACHE);
  return Promise.all(
    urls.map(url => {
      return fetch(url).then(response => {
        if (response.ok) {
          return cache.put(url, response);
        }
      }).catch(err => {
        console.warn('[SW] Failed to cache URL:', url, err);
      });
    })
  );
}

/**
 * Helper: Get placeholder image for offline mode
 */
function getPlaceholderImage() {
  // SVG placeholder
  const svg = `
    <svg width="256" height="256" xmlns="http://www.w3.org/2000/svg">
      <rect width="256" height="256" fill="#e5e7eb"/>
      <text x="50%" y="50%" text-anchor="middle" dy=".3em" fill="#9ca3af" font-family="Arial" font-size="16">
        Offline
      </text>
    </svg>
  `;

  return new Response(svg, {
    headers: {
      'Content-Type': 'image/svg+xml',
      'Cache-Control': 'no-cache'
    }
  });
}

/**
 * Helper: Sync pending searches when back online
 */
async function syncPendingSearches() {
  try {
    // Get pending searches from IndexedDB or localStorage
    const pendingSearches = await getPendingSearches();

    for (const search of pendingSearches) {
      try {
        await fetch('/api/search', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(search)
        });
      } catch (error) {
        console.error('[SW] Failed to sync search:', error);
      }
    }

    await clearPendingSearches();
    console.log('[SW] Synced pending searches');
  } catch (error) {
    console.error('[SW] Background sync failed:', error);
  }
}

/**
 * Helper: Sync pending tag operations
 */
async function syncPendingTags() {
  try {
    const pendingTags = await getPendingTags();

    for (const tagOp of pendingTags) {
      try {
        await fetch('/api/tags', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(tagOp)
        });
      } catch (error) {
        console.error('[SW] Failed to sync tag:', error);
      }
    }

    await clearPendingTags();
    console.log('[SW] Synced pending tags');
  } catch (error) {
    console.error('[SW] Tag sync failed:', error);
  }
}

/**
 * Helper: Get pending searches (stub - implement with IndexedDB)
 */
async function getPendingSearches() {
  // TODO: Implement with IndexedDB
  return [];
}

/**
 * Helper: Clear pending searches (stub)
 */
async function clearPendingSearches() {
  // TODO: Implement with IndexedDB
}

/**
 * Helper: Get pending tags (stub)
 */
async function getPendingTags() {
  // TODO: Implement with IndexedDB
  return [];
}

/**
 * Helper: Clear pending tags (stub)
 */
async function clearPendingTags() {
  // TODO: Implement with IndexedDB
}

console.log('[SW] Service worker loaded');
