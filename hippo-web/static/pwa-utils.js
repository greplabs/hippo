/**
 * Hippo PWA Utilities
 * Reusable functions for Progressive Web App features
 */

// ============================================================================
// Service Worker Management
// ============================================================================

/**
 * Register service worker and handle updates
 * @param {string} swPath - Path to service worker file (default: '/sw.js')
 * @param {Function} onUpdate - Callback when update is available
 * @returns {Promise<ServiceWorkerRegistration|null>}
 */
export async function registerServiceWorker(swPath = '/sw.js', onUpdate = null) {
  if (!('serviceWorker' in navigator)) {
    console.warn('Service workers not supported in this browser');
    return null;
  }

  try {
    const registration = await navigator.serviceWorker.register(swPath);
    console.log('[PWA] Service worker registered:', registration.scope);

    // Handle updates
    registration.addEventListener('updatefound', () => {
      const newWorker = registration.installing;

      newWorker.addEventListener('statechange', () => {
        if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
          console.log('[PWA] New service worker available');
          if (onUpdate) {
            onUpdate(registration);
          }
        }
      });
    });

    // Check for updates periodically (every hour)
    setInterval(() => {
      registration.update();
    }, 60 * 60 * 1000);

    return registration;
  } catch (error) {
    console.error('[PWA] Service worker registration failed:', error);
    return null;
  }
}

/**
 * Update service worker and reload
 * @param {ServiceWorkerRegistration} registration
 */
export function updateServiceWorker(registration) {
  if (registration.waiting) {
    registration.waiting.postMessage({ type: 'SKIP_WAITING' });

    // Reload when new worker takes control
    let refreshing = false;
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (!refreshing) {
        refreshing = true;
        window.location.reload();
      }
    });
  }
}

/**
 * Clear all service worker caches
 * @returns {Promise<void>}
 */
export async function clearAllCaches() {
  const cacheNames = await caches.keys();
  await Promise.all(cacheNames.map(name => caches.delete(name)));
  console.log('[PWA] All caches cleared');
}

/**
 * Send message to service worker
 * @param {Object} message - Message to send
 * @returns {Promise<any>}
 */
export function sendMessageToSW(message) {
  return new Promise((resolve, reject) => {
    const messageChannel = new MessageChannel();

    messageChannel.port1.onmessage = (event) => {
      if (event.data.error) {
        reject(event.data.error);
      } else {
        resolve(event.data);
      }
    };

    if (navigator.serviceWorker.controller) {
      navigator.serviceWorker.controller.postMessage(message, [messageChannel.port2]);
    } else {
      reject(new Error('No service worker controller'));
    }
  });
}

// ============================================================================
// Install Prompt Management
// ============================================================================

/**
 * Handle PWA install prompt
 * @returns {Object} - Object with methods to control install prompt
 */
export function setupInstallPrompt() {
  let deferredPrompt = null;
  const listeners = {
    show: [],
    install: [],
    dismiss: []
  };

  // Capture beforeinstallprompt event
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    deferredPrompt = e;

    // Check if user dismissed recently
    const dismissed = localStorage.getItem('pwa-install-dismissed');
    const dismissedTime = localStorage.getItem('pwa-install-dismissed-time');

    if (!dismissed || (dismissedTime && Date.now() - parseInt(dismissedTime) > 7 * 24 * 60 * 60 * 1000)) {
      // Show after delay or trigger callbacks
      listeners.show.forEach(cb => cb());
    }
  });

  // Track installation
  window.addEventListener('appinstalled', () => {
    console.log('[PWA] App installed');
    deferredPrompt = null;
    listeners.install.forEach(cb => cb());
  });

  return {
    // Show install prompt
    async show() {
      if (!deferredPrompt) {
        console.warn('[PWA] Install prompt not available');
        return null;
      }

      deferredPrompt.prompt();
      const { outcome } = await deferredPrompt.userChoice;
      console.log('[PWA] Install prompt result:', outcome);

      deferredPrompt = null;
      return outcome;
    },

    // Dismiss install prompt
    dismiss(duration = 7 * 24 * 60 * 60 * 1000) {
      localStorage.setItem('pwa-install-dismissed', 'true');
      localStorage.setItem('pwa-install-dismissed-time', Date.now().toString());

      // Auto-clear after duration
      setTimeout(() => {
        localStorage.removeItem('pwa-install-dismissed');
        localStorage.removeItem('pwa-install-dismissed-time');
      }, duration);

      listeners.dismiss.forEach(cb => cb());
    },

    // Check if prompt is available
    isAvailable() {
      return deferredPrompt !== null;
    },

    // Add event listeners
    on(event, callback) {
      if (listeners[event]) {
        listeners[event].push(callback);
      }
    },

    // Remove event listener
    off(event, callback) {
      if (listeners[event]) {
        listeners[event] = listeners[event].filter(cb => cb !== callback);
      }
    }
  };
}

