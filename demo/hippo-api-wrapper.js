// Hippo Web API Wrapper
// This script provides a compatibility layer between Tauri's invoke() and REST API
// Allows the same UI to work in both desktop (Tauri) and web (static demo) modes

(function() {
  'use strict';

  // Detect if we're running in Tauri or web mode
  const isTauri = window.__TAURI__ && window.__TAURI__.core;
  const API_BASE = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1'
    ? 'http://localhost:3000/api'
    : '/api';

  console.log('[Hippo] Mode:', isTauri ? 'Tauri Desktop' : 'Web Demo');
  console.log('[Hippo] API Base:', API_BASE);

  // Create mock invoke function for web mode
  window.invoke = isTauri
    ? window.__TAURI__.core.invoke
    : createWebInvoke();

  window.convertFileSrc = isTauri
    ? window.__TAURI__.core.convertFileSrc
    : (path) => path; // In web mode, just return the path

  // Web-based invoke using fetch to JSON files or REST API
  function createWebInvoke() {
    return async function invoke(command, args = {}) {
      console.log('[Hippo Web] Invoke:', command, args);

      try {
        // Map Tauri commands to API endpoints
        const endpoint = mapCommandToEndpoint(command, args);
        const response = await fetch(endpoint);

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        console.log('[Hippo Web] Response:', command, data);
        return data;
      } catch (error) {
        console.error('[Hippo Web] Error:', command, error);

        // Return sensible defaults for demo mode
        return getDefaultResponse(command);
      }
    };
  }

  // Map Tauri command names to REST API endpoints
  function mapCommandToEndpoint(command, args) {
    const commandMap = {
      'initialize': `${API_BASE}/health.json`,
      'get_stats': `${API_BASE}/stats.json`,
      'get_sources': `${API_BASE}/sources.json`,
      'get_tags': `${API_BASE}/tags.json`,
      'search': buildSearchEndpoint(args),
      'list_collections': `${API_BASE}/collections.json`,
      'get_qdrant_stats': `${API_BASE}/qdrant-stats.json`,
      'ollama_status': `${API_BASE}/ollama-status.json`,
      'get_virtual_paths': `${API_BASE}/virtual-paths.json`
    };

    return commandMap[command] || `${API_BASE}/search.json`;
  }

  // Build search endpoint with query parameters
  function buildSearchEndpoint(args) {
    const params = new URLSearchParams();

    if (args.query) {
      params.append('q', args.query);
    }

    if (args.tags && args.tags.length > 0) {
      params.append('tags', args.tags.join(','));
    }

    const queryString = params.toString();
    return `${API_BASE}/search.json${queryString ? '?' + queryString : ''}`;
  }

  // Default responses for commands when API is unavailable
  function getDefaultResponse(command) {
    const defaults = {
      'initialize': { status: 'ok', mode: 'demo-offline' },
      'get_stats': {
        total_memories: 247,
        by_kind: { Image: 128, Video: 23, Code: 45, Document: 28 },
        by_source: { Local: 247 },
        total_size_bytes: 5432198765
      },
      'get_sources': [
        { source: { Local: { root_path: '/Users/demo/Documents' } }, enabled: true }
      ],
      'get_tags': [
        { name: 'work', count: 87 },
        { name: 'personal', count: 56 },
        { name: 'vacation', count: 34 }
      ],
      'search': {
        memories: [],
        total_count: 0,
        suggested_tags: [],
        clusters: []
      },
      'list_collections': [],
      'get_qdrant_stats': { available: false },
      'ollama_status': { available: false },
      'get_virtual_paths': []
    };

    return defaults[command] || {};
  }

  // Add demo mode banner
  if (!isTauri) {
    const banner = document.createElement('div');
    banner.id = 'demo-banner';
    banner.innerHTML = `
      <style>
        #demo-banner {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          color: white;
          padding: 12px 20px;
          text-align: center;
          font-size: 14px;
          font-weight: 500;
          z-index: 10000;
          box-shadow: 0 2px 8px rgba(0,0,0,0.15);
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 12px;
        }
        #demo-banner svg {
          width: 20px;
          height: 20px;
        }
        #demo-banner a {
          color: white;
          text-decoration: underline;
          font-weight: 600;
        }
        body { margin-top: 44px !important; }
      </style>
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>
        This is a demo version with sample data.
        <a href="https://github.com/greplabs/hippo" target="_blank">Download the desktop app</a> for full functionality.
      </span>
    `;

    // Insert banner when DOM is ready
    if (document.body) {
      document.body.insertBefore(banner, document.body.firstChild);
    } else {
      document.addEventListener('DOMContentLoaded', () => {
        document.body.insertBefore(banner, document.body.firstChild);
      });
    }
  }

  // Export for debugging
  window.HippoAPI = {
    isTauri,
    API_BASE,
    invoke: window.invoke
  };
})();