// ============================================================================
// Offline Detection
// ============================================================================

/**
 * Setup online/offline detection
 * @param {Function} onOnline - Callback when coming online
 * @param {Function} onOffline - Callback when going offline
 * @returns {Object} - Object with current status and cleanup function
 */
export function setupOfflineDetection(onOnline, onOffline) {
  function handleOnline() {
    console.log('[PWA] Connection restored');
    if (onOnline) onOnline();
  }

  function handleOffline() {
    console.log('[PWA] Connection lost');
    if (onOffline) onOffline();
  }

  window.addEventListener('online', handleOnline);
  window.addEventListener('offline', handleOffline);

  // Check initial state
  const isOnline = navigator.onLine;
  console.log('[PWA] Initial connection state:', isOnline ? 'online' : 'offline');

  return {
    isOnline: () => navigator.onLine,
    cleanup: () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    }
  };
}

/**
 * Create offline banner UI element
 * @returns {HTMLElement}
 */
export function createOfflineBanner() {
  const banner = document.createElement('div');
  banner.id = 'offline-banner';
  banner.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    background: #ef4444;
    color: white;
    padding: 12px 20px;
    text-align: center;
    font-size: 14px;
    font-weight: 500;
    z-index: 9999;
    transform: translateY(-100%);
    transition: transform 0.3s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  `;

  const text = document.createElement('span');
  text.textContent = "You're offline. Some features may be limited.";
  banner.appendChild(text);

  setupOfflineDetection(
    () => {
      banner.style.background = '#10b981';
      text.textContent = "You're back online!";
      banner.style.transform = 'translateY(0)';

      setTimeout(() => {
        banner.style.transform = 'translateY(-100%)';
      }, 3000);
    },
    () => {
      banner.style.background = '#ef4444';
      text.textContent = "You're offline. Some features may be limited.";
      banner.style.transform = 'translateY(0)';
    }
  );

  // Show if initially offline
  if (!navigator.onLine) {
    setTimeout(() => {
      banner.style.transform = 'translateY(0)';
    }, 100);
  }

  return banner;
}

// ============================================================================
// Background Sync
// ============================================================================

/**
 * Queue operation for background sync
 * @param {string} tag - Sync tag name
 * @param {Object} data - Data to sync
 * @returns {Promise<void>}
 */
export async function queueBackgroundSync(tag, data) {
  // Store data in IndexedDB for service worker to process
  await saveToIndexedDB('sync-queue', { tag, data, timestamp: Date.now() });

  // Register sync event
  if ('sync' in navigator.serviceWorker) {
    const registration = await navigator.serviceWorker.ready;
    await registration.sync.register(tag);
    console.log('[PWA] Background sync registered:', tag);
  } else {
    console.warn('[PWA] Background sync not supported');
  }
}

/**
 * Get all queued sync operations
 * @returns {Promise<Array>}
 */
export async function getSyncQueue() {
  return await getAllFromIndexedDB('sync-queue');
}

/**
 * Clear sync queue
 * @returns {Promise<void>}
 */
export async function clearSyncQueue() {
  return await clearIndexedDBStore('sync-queue');
}

// ============================================================================
// IndexedDB Utilities
// ============================================================================

/**
 * Open IndexedDB database
 * @param {string} dbName - Database name
 * @param {number} version - Database version
 * @returns {Promise<IDBDatabase>}
 */
function openDB(dbName = 'HippoPWA', version = 1) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(dbName, version);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = (event) => {
      const db = event.target.result;

      // Create object stores
      if (!db.objectStoreNames.contains('sync-queue')) {
        db.createObjectStore('sync-queue', { keyPath: 'id', autoIncrement: true });
      }

      if (!db.objectStoreNames.contains('offline-cache')) {
        db.createObjectStore('offline-cache', { keyPath: 'key' });
      }
    };
  });
}

/**
 * Save data to IndexedDB
 * @param {string} storeName - Object store name
 * @param {Object} data - Data to save
 * @returns {Promise<void>}
 */
async function saveToIndexedDB(storeName, data) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([storeName], 'readwrite');
    const store = transaction.objectStore(storeName);
    const request = store.add(data);

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}

/**
 * Get all items from IndexedDB store
 * @param {string} storeName - Object store name
 * @returns {Promise<Array>}
 */
async function getAllFromIndexedDB(storeName) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([storeName], 'readonly');
    const store = transaction.objectStore(storeName);
    const request = store.getAll();

    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/**
 * Clear IndexedDB store
 * @param {string} storeName - Object store name
 * @returns {Promise<void>}
 */
async function clearIndexedDBStore(storeName) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([storeName], 'readwrite');
    const store = transaction.objectStore(storeName);
    const request = store.clear();

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}

// ============================================================================
// Push Notifications
// ============================================================================

/**
 * Request notification permission
 * @returns {Promise<NotificationPermission>}
 */
export async function requestNotificationPermission() {
  if (!('Notification' in window)) {
    console.warn('[PWA] Notifications not supported');
    return 'denied';
  }

  const permission = await Notification.requestPermission();
  console.log('[PWA] Notification permission:', permission);
  return permission;
}

/**
 * Show local notification
 * @param {string} title - Notification title
 * @param {Object} options - Notification options
 * @returns {Promise<void>}
 */
export async function showNotification(title, options = {}) {
  if (Notification.permission !== 'granted') {
    console.warn('[PWA] Notification permission not granted');
    return;
  }

  const registration = await navigator.serviceWorker.ready;
  await registration.showNotification(title, {
    icon: '/icons/icon-192.png',
    badge: '/icons/badge-72.png',
    vibrate: [200, 100, 200],
    ...options
  });
}

// ============================================================================
// Share API
// ============================================================================

/**
 * Check if Web Share API is supported
 * @returns {boolean}
 */
export function canShare() {
  return 'share' in navigator;
}

/**
 * Share content using Web Share API
 * @param {Object} data - Data to share (title, text, url, files)
 * @returns {Promise<void>}
 */
export async function shareContent(data) {
  if (!canShare()) {
    console.warn('[PWA] Web Share API not supported');
    throw new Error('Web Share API not supported');
  }

  try {
    await navigator.share(data);
    console.log('[PWA] Content shared successfully');
  } catch (error) {
    if (error.name === 'AbortError') {
      console.log('[PWA] Share cancelled by user');
    } else {
      console.error('[PWA] Share failed:', error);
      throw error;
    }
  }
}

// ============================================================================
// Performance Monitoring
// ============================================================================

/**
 * Get page load performance metrics
 * @returns {Object}
 */
export function getPerformanceMetrics() {
  if (!('performance' in window)) {
    return null;
  }

  const perfData = performance.getEntriesByType('navigation')[0];
  if (!perfData) return null;

  return {
    // Time to First Byte
    ttfb: perfData.responseStart - perfData.requestStart,

    // DOM Content Loaded
    domContentLoaded: perfData.domContentLoadedEventEnd - perfData.domContentLoadedEventStart,

    // Total Load Time
    loadTime: perfData.loadEventEnd - perfData.fetchStart,

    // DNS Lookup
    dnsTime: perfData.domainLookupEnd - perfData.domainLookupStart,

    // TCP Connection
    tcpTime: perfData.connectEnd - perfData.connectStart,

    // Request/Response
    requestTime: perfData.responseEnd - perfData.requestStart,

    // DOM Processing
    domProcessing: perfData.domComplete - perfData.domLoading,

    // Page Rendering
    renderTime: perfData.loadEventEnd - perfData.domComplete
  };
}

/**
 * Log performance metrics to console
 */
export function logPerformanceMetrics() {
  const metrics = getPerformanceMetrics();
  if (!metrics) return;

  console.group('[PWA] Performance Metrics');
  console.log('TTFB:', metrics.ttfb.toFixed(2), 'ms');
  console.log('DOM Content Loaded:', metrics.domContentLoaded.toFixed(2), 'ms');
  console.log('Total Load Time:', metrics.loadTime.toFixed(2), 'ms');
  console.log('DNS Lookup:', metrics.dnsTime.toFixed(2), 'ms');
  console.log('TCP Connection:', metrics.tcpTime.toFixed(2), 'ms');
  console.log('Request Time:', metrics.requestTime.toFixed(2), 'ms');
  console.log('DOM Processing:', metrics.domProcessing.toFixed(2), 'ms');
  console.log('Render Time:', metrics.renderTime.toFixed(2), 'ms');
  console.groupEnd();
}

// ============================================================================
// Network Information
// ============================================================================

/**
 * Get network information
 * @returns {Object|null}
 */
export function getNetworkInfo() {
  if (!('connection' in navigator)) {
    return null;
  }

  const conn = navigator.connection;
  return {
    effectiveType: conn.effectiveType, // '4g', '3g', etc.
    downlink: conn.downlink, // Mbps
    rtt: conn.rtt, // Round-trip time in ms
    saveData: conn.saveData // Data saver enabled
  };
}

/**
 * Check if on slow network
 * @returns {boolean}
 */
export function isSlowNetwork() {
  const info = getNetworkInfo();
  if (!info) return false;

  return info.effectiveType === 'slow-2g' ||
         info.effectiveType === '2g' ||
         info.saveData ||
         info.rtt > 300;
}

// ============================================================================
// Utilities
// ============================================================================

/**
 * Check if running as installed PWA
 * @returns {boolean}
 */
export function isInstalledPWA() {
  return window.matchMedia('(display-mode: standalone)').matches ||
         window.navigator.standalone === true; // iOS
}

/**
 * Check if running in Tauri
 * @returns {boolean}
 */
export function isTauriApp() {
  return window.__TAURI__ !== undefined;
}

/**
 * Detect platform
 * @returns {string} - 'ios', 'android', 'desktop'
 */
export function detectPlatform() {
  const ua = navigator.userAgent.toLowerCase();

  if (/iphone|ipad|ipod/.test(ua)) {
    return 'ios';
  } else if (/android/.test(ua)) {
    return 'android';
  } else {
    return 'desktop';
  }
}

// ============================================================================
// Initialize PWA
// ============================================================================

/**
 * Initialize all PWA features
 * @param {Object} options - Configuration options
 * @returns {Promise<Object>}
 */
export async function initializePWA(options = {}) {
  const {
    swPath = '/sw.js',
    showInstallPrompt = true,
    showOfflineBanner = true,
    enableNotifications = false,
    onUpdate = null,
    onOnline = null,
    onOffline = null
  } = options;

  const result = {
    registration: null,
    installPrompt: null,
    offlineDetection: null,
    isInstalled: isInstalledPWA(),
    isTauri: isTauriApp(),
    platform: detectPlatform()
  };

  // Register service worker
  if (!isTauriApp()) {
    result.registration = await registerServiceWorker(swPath, onUpdate);
  }

  // Setup install prompt
  if (showInstallPrompt) {
    result.installPrompt = setupInstallPrompt();
  }

  // Setup offline detection
  if (showOfflineBanner) {
    result.offlineDetection = setupOfflineDetection(onOnline, onOffline);

    // Add offline banner to page
    const banner = createOfflineBanner();
    document.body.insertBefore(banner, document.body.firstChild);
  }

  // Request notification permission
  if (enableNotifications) {
    await requestNotificationPermission();
  }

  // Log performance metrics
  if (window.performance) {
    window.addEventListener('load', () => {
      logPerformanceMetrics();
    });
  }

  console.log('[PWA] Initialized:', result);
  return result;
}

// Export default initialization function
export default initializePWA;
